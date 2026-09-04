import type { EditNoteMessage, NoteEdit, ServerEvent } from "./protocol";
import { syncRequest } from "./api";
import { setAllNotes } from "./stores";

/** Delay before reconnecting, doubling per attempt up to a cap. */
const RECONNECT_MIN_MS = 500;
const RECONNECT_MAX_MS = 10_000;

let socket: WebSocket | null = null;
let reconnectDelay = RECONNECT_MIN_MS;

/* Session identity: regenerated per connection. Everything sent on this
 * socket is attributed to this id; the server excludes same-id ops from
 * transformation, which is what makes bursting safe. */
let clientId: string = crypto.randomUUID();
export function getClientId(): string {
  return clientId;
}

type EventHandler = (event: ServerEvent) => void;
const handlers = new Set<EventHandler>();

/** Subscribe to server events. Returns an unsubscribe function. */
export function onServerEvent(handler: EventHandler): () => void {
  handlers.add(handler);
  return () => handlers.delete(handler);
}

type ReconnectHandler = () => void;
const reconnectHandlers = new Set<ReconnectHandler>();

export function onReconnect(handler: ReconnectHandler): () => void {
  reconnectHandlers.add(handler);
  return () => reconnectHandlers.delete(handler);
}

/** Open the websocket and keep it alive for the lifetime of the app.
 * Call once at startup. */
export function startWs() {
  connect();
}

function connect() {
  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  socket = new WebSocket(`${proto}//${location.host}/ws`);

  socket.onopen = async () => {
    clientId = crypto.randomUUID();
    reconnectDelay = RECONNECT_MIN_MS;
    const response = await syncRequest();
    setAllNotes(response.notes);
    for (const handler of reconnectHandlers) handler();
  };

  socket.onmessage = (message: MessageEvent) => {
    const event: ServerEvent = JSON.parse(message.data);
    for (const handler of handlers) handler(event);
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

/* Sends a note edit message */
export function SendNoteEdit(
  note_id: string,
  base_version: number,
  op: NoteEdit,
): boolean {
  if (!socket) return false;

  const message: EditNoteMessage = {
    note_id,
    client_id: clientId,
    base_version,
    op,
  };
  socket.send(JSON.stringify(message));

  return true;
}
