/** Mirrors of the rust `api` crate. Keep in sync by hand. */

export type NoteId = string; // uuid

export interface Note {
  id: NoteId;
  title: string;
  content: string;
  version: number;
  updated_at: string; // chrono DateTime<Utc>, ISO 8601
}

/** api::NoteEdit — internally tagged, snake_case. */
export type NoteEdit =
  | { op: "insert"; line: number; col: number; text: string }
  | { op: "delete"; line: number; col: number; len: number }
  | { op: "replace_all"; content: string };

/* ── client requests ─────────────────────────────── */

export interface CreateNoteRequest {
  title: string;
}

export interface EditNoteRequest {
  note_id: NoteId;
  base_version: number;
  op: NoteEdit;
}

export interface RenameNoteRequest {
  note_id: NoteId;
  base_version: number;
  title: string;
}

export interface DeleteNoteRequest {
  note_id: NoteId;
}

export interface ResyncNoteRequest {
  note_id: NoteId;
}

/* ── server responses ────────────────────────────── */

export interface SyncResponse {
  notes: Note[];
}

export type CreateNoteResponse =
  { result: "Ok"; note_id: NoteId } | { result: "Error"; message: string };

export type EditNoteResponse =
  | { result: "Ok" }
  | { result: "Rejected"; note_id: NoteId; current_version: number }
  | { result: "Error"; message: string };

export type RenameNoteResponse =
  { result: "Ok" } | { result: "Error"; message: string };

export type DeleteNoteResponse =
  { result: "Ok" } | { result: "Error"; message: string };

export type ResyncNoteResponse =
  { result: "Ok"; note: Note } | { result: "Error"; message: string };

/* ── server events (websocket) ───────────────────── */

export type ServerEvent =
  | { NoteCreated: { note: Note } }
  | { NoteEdited: { note_id: NoteId; version: number; op: NoteEdit } }
  | { NoteRenamed: { note_id: NoteId; version: number; title: string } }
  | { NoteDeleted: { note_id: NoteId } };
