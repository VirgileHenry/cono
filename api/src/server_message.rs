//! Server to client WS messages, not broadcasted.

/// Response to a note edit request.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum EditNoteResultMessage {
    /// The note has been edited successefuly.
    Ok,
    /// The client is probably out of sync, please resync
    PleaseResync,
    /// There was an error when editing the note.
    InternalError { message: String },
}
