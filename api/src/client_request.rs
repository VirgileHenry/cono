//! Client to server HTTP requests.

/// Resquest a full notes states to the server.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SyncRequest;

/// Create a new note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateNoteRequest {
    /// Title of the note to create.
    pub title: String,
}

/// Request to rename an existing note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct RenameNoteRequest {
    /// Id of the note to rename.
    pub note_id: uuid::Uuid,
    /// Version of the note we apply our change on.
    /// If this version is not the last version on the server,
    /// the client is behind and the request will be rejected.
    pub base_version: u64,
    /// New note title.
    pub title: String,
}

/// Request to delete an existing note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DeleteNoteRequest {
    /// Id of the note to delete.
    pub note_id: uuid::Uuid,
}

/// Ask for a full reload of a given note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResyncNoteRequest {
    /// Id of the note to resync.
    pub note_id: uuid::Uuid,
}
