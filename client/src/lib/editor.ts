import { get, writable } from "svelte/store";
import type { Note, NoteEdit, NoteId } from "./protocol";
import { editNoteRequest, resyncNoteRequest } from "./api";
import { applyNoteResynced } from "./stores";

/* ── position helpers ─────────────────────────────────────────
 * Protocol positions are (line, col) with col in CODE POINTS
 * (matches rust `chars()`). JS indices are UTF-16 units, so we
 * convert at the boundary. */

/** UTF-16 index -> protocol (line, col). */
function indexToLineCol(
  text: string,
  index: number,
): { line: number; col: number } {
  let line = 0;
  let lineStart = 0;
  for (let i = 0; i < index; i++) {
    if (text[i] === "\n") {
      line += 1;
      lineStart = i + 1;
    }
  }
  /* col in code points between line start and index. */
  const col = [...text.slice(lineStart, index)].length;
  return { line, col };
}

/** Protocol (line, col) -> UTF-16 index. Returns null if out of bounds. */
function lineColToIndex(
  text: string,
  line: number,
  col: number,
): number | null {
  const lines = text.split("\n");
  if (line >= lines.length) return null;
  let index = 0;
  for (let i = 0; i < line; i++) {
    index += lines[i].length + 1; /* +1 for '\n' */
  }
  const points = [...lines[line]];
  if (col > points.length) return null;
  for (let i = 0; i < col; i++) {
    index += points[i].length; /* surrogate pairs have .length 2 */
  }
  return index;
}

// applyEdit:
export function applyEdit(content: string, op: NoteEdit): string | null {
  if (op.op === "insert") {
    const index = lineColToIndex(content, op.line, op.col);
    if (index === null) return null;
    return content.slice(0, index) + op.text + content.slice(index);
  } else if (op.op === "delete") {
    const start = lineColToIndex(content, op.line, op.col);
    if (start === null) return null;
    const points = [...content.slice(start)];
    if (op.len > points.length) return null;
    let end = start;
    for (let i = 0; i < op.len; i++) {
      end += points[i].length;
    }
    return content.slice(0, start) + content.slice(end);
  } else {
    return op.content;
  }
}

/* ── editor state ───────────────────────────────────────────── */

interface PendingOp {
  version: number; /* the version this op should produce */
  op: NoteEdit;
}

export interface EditorState {
  noteId: NoteId;
  content: string;
  /** Version of the last server state we built on. */
  baseVersion: number;
  /** baseVersion + pending.length: what we predict the server will assign next. */
  predictedVersion: number;
  pending: PendingOp[];
  /** Bumped every time content is changed from OUTSIDE the textarea
   * (remote op, resync) so the view can restore the cursor. */
  externalChange: number;
  /** Cursor adjustment computed for the latest external change. */
  cursorShift: { from: number; shift: number } | null;
}

export const editor = writable<EditorState | null>(null);

/** Open a note in the editor. */
export function openEditor(note: Note) {
  editor.set({
    noteId: note.id,
    content: note.content,
    baseVersion: note.version,
    predictedVersion: note.version,
    pending: [],
    externalChange: 0,
    cursorShift: null,
  });
}

/** Close the editor. */
export function closeEditor() {
  editor.set(null);
}

/* ── typing: diff, predict, send ────────────────────────────── */

/** Common prefix/suffix diff between old and new textarea values.
 * Returns the op, or null if nothing changed.
 * `cursor` (selectionStart after the change) disambiguates cases
 * like "aa" -> "aaa" where prefix/suffix overlap. */
function diffToOp(
  oldText: string,
  newText: string,
  cursor: number,
): NoteEdit | null {
  if (oldText === newText) return null;
  let prefix = 0;
  const minLen = Math.min(oldText.length, newText.length);
  while (prefix < minLen && oldText[prefix] === newText[prefix]) prefix++;
  /* The change ends at the cursor for inserts; don't let the common
   * prefix run past it (handles repeated chars). */
  if (prefix > cursor) prefix = cursor;
  let suffix = 0;
  while (
    suffix < minLen - prefix &&
    oldText[oldText.length - 1 - suffix] ===
      newText[newText.length - 1 - suffix]
  ) {
    suffix++;
  }
  const removed = oldText.slice(prefix, oldText.length - suffix);
  const inserted = newText.slice(prefix, newText.length - suffix);

  if (removed.length === 0 && inserted.length > 0) {
    const { line, col } = indexToLineCol(oldText, prefix);
    return { op: "insert", line, col, text: inserted };
  }
  if (removed.length > 0 && inserted.length === 0) {
    const { line, col } = indexToLineCol(oldText, prefix);
    return { op: "delete", line, col, len: [...removed].length };
  }
  /* Replacement (selection overwrite, autocorrect): send as delete+insert.
   * Two ops, two versions. */
  return null; /* handled by caller splitting; see onInput below */
}

