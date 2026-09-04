/** Mirrors of the rust `api` crate. Keep in sync by hand. */

export type NoteId = string; // uuid

export interface Note {
  id: NoteId;
  title: string;
  content: string;
  version: number;
  updated_at: string; // chrono DateTime<Utc>, ISO 8601
}

/** api::NoteEdit — internally tagged, snake_case.
 *
 * `pos` and `len` are BYTE OFFSETS into the note's UTF-8 encoding —
 * NOT JS string indices (UTF-16 units). Convert before building or
 * applying ops; a raw selectionStart is only valid for pure-ASCII text.
 */
export type NoteEdit =
  | { op: "insert"; pos: number; text: string }
  | { op: "delete"; pos: number; len: number }
  | { op: "replace_all"; content: string };

/* ── websocket: client → server ──────────────────── */

/** Sent over the WS to edit a note. */
export interface EditNoteMessage {
  note_id: NoteId;
  /** Unique client session identifier (generated at connection time). */
  client_id: string;
  /** Last server version this client has integrated. */
  base_version: number;
  op: NoteEdit;
}

/* ── websocket: server → client ──────────────────── */

export type EditNoteResultMessage =
  | { result: "ok" }
  | { result: "please_resync" }
  | { result: "internal_error"; message: string };

/* ── client requests ─────────────────────────────── */

export interface CreateNoteRequest {
  title: string;
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
  { result: "ok" } | { result: "error"; message: string };

export type RenameNoteResponse =
  { result: "ok" } | { result: "error"; message: string };

export type DeleteNoteResponse =
  { result: "ok" } | { result: "error"; message: string };

export type ResyncNoteResponse =
  { result: "ok"; note: Note } | { result: "error"; message: string };

/* ── server events (websocket) ───────────────────── */

export interface NoteCreatedEvent {
  note: Note;
}

export interface NoteEditedEvent {
  note_id: NoteId;
  version: number;
  client_id: string;
  op: NoteEdit;
}

export interface NoteRenamedEvent {
  note_id: NoteId;
  version: number;
  title: string;
}

export interface NoteDeletedEvent {
  note_id: NoteId;
}

/** Mirror of the rust ServerEvent — tagged on "kind", snake_case,
 * payload nested under "event". */
export type ServerEvent =
  | { kind: "note_created"; event: NoteCreatedEvent }
  | { kind: "note_edited"; event: NoteEditedEvent }
  | { kind: "note_renamed"; event: NoteRenamedEvent }
  | { kind: "note_deleted"; event: NoteDeletedEvent };
