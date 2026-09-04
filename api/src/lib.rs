pub mod client_message;
pub mod client_request;
pub mod server_event;
pub mod server_message;
pub mod server_response;

/// A full note as a message to send to the client
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct Note {
    /// Unique identifier for the note.
    pub id: uuid::Uuid,
    /// Title of the note.
    pub title: String,
    /// full content of the note.
    pub content: String,
    /// Current version of the note.
    pub version: u64,
    /// Last update time of the note.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A single modification withing a note.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum NoteEdit {
    /// Insert the given text at the byte index provided by pos.
    /// pos must point to a valid character bound, or this will be rejected.
    Insert {
        /// Byte index in the note's content of the text to add.
        pos: usize,
        /// Text to insert in the note content.
        text: String,
    },
    /// Delete the bytes at `[pos..pos+len]`.
    /// Both boundaries shall be valid character bound, or this will be rejected.
    Delete {
        /// Byte index of the text to delete from the note's content.
        pos: usize,
        /// Length in bytes of the text to delete from the note's content.
        len: usize,
    },
}
