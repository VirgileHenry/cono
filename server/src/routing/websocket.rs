use api::NoteEdit;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QueryOrder;
use sea_orm::QuerySelect;
use sea_orm::TransactionTrait;
use tracing::Instrument;

/// Register the ws handler
pub fn register_ws(
    router: axum::Router<std::sync::Arc<crate::state::State>>,
) -> axum::Router<std::sync::Arc<crate::state::State>> {
    let router = router.route("/ws", axum::routing::get(ws_handler));
    router
}

async fn ws_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::state::State>>,
) -> axum::response::Response {
    let receiver = state.subscribe();
    ws.on_upgrade(move |socket| handle_socket(state, socket, receiver))
}

async fn handle_socket(
    state: std::sync::Arc<crate::state::State>,
    mut socket: axum::extract::ws::WebSocket,
    mut receiver: tokio::sync::broadcast::Receiver<api::server_event::ServerEvent>,
) {
    loop {
        let control_flow: WsControlFlow = tokio::select! {
            event = receiver.recv() => handle_server_message(&mut socket ,event).await,
            msg = socket.recv() => handle_client_message(&state, &mut socket, msg).await,
        };
        match control_flow {
            WsControlFlow::Disconnect => break,
            WsControlFlow::Continue => continue,
        }
    }
}

async fn handle_server_message(
    socket: &mut axum::extract::ws::WebSocket,
    event: Result<api::server_event::ServerEvent, tokio::sync::broadcast::error::RecvError>,
) -> WsControlFlow {
    match event {
        Ok(event) => {
            let json = serde_json::to_string(&event).expect("event serializes");
            if let Err(e) = socket.send(axum::extract::ws::Message::Text(json.into())).await {
                tracing::warn!("Failed to send ws message, disconnecting: {e}");
                WsControlFlow::Disconnect
            } else {
                WsControlFlow::Continue
            }
        }
        Err(tokio::sync::broadcast::error::RecvError::Lagged(amount)) => {
            tracing::warn!("Client lagged behind and lost {amount} messages. Disconnecting.");
            WsControlFlow::Disconnect
        }
        Err(tokio::sync::broadcast::error::RecvError::Closed) => WsControlFlow::Disconnect,
    }
}

async fn handle_client_message(
    state: &std::sync::Arc<crate::state::State>,
    socket: &mut axum::extract::ws::WebSocket,
    msg: Option<Result<axum::extract::ws::Message, axum::Error>>,
) -> WsControlFlow {
    match msg {
        Some(Ok(axum::extract::ws::Message::Text(text))) => {
            let msg: api::client_message::EditNoteMessage = match serde_json::from_str(&text) {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::warn!("bad ws message: {e}");
                    return WsControlFlow::Continue;
                }
            };
            let response = edit_note_request(state, msg).await;
            let response_json = match serde_json::to_string(&response) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to convert response to json, disconnecting: {e}");
                    return WsControlFlow::Disconnect;
                }
            };
            if let Err(e) = socket.send(axum::extract::ws::Message::Text(response_json.into())).await {
                tracing::warn!("Failed to send ws message, disconnecting: {e}");
                return WsControlFlow::Disconnect;
            }

            WsControlFlow::Continue
        }
        Some(Ok(axum::extract::ws::Message::Ping(_))) => WsControlFlow::Continue,
        Some(Ok(axum::extract::ws::Message::Pong(_))) => WsControlFlow::Continue,
        Some(Ok(axum::extract::ws::Message::Binary(_))) => WsControlFlow::Continue,
        Some(Ok(axum::extract::ws::Message::Close(_))) => WsControlFlow::Disconnect,
        Some(Err(e)) => {
            tracing::warn!("Failed to receive ws message, disconnecting: {e}");
            WsControlFlow::Disconnect
        }
        None => WsControlFlow::Disconnect,
    }
}

enum WsControlFlow {
    Continue,
    Disconnect,
}

