pub mod client_request;
pub mod server_event;
pub mod server_response;

/// Note identifier, a type safe wrapper around uuid.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteId(uuid::Uuid);

impl NoteId {
    pub fn new(note_id: uuid::Uuid) -> Self {
        Self(note_id)
    }
}

impl std::fmt::Display for NoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Note id: {}", self.0)
    }
}

impl Into<uuid::Uuid> for NoteId {
    fn into(self) -> uuid::Uuid {
        self.0
    }
}

/// A full note as a message to send to the client
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct Note {
    pub id: crate::NoteId,
    pub title: String,
    pub content: String,
    pub version: u64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A single modification withing a note.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum NoteEdit {
    Insert { line: u32, col: u32, text: String },
    Delete { line: u32, col: u32, len: u32 },
    ReplaceAll { content: String },
}
