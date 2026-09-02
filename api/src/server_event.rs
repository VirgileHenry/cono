/// Event for a note being created.
/// This can be sent to any client without a request, since other users can create notes.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct NoteCreatedEvent {
    pub note: crate::Note,
}

/// Event for a note being edited.
/// This can be sent to any client without a request, since other users can edit notes.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct NoteEditedEvent {
    pub note_id: crate::NoteId,
    pub version: u64,
    pub op: crate::NoteEdit,
}

/// Event for a note being renamed.
/// This can be sent to any client without a request, since other users can rename notes.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct NoteRenamedEvent {
    pub note_id: crate::NoteId,
    pub version: u64,
    pub title: String,
}

/// Event for a note being deleted.
/// This can be sent to any client without a request, since other users can delete notes.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct NoteDeletedEvent {
    pub note_id: crate::NoteId,
}

/// Grouping of all server events behind a single type.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub enum ServerEvent {
    NoteCreated(NoteCreatedEvent),
    NoteEdited(NoteEditedEvent),
    NoteRenamed(NoteRenamedEvent),
    NoteDeleted(NoteDeletedEvent),
}