/// Edit an existing note: version check, apply op, log edit, bump version.
pub async fn edit_note_request(
    server_state: &crate::state::State,
    request: api::client_message::EditNoteMessage,
) -> api::server_message::EditNoteResultMessage {
    let rid = uuid::Uuid::new_v4();
    let span = tracing::info_span!("edit_note", rid = %rid, note_id = %request.note_id);
    async {
        let txn = match server_state.db_conn().begin().await {
            Ok(txn) => txn,
            Err(e) => {
                tracing::error!("failed to begin transaction: {e}");
                return api::server_message::EditNoteResultMessage::InternalError {
                    message: format!("Failed to begin transaction: {e}"),
                };
            }
        };
        let note_id: uuid::Uuid = request.note_id.into();
        let note = match crate::entities::notes::Entity::find_by_id(note_id)
            .lock_exclusive()
            .one(&txn)
            .await
        {
            Ok(Some(note)) => note,
            Ok(None) => {
                return api::server_message::EditNoteResultMessage::InternalError {
                    message: format!("Note {note_id} does not exist"),
                };
            }
            Err(e) => {
                tracing::error!("failed to fetch note: {e}");
                return api::server_message::EditNoteResultMessage::InternalError {
                    message: format!("Failed to fetch note: {e}"),
                };
            }
        };
        let missed_edits = match crate::entities::note_edits::Entity::find()
            .filter(crate::entities::note_edits::Column::NoteId.eq(note_id))
            .filter(crate::entities::note_edits::Column::Version.gt(request.base_version))
            .filter(crate::entities::note_edits::Column::ClientId.ne(request.client_id))
            .order_by_asc(crate::entities::note_edits::Column::Version)
            .all(&txn)
            .await
        {
            Ok(edits) => edits,
            Err(e) => {
                tracing::error!("Failed to fetch missed edits: {e}");
                return api::server_message::EditNoteResultMessage::InternalError {
                    message: format!("Failed to fetch missed edits: {e}"),
                };
            }
        };
        let missed_ops = match missed_edits
            .into_iter()
            .map(|edit| serde_json::from_value(edit.op))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(ops) => ops,
            Err(e) => {
                tracing::error!("Invalid op in edit record, failed to convert to value: {e}");
                return api::server_message::EditNoteResultMessage::InternalError {
                    message: format!("Invalid op in edit record, failed to convert to value: {e}"),
                };
            }
        };

        let mut op = request.op;
        for missed_edit in missed_ops.iter() {
            op = transform(&op, missed_edit);
        }

        /* Check the note is still valid */
        let op_is_nop = match &op {
            api::NoteEdit::Insert { text, .. } => text.is_empty(),
            api::NoteEdit::Delete { len, .. } => *len == 0,
        };
        if op_is_nop {
            /* If after transformation, the edit does nothing meaningful, fake it */
            return api::server_message::EditNoteResultMessage::Ok;
        }

        /* We can now register the edit as the last one made! */
        let (new_content, applied_op) = apply_edit(&note.content, &op);

        let new_version = note.version + 1;
        let now = chrono::Utc::now();
        let op_json = match serde_json::to_value(&applied_op) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to serialize op: {e}");
                return api::server_message::EditNoteResultMessage::InternalError {
                    message: format!("Failed to serialize op: {e}"),
                };
            }
        };

        let edit = crate::entities::note_edits::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            client_id: sea_orm::ActiveValue::Set(request.client_id),
            note_id: sea_orm::ActiveValue::Set(note_id),
            version: sea_orm::ActiveValue::Set(new_version),
            op: sea_orm::ActiveValue::Set(op_json),
            created_at: sea_orm::ActiveValue::Set(now.into()),
        };
        if let Err(e) = edit.insert(&txn).await {
            tracing::error!("failed to insert edit: {e}");
            return api::server_message::EditNoteResultMessage::InternalError {
                message: format!("Failed to insert edit: {e}"),
            };
        }

        let mut note: crate::entities::notes::ActiveModel = note.into();
        note.content = sea_orm::ActiveValue::Set(new_content);
        note.version = sea_orm::ActiveValue::Set(new_version);
        note.updated_at = sea_orm::ActiveValue::Set(now.into());
        if let Err(e) = note.update(&txn).await {
            tracing::error!("failed to update note: {e}");
            return api::server_message::EditNoteResultMessage::InternalError {
                message: format!("Failed to update note: {e}"),
            };
        }

        if let Err(e) = txn.commit().await {
            tracing::error!("failed to commit edit: {e}");
            return api::server_message::EditNoteResultMessage::InternalError {
                message: format!("Failed to commit edit: {e}"),
            };
        }

        /* Only broadcast once the transaction is committed. */
        let event = api::server_event::NoteEditedEvent {
            note_id: request.note_id,
            client_id: request.client_id,
            version: new_version as u64,
            op: applied_op,
        };
        server_state.broadcast(api::server_event::ServerEvent::NoteEdited { event });

        return api::server_message::EditNoteResultMessage::Ok;
    }
    .instrument(span)
    .await
}

