import { writable, derived } from "svelte/store";
import type {
  Note,
  NoteCreatedEvent,
  NoteDeletedEvent,
  NoteEditedEvent,
  NoteId,
  NoteRenamedEvent,
  ServerEvent,
} from "./protocol";
import { onServerEvent } from "./ws";
import { applyNoteEditOp } from "./editor";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/** The collection of all notes, keyed by id. Single source of truth,
 * populated by sync and later mutated by websocket events. */
export const notes = writable<Map<NoteId, Note>>(new Map());

/** Which note is open, or null for the list view. Poor man's SPA routing. */
export const openNoteId = writable<NoteId | null>(null);

/** Notes as a list, most recently changed first. */
export const sortedNotes = derived(notes, (notes) =>
  Array.from(notes.values()).sort(
    (a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at),
  ),
);

/** The currently open note, or null. Resolves live from the collection,
 * so websocket updates to the open note will flow into the editor. */
export const openNote = derived([notes, openNoteId], ([notes, openNoteId]) =>
  openNoteId === null ? null : (notes.get(openNoteId) ?? null),
);

/** Replace the whole collection (sync response). */
export function setAllNotes(list: Note[]) {
  const map: Map<NoteId, Note> = new Map();
  for (const note of list) {
    map.set(note.id, note);
  }
  notes.set(map);
}

/** Apply a note creation from the server. */
function applyNoteCreated(event: NoteCreatedEvent) {
  const note = event.note;
  notes.update((notes) => {
    notes.set(note.id, note);
    return notes;
  });
}

/** Apply a rename from the server. */
function applyNoteRenamed(event: NoteRenamedEvent) {
  const id = event.note_id;
  const title = event.title;
  const version = event.version;
  notes.update((notes) => {
    const note = notes.get(id);
    if (note) {
      note.title = title;
      note.version = version;
      note.updated_at = new Date().toISOString();
    }
    return notes;
  });
}

/** Apply a note deletion from the server. */
function applyNoteDeleted(event: NoteDeletedEvent) {
  const id = event.note_id;
  notes.update((notes) => {
    notes.delete(id);
    return notes;
  });
}

/** Apply a note edit event from the server. */
function applyNoteEdit(event: NoteEditedEvent) {
  notes.update((notes) => {
    const note = notes.get(event.note_id);
    if (note) {
      const prev_content = encoder.encode(note.content);
      const new_content = applyNoteEditOp(prev_content, event.op, encoder);
      note.content = decoder.decode(new_content);
      note.version = event.version;
      note.updated_at = new Date().toISOString();
    }
    return notes;
  });
}

/** Handle any event from the server */
function handleEvent(event: ServerEvent) {
  switch (event.kind) {
    case "note_created":
      applyNoteCreated(event.event);
      break;
    case "note_deleted":
      applyNoteDeleted(event.event);
      break;
    case "note_renamed":
      applyNoteRenamed(event.event);
      break;
    case "note_edited":
      applyNoteEdit(event.event);
      break;
  }
}

/* Register a server event handler for note creation, deletion, rename */
onServerEvent(handleEvent);
