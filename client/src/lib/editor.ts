import { get, writable } from "svelte/store";
import type {
  EditNoteMessage,
  EditNoteResultMessage,
  Note,
  NoteEdit,
  NoteId,
  ServerEvent,
} from "./protocol";
import { resyncNoteRequest } from "./api";
import { getClientId, SendNoteEdit } from "./ws";

export interface EditorCallbacks {
  /** Content changed from OUTSIDE the textarea (remote op / resync). */
  onExternalChange: (text: string) => void;
  /** Editor is requiring to close the note (the note got deleted) */
  onClose: () => void;
}

export class EditorState {
  /* Id of the note */
  noteId: NoteId;
  /* Content of the note */
  content: Uint8Array;
  /* Version of the last server state we built on. */
  baseVersion: number;
  /* Number of changes sent but not received yet */
  pendingCount: number;
  /* encoder for the textarea / utf8 content conversion */
  encoder = new TextEncoder();
  /* decoder for the textarea / utf8 content conversion */
  decoder = new TextDecoder();
  /* Are we currently doing a resync ? */
  resyncing = false;
  /* Callbacks */
  callbacks: EditorCallbacks;

  constructor(note: Note, callbacks: EditorCallbacks) {
    this.noteId = note.id;
    this.content = this.encoder.encode(note.content);
    this.baseVersion = note.version;
    this.pendingCount = 0;
    this.callbacks = callbacks;
  }

  /** Get the status of the editor for component display */
  status(): { version: number; pending: number; resyncing: boolean } {
    return {
      version: this.baseVersion,
      pending: this.pendingCount,
      resyncing: this.resyncing,
    };
  }

  /** We are in a bad state, stop everything and reload */
  async resync() {
    if (this.resyncing) return;

    console.log("Resyncing note...");
    this.resyncing = true;

    /* Send a resync request for the note */
    const response = await resyncNoteRequest(this.noteId);
    if (response.result === "ok") {
      this.content = this.encoder.encode(response.note.content);
      this.baseVersion = response.note.version;
    } else {
      /* Fuck it I'm out */
      this.callbacks.onClose();
    }

    const new_text = this.decoder.decode(this.content);
    this.callbacks.onExternalChange(new_text);
    this.resyncing = false;
  }

  /** Function for when the user changed anything in the note content. */
  userInput(textarea_content: string) {
    if (this.resyncing) return;

    const next_content = this.encoder.encode(textarea_content);
    const edits = diff(this.content, next_content, this.decoder);

    for (const edit of edits) this.sendLocalEdit(edit);

    /* Client side prediction */
    this.content = next_content;
  }

  /** Handle a single edit: send the edit to the server, and put it in the local pending ops. */
  sendLocalEdit(edit: NoteEdit) {
    if (this.resyncing) return;

    /* Finally, send the request now that our inner state is ready */
    if (SendNoteEdit(this.noteId, this.baseVersion, edit)) {
      this.pendingCount += 1;
    }
  }

  /** Handle a server event */
  handleEvent(event: ServerEvent) {
    if (event.kind === "note_edited" && event.event.note_id == this.noteId) {
      this.handleRemoteEdit(
        event.event.op,
        event.event.version,
        event.event.client_id,
      );
    }
  }

  /** Handle an edit sent by the server */
  handleRemoteEdit(op: NoteEdit, version: number, clientId: string) {
    if (this.resyncing) return;

    /* Every version produces exactly one event, in order: any gap means
     * we dropped a frame somewhere. */
    if (version !== this.baseVersion + 1) {
      console.warn(`version gap: got ${version}, at ${this.baseVersion}`);
      this.resync();
      return;
    }

    if (clientId === getClientId()) {
      /* Our own echo. Content already reflects it (and no foreign op
       * interleaved — we'd have resynced on its event first — so the
       * server applied it untransformed). */
      this.pendingCount -= 1;
    } else {
      if (this.pendingCount > 0) {
        console.warn(
          "Received foreign change with pending change: asking for resync",
        );
        this.resync();
        return;
      }

      this.content = applyNoteEditOp(this.content, op, this.encoder);
    }

    this.baseVersion = version;
    const new_text = this.decoder.decode(this.content);
    this.callbacks.onExternalChange(new_text);
  }
}

