//! Client to server WS messages.

/// Request to edit an existing note.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EditNoteMessage {
    /// Id of the note to edit.
    pub note_id: uuid::Uuid,
    /// Unique client session identifier.
    pub client_id: uuid::Uuid,
    /// Version of the note we apply our change on.
    /// If this version is not the last version on the server,
    /// the client is behind and the request will be rejected.
    pub base_version: u64,
    /// Edit operation on the note.
    pub op: crate::NoteEdit,
}