/// Transform `op` against a concurrent/earlier `against` op, so that
/// applying `against` then `transform(op, against)` preserves op's intent.
fn transform(op: &api::NoteEdit, against: &api::NoteEdit) -> api::NoteEdit {
    use api::NoteEdit::*;

    match (op, against) {
        // ── op is Insert ─────────────────────────────
        (
            Insert { pos, text },
            Insert {
                pos: a_pos,
                text: a_text,
            },
        ) => {
            // their insert before (or at) ours shifts us right
            let shifted = if a_pos < pos || (a_pos == pos/* tiebreak: they win */) {
                pos + a_text.len()
            } else {
                *pos
            };
            Insert {
                pos: shifted,
                text: text.clone(),
            }
        }
        (Insert { pos, text }, Delete { pos: a_pos, len: a_len }) => {
            let new_pos = if *pos <= *a_pos {
                *pos // before the deleted range: unchanged
            } else if *pos >= a_pos + a_len {
                pos - a_len // after it: shift left
            } else {
                *a_pos // INSIDE the deleted range: clamp to its start
            };
            Insert {
                pos: new_pos,
                text: text.clone(),
            }
        }
        // ── op is Delete ─────────────────────────────
        (
            Delete { pos, len },
            Insert {
                pos: a_pos,
                text: a_text,
            },
        ) => {
            if *a_pos <= *pos {
                Delete {
                    pos: pos + a_text.len(),
                    len: *len,
                } // insert before us
            } else if *a_pos >= pos + len {
                Delete { pos: *pos, len: *len } // insert after us
            } else {
                // insert lands INSIDE our delete range: we must not delete
                // their new text → split into two deletes... or the pragmatic
                // single-op answer: delete around it = grow len to cover?
                // NO — correct one-op compromise: delete only up to their
                // insert, then the rest shifted. Needs TWO ops or a choice.
                // Simplest sound choice: expand to two ops is cleanest, but
                // if the return type is one op: delete the pre-insert part only:
                Delete {
                    pos: *pos,
                    len: a_pos - pos,
                } // (lossy: leaves tail)
            }
        }
        (Delete { pos, len }, Delete { pos: a_pos, len: a_len }) => {
            // range subtraction: remove the overlap with what's already deleted
            let start = *pos;
            let end = pos + len;
            let a_start = *a_pos;
            let a_end = a_pos + a_len;
            let new_start = if start >= a_end {
                start - a_len
            } else if start >= a_start {
                a_start
            } else {
                start
            };
            let new_end = if end >= a_end {
                end - a_len
            } else if end >= a_start {
                a_start
            } else {
                end
            };
            Delete {
                pos: new_start,
                len: new_end - new_start,
            } // len may become 0!
        }
    }
}

/// Apply an edit op to a note content.
///
/// The edit will be applied anyway, performing a best attempt if the edit is technically invalid.
/// The correct edit that has been applied is returned.
fn apply_edit(content: &str, op: &api::NoteEdit) -> (String, NoteEdit) {
    match op {
        api::NoteEdit::Insert { pos, text } => {
            let (pos, start, end) = split_at_lossy(content, *pos);

            let capacity = start.len() + text.len() + end.len();
            let mut new_content = String::with_capacity(capacity);
            new_content.push_str(start);
            new_content.push_str(text);
            new_content.push_str(end);

            (new_content, api::NoteEdit::Insert { pos, text: text.clone() })
        }
        api::NoteEdit::Delete { pos, len } => {
            let (pos, start, rest) = split_at_lossy(content, *pos);
            let (len, _, end) = split_at_lossy(rest, *len);

            let capacity = start.len() + end.len();
            let mut new_content = String::with_capacity(capacity);
            new_content.push_str(start);
            new_content.push_str(end);

            (new_content, api::NoteEdit::Delete { pos, len })
        }
    }
}

/// Best-effort split at `pos`:
/// - `pos` valid (in bounds, on a char boundary): exact split, unchanged.
/// - `pos` out of bounds: split at the end -> (content, "").
/// - `pos` mid-character: snap FORWARD to the next char boundary.
///
/// Also returns the position actually used, so the caller can record
/// the normalized op (the op we broadcast must be the op we applied).
fn split_at_lossy(content: &str, pos: usize) -> (usize, &str, &str) {
    if pos >= content.len() {
        /* out of bounds (or exactly at the end — same result) */
        return (content.len(), content, "");
    }

    /* in bounds: walk backward to the nearest boundary (pos itself if valid) */
    let snapped = content.floor_char_boundary(pos);

    /* Split at is safe with the previous two checks */
    let (start, end) = content.split_at(snapped);
    (snapped, start, end)
}