/** Apply an edit operation on a utf8 encoded note content */
export function applyNoteEditOp(
  content: Uint8Array,
  op: NoteEdit,
  encoder: TextEncoder,
): Uint8Array {
  if (op.op === "insert") {
    /* Insert the given text at the given pos */
    const start = content.slice(0, op.pos);
    const mid = encoder.encode(op.text);
    const end = content.slice(op.pos);
    return mergeUint8Arrays(start, mid, end);
  } else if (op.op === "delete") {
    /* Remove the given number of bytes from the given pos */
    const start = content.slice(0, op.pos);
    const end = content.slice(op.pos + op.len);
    return mergeUint8Arrays(start, end);
  } else /* op.op === "replace_all" */ {
    /* Replace the entire content */
    return encoder.encode(op.content);
  }
}

/** Compute a simple diff from two given content.
  This can create at most a single deletion and a single insertion. */
function diff(
  prev: Uint8Array,
  next: Uint8Array,
  decoder: TextDecoder,
): NoteEdit[] {
  /* get the common prefix and suffix of both content */
  const common_prefix = commonPrefix(prev, next);
  const common_suffix = commonSuffix(prev, next, common_prefix);

  const removed = prev.slice(common_prefix, prev.length - common_suffix);
  const inserted = next.slice(common_prefix, next.length - common_suffix);

  let edits: NoteEdit[] = [];

  if (removed.length > 0) {
    edits.push({ op: "delete", pos: common_prefix, len: removed.length });
  }
  if (inserted.length > 0) {
    const text = decoder.decode(inserted);
    edits.push({ op: "insert", pos: common_prefix, text });
  }

  return edits;
}

/** Get the length of the common prefix of two contents */
function commonPrefix(a: Uint8Array, b: Uint8Array): number {
  const shortest = a.length < b.length ? a : b;
  let result = 0;

  /* Greedy advance and compare */
  while (result < shortest.length && a[result] === b[result]) {
    result += 1;
  }

  /* clamp prefix down to a char boundary */
  while (
    result > 0 &&
    result < shortest.length &&
    isContinuationByte(a[result])
  ) {
    result -= 1;
  }

  return result;
}

/** Get the length of the common suffix of two contents,
 * ignoring the already computed common prefix
 */
function commonSuffix(a: Uint8Array, b: Uint8Array, prefix: number): number {
  const max = Math.min(a.length, b.length) - prefix;
  let result = 0;

  /* Greedy advance and compare */
  while (
    result < max &&
    a[a.length - 1 - result] === b[b.length - 1 - result]
  ) {
    result += 1;
  }

  /* clamp suffix so the cut in prev lands on a boundary */
  while (result > 0 && isContinuationByte(a[a.length - result])) {
    result -= 1;
  }

  return result;
}

/** Merge multiple uint 8 arrays
 *
 * Source - https://stackoverflow.com/a/76332760
 * Posted by Alex
 * Retrieved 2026-09-03, License - CC BY-SA 4.0
 */
function mergeUint8Arrays(...arrays: Uint8Array[]): Uint8Array {
  const totalSize = arrays.reduce((acc, e) => acc + e.length, 0);
  const merged = new Uint8Array(totalSize);

  arrays.forEach((array, i, arrays) => {
    const offset = arrays.slice(0, i).reduce((acc, e) => acc + e.length, 0);
    merged.set(array, offset);
  });

  return merged;
}

/** True if this byte is a UTF-8 continuation byte (0b10xxxxxx),
 * i.e. NOT the start of a character. */
function isContinuationByte(byte: number): boolean {
  return (byte & 0xc0) === 0x80;
}
