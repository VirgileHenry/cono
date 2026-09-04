//! Server to client HTTP responses.

/// Response of the sync request with all notes.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SyncResponse {
    /// All notes present on this server.
    pub notes: Vec<crate::Note>,
}

/// Response for a new note being created.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum CreateNoteResponse {
    /// The note has been created.
    Ok,
    /// The note could not be created.
    Error { message: String },
}

/// Response for a note being renamed.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum RenameNoteResponse {
    /// The note has been renamed.
    Ok,
    /// There was an error when renaming the note.
    Error { message: String },
}

/// Response to deleting an existing note.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum DeleteNoteResponse {
    /// The note has been deleted.
    Ok,
    /// There was an error when deleting the note.
    Error { message: String },
}

/// Rsponse of a resync of a single note.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ResyncNoteResponse {
    /// The note has been resynced, and the last note version is sent.
    Ok { note: crate::Note },
    /// There was an error when resyncing the note.
    Error { message: String },
}
