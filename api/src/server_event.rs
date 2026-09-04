//! Server to client broadcast WS messages (events).

/// Event for a note being created.
/// This can be sent to any client without a request, since other users can create notes.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct NoteCreatedEvent {
    /// note that was created.
    pub note: crate::Note,
}

/// Event for a note being edited.
/// This can be sent to any client without a request, since other users can edit notes.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct NoteEditedEvent {
    /// Id of the note that was edited.
    pub note_id: uuid::Uuid,
    /// Id of the active connection that edited the note.
    pub client_id: uuid::Uuid,
    /// new version of the note.
    pub version: u64,
    /// edit operation on the note.
    pub op: crate::NoteEdit,
}

/// Event for a note being renamed.
/// This can be sent to any client without a request, since other users can rename notes.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct NoteRenamedEvent {
    /// Id of the note that has been renamed.
    pub note_id: uuid::Uuid,
    /// New version of the note.
    pub version: u64,
    /// New title of the note.
    pub title: String,
}

/// Event for a note being deleted.
/// This can be sent to any client without a request, since other users can delete notes.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct NoteDeletedEvent {
    /// Id of the note that has been deleted.
    pub note_id: uuid::Uuid,
}

/// Grouping of all server events behind a single type.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum ServerEvent {
    NoteCreated { event: NoteCreatedEvent },
    NoteEdited { event: NoteEditedEvent },
    NoteRenamed { event: NoteRenamedEvent },
    NoteDeleted { event: NoteDeletedEvent },
}
