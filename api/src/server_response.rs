//! Server to client messages.

/// Response of the sync request with all notes.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SyncResponse {
    pub notes: Vec<crate::Note>,
}

/// Response for a new note being created.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result")]
pub enum CreateNoteResponse {
    Ok { note_id: crate::NoteId },
    Error { message: String },
}

/// Response to a note edit request.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result")]
pub enum EditNoteResponse {
    Ok,
    Rejected { note_id: crate::NoteId, current_version: u64 },
    Error { message: String },
}

/// Response for a note being renamed.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result")]
pub enum RenameNoteResponse {
    Ok,
    Error { message: String },
}

/// Response to deleting an existing note.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result")]
pub enum DeleteNoteResponse {
    Ok,
    Error { message: String },
}

/// Rsponse of a resync of a single note.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result")]
pub enum ResyncNoteResponse {
    Ok { note: crate::Note },
    Error { message: String },
}
