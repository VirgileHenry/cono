//! Client to server messages.

/// Resquest a full notes states to the server.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SyncRequest;

/// Create a new note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
}

/// Request to edit an existing note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EditNoteRequest {
    pub note_id: crate::NoteId,
    pub base_version: u64, // version the client applied this on
    pub op: crate::NoteEdit,
}

/// Request to rename an existing note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct RenameNoteRequest {
    pub note_id: crate::NoteId,
    pub base_version: u64,
    pub title: String,
}

/// Request to delete an existing note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DeleteNoteRequest {
    pub note_id: crate::NoteId,
}

/// Ask for a full reload of a given note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResyncNoteRequest {
    pub note_id: crate::NoteId,
}