/** Called by the view on every textarea input. */
export function onLocalInput(newContent: string, cursor: number) {
  const state = get(editor);
  if (state === null) return;
  const oldContent = state.content;
  if (oldContent === newContent) return;

  /* Try the single-op diff; if it's a replacement, emit delete then insert. */
  const ops: NoteEdit[] = [];
  const single = diffToOp(oldContent, newContent, cursor);
  if (single !== null) {
    ops.push(single);
  } else {
    /* Replacement: recompute prefix/suffix, split in two ops. */
    let prefix = 0;
    const minLen = Math.min(oldContent.length, newContent.length);
    while (prefix < minLen && oldContent[prefix] === newContent[prefix])
      prefix++;
    if (prefix > cursor) prefix = cursor;
    let suffix = 0;
    while (
      suffix < minLen - prefix &&
      oldContent[oldContent.length - 1 - suffix] ===
        newContent[newContent.length - 1 - suffix]
    ) {
      suffix++;
    }
    const removed = oldContent.slice(prefix, oldContent.length - suffix);
    const inserted = newContent.slice(prefix, newContent.length - suffix);
    const { line, col } = indexToLineCol(oldContent, prefix);
    ops.push({ op: "delete", line, col, len: [...removed].length });
    ops.push({ op: "insert", line, col, text: inserted });
  }

  editor.update((s) => {
    if (s === null) return s;
    let version = s.predictedVersion;
    for (const op of ops) {
      version += 1;
      s.pending.push({ version, op });
      sendOp(s.noteId, version - 1, op);
    }
    return { ...s, content: newContent, predictedVersion: version };
  });
}

function sendOp(noteId: NoteId, baseVersion: number, op: NoteEdit) {
  editNoteRequest(noteId, baseVersion, op)
    .then((response) => {
      if (response.result === "Rejected") {
        /* Someone got in before us; full recovery. */
        resyncEditor(noteId);
      } else if (response.result === "Error") {
        console.error("edit error:", response.message);
        resyncEditor(noteId);
      }
      /* Ok: the echoed event does the bookkeeping. */
    })
    .catch((e) => {
      console.error("edit request failed:", e);
      resyncEditor(noteId);
    });
}

/* ── incoming events ────────────────────────────────────────── */

/** Called by ws dispatch for every NoteEdited event.
 * Returns the new content for the store if the note is not open, so
 * the caller can keep the store copy in sync. */
export function handleRemoteEdit(
  noteId: NoteId,
  version: number,
  op: NoteEdit,
) {
  const state = get(editor);
  if (state === null || state.noteId !== noteId) return;

  /* Our own echo? It must match the head of the pending queue. */
  const head = state.pending[0];
  if (head !== undefined && head.version === version && sameOp(head.op, op)) {
    editor.update((s) => {
      if (s === null) return s;
      s.pending.shift();
      return { ...s, baseVersion: version };
    });
    return;
  }

  /* Someone else's op. */
  if (state.pending.length > 0) {
    /* Interleaved with our un-acked typing: give up and resync. */
    resyncEditor(noteId);
    return;
  }

  /* Version gap: we missed an event. Resync. */
  if (version !== state.baseVersion + 1) {
    resyncEditor(noteId);
    return;
  }

  const newContent = applyEdit(state.content, op);
  if (newContent === null) {
    /* Op doesn't apply to what we hold: we're corrupted somehow. */
    resyncEditor(noteId);
    return;
  }

  /* Cursor adjustment: how much did the text before position X move? */
  const shift = cursorShiftFor(state.content, op);
  editor.update((s) => {
    if (s === null) return s;
    return {
      ...s,
      content: newContent,
      baseVersion: version,
      predictedVersion: version,
      externalChange: s.externalChange + 1,
      cursorShift: shift,
    };
  });
}

/** For a remote op, compute: positions >= `from` shift by `shift` utf-16 units. */
function cursorShiftFor(
  oldContent: string,
  op: NoteEdit,
): { from: number; shift: number } | null {
  if (op.op === "insert") {
    const index = lineColToIndex(oldContent, op.line, op.col);
    if (index === null) return null;
    return { from: index, shift: op.text.length };
  }
  if (op.op === "delete") {
    const start = lineColToIndex(
      oldContent,
      op.op === "delete" ? op.line : 0,
      op.col,
    );
    if (start === null) return null;
    const points = [...oldContent.slice(start)];
    let units = 0;
    for (let i = 0; i < Math.min(op.len, points.length); i++) {
      units += points[i].length;
    }
    return { from: start + units, shift: -units };
  }
  return null; /* ReplaceAll: no sensible cursor mapping */
}

function sameOp(a: NoteEdit, b: NoteEdit): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/* ── recovery ───────────────────────────────────────────────── */

let resyncing = false;

/** Fetch the authoritative note, reset the editor on it, drop pending. */
export async function resyncEditor(noteId: NoteId) {
  if (resyncing) return;
  resyncing = true;
  try {
    const response = await resyncNoteRequest(noteId);
    if (response.result === "Ok") {
      const note = response.note;
      applyNoteResynced(note);
      editor.update((s) => {
        if (s === null || s.noteId !== noteId) return s;
        return {
          noteId: s.noteId,
          content: note.content,
          baseVersion: note.version,
          predictedVersion: note.version,
          pending: [],
          externalChange: s.externalChange + 1,
          cursorShift: null,
        };
      });
    } else {
      console.error("resync error:", response.message);
    }
  } catch (e) {
    console.error("resync failed:", e);
  } finally {
    resyncing = false;
  }
}
