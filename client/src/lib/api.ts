import type {
  CreateNoteRequest,
  CreateNoteResponse,
  DeleteNoteRequest,
  DeleteNoteResponse,
  NoteId,
  RenameNoteRequest,
  RenameNoteResponse,
  ResyncNoteRequest,
  ResyncNoteResponse,
  SyncResponse,
} from "./protocol";

/** POST a JSON body, expect a JSON response.
 * Throws on network failure or non-2xx status. */
async function post<Req, Resp>(url: string, body: Req): Promise<Resp> {
  const response = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    throw new Error(`${url} failed: ${response.status}`);
  }
  return response.json();
}

/** Fetch the full state of all notes. */
export async function syncRequest(): Promise<SyncResponse> {
  const response = await fetch("/api/v1/sync");
  if (!response.ok) {
    throw new Error(`sync failed: ${response.status}`);
  }
  return response.json();
}

/** Create a new empty note with the given title. */
export async function createNoteRequest(
  title: string,
): Promise<CreateNoteResponse> {
  const request: CreateNoteRequest = { title };
  return post("/api/v1/create", request);
}

/** Rename an existing note. */
export async function renameNoteRequest(
  note_id: NoteId,
  base_version: number,
  title: string,
): Promise<RenameNoteResponse> {
  const request: RenameNoteRequest = { note_id, base_version, title };
  return post("/api/v1/rename", request);
}

/** Delete an existing note. */
export async function deleteNoteRequest(
  note_id: NoteId,
): Promise<DeleteNoteResponse> {
  const request: DeleteNoteRequest = { note_id };
  return post("/api/v1/delete", request);
}

/** Ask for a full reload of a single note. */
export async function resyncNoteRequest(
  note_id: NoteId,
): Promise<ResyncNoteResponse> {
  const request: ResyncNoteRequest = { note_id };
  return post("/api/v1/resync", request);
}
