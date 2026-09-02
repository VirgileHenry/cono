import { get } from "svelte/store";
import type { ServerEvent } from "./protocol";
import { syncRequest } from "./api";
import { applyEdit, handleRemoteEdit } from "./editor";
import { applyNoteEdited } from "./stores";
import { applyNoteCreated } from "./stores";
import { applyNoteRenamed } from "./stores";
import { applyNoteDeleted } from "./stores";
import { notes } from "./stores";
import { setAllNotes } from "./stores";

/** Delay before reconnecting, doubling per attempt up to a cap. */
const RECONNECT_MIN_MS = 500;
const RECONNECT_MAX_MS = 10_000;

let socket: WebSocket | null = null;
let reconnectDelay = RECONNECT_MIN_MS;

/** Open the websocket and keep it alive for the lifetime of the app.
 * Call once at startup. */
export function startWs() {
  connect();
}

function connect() {
  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  socket = new WebSocket(`${proto}//${location.host}/ws`);

  socket.onopen = async () => {
    reconnectDelay = RECONNECT_MIN_MS;
    /* Resync AFTER the socket is open: any event that lands while the
     * sync request is in flight is buffered by the browser and applied
     * after, so we can miss nothing. (Events older than the sync state
     * may be re-applied; idempotent for create/rename/delete.) */
    try {
      const response = await syncRequest();
      setAllNotes(response.notes);
    } catch (e) {
      console.error("resync after ws connect failed:", e);
    }
  };

  socket.onmessage = (message: MessageEvent) => {
    let event: ServerEvent;
    try {
      event = JSON.parse(message.data);
    } catch (e) {
      console.error("unparseable server event:", message.data);
      return;
    }
    dispatch(event);
  };

  socket.onclose = () => {
    socket = null;
    /* Server restart, network drop, laptop sleep... always come back. */
    console.warn(`ws closed, reconnecting in ${reconnectDelay}ms`);
    setTimeout(connect, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 2, RECONNECT_MAX_MS);
  };

  socket.onerror = () => {
    /* onclose fires right after; nothing to do here. */
  };
}

/** Apply a server event to the store. */
function dispatch(event: ServerEvent) {
  if ("NoteCreated" in event) {
    applyNoteCreated(event.NoteCreated.note);
  } else if ("NoteEdited" in event) {
    const { note_id, version, op } = event.NoteEdited;
    /* Keep the store copy current (list sorting + non-open notes). */
    const current = get(notes)[note_id];
    if (current !== undefined) {
      const newContent =
        version === current.version + 1 ? applyEdit(current.content, op) : null;
      applyNoteEdited(note_id, version, newContent);
    }
    /* Let the editor do prediction/echo/conflict logic if this note is open. */
    handleRemoteEdit(note_id, version, op);
  } else if ("NoteRenamed" in event) {
    const { note_id, version, title } = event.NoteRenamed;
    applyNoteRenamed(note_id, version, title);
  } else if ("NoteDeleted" in event) {
    applyNoteDeleted(event.NoteDeleted.note_id);
  } else {
    console.error("unknown server event:", event);
  }
}
