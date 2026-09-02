import { writable, derived } from "svelte/store";
import type { Note, NoteId } from "./protocol";

/** The collection of all notes, keyed by id. Single source of truth,
 * populated by sync and later mutated by websocket events. */
export const notes = writable<Record<NoteId, Note>>({});

/** Which note is open, or null for the list view. Poor man's SPA routing. */
export const openNoteId = writable<NoteId | null>(null);

/** Notes as a list, most recently changed first. */
export const sortedNotes = derived(notes, ($notes) =>
  Object.values($notes).sort(
    (a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at),
  ),
);

/** The currently open note, or null. Resolves live from the collection,
 * so websocket updates to the open note will flow into the editor. */
export const openNote = derived([notes, openNoteId], ([$notes, $openNoteId]) =>
  $openNoteId === null ? null : ($notes[$openNoteId] ?? null),
);

/** Replace the whole collection (sync response). */
export function setAllNotes(list: Note[]) {
  const map: Record<NoteId, Note> = {};
  for (const note of list) {
    map[note.id] = note;
  }
  notes.set(map);
}

/** Apply a note creation from the server. */
export function applyNoteCreated(note: Note) {
  notes.update(($notes) => ({ ...$notes, [note.id]: note }));
}

/** Apply a rename from the server. */
export function applyNoteRenamed(id: NoteId, version: number, title: string) {
  notes.update(($notes) => {
    const note = $notes[id];
    if (note === undefined) return $notes;
    return {
      ...$notes,
      [id]: { ...note, title, version, updated_at: new Date().toISOString() },
    };
  });
}

/** Apply a note deletion from the server. */
export function applyNoteDeleted(id: NoteId) {
  notes.update(($notes) => {
    const { [id]: _, ...rest } = $notes;
    return rest;
  });
}

/** Replace one note wholesale (resync response). */
export function applyNoteResynced(note: Note) {
  notes.update(($notes) => ({ ...$notes, [note.id]: note }));
}

/** Apply an edit event to the store copy of a note (list view bookkeeping
 * + keeping non-open notes' content current). */
export function applyNoteEdited(
  id: NoteId,
  version: number,
  content: string | null,
) {
  notes.update(($notes) => {
    const note = $notes[id];
    if (note === undefined) return $notes;
    return {
      ...$notes,
      [id]: {
        ...note,
        version,
        content: content ?? note.content,
        updated_at: new Date().toISOString(),
      },
    };
  });
}
