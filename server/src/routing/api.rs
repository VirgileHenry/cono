use axum::extract;
use axum::http;
use axum::response::IntoResponse;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QueryOrder;
use sea_orm::TransactionTrait;
use tracing::Instrument;

/// Register all api method for v1
pub fn register_api(
    router: axum::Router<std::sync::Arc<crate::state::State>>,
) -> axum::Router<std::sync::Arc<crate::state::State>> {
    let router = router.route("/api/v1/sync", axum::routing::get(sync_request));
    let router = router.route("/api/v1/create", axum::routing::post(create_note_request));
    let router = router.route("/api/v1/edit", axum::routing::post(edit_note_request));
    let router = router.route("/api/v1/rename", axum::routing::post(rename_note_request));
    let router = router.route("/api/v1/delete", axum::routing::post(delete_note_request));
    let router = router.route("/api/v1/resync", axum::routing::post(resync_note_request));
    router
}

/// Request a full state of all notes.
pub async fn sync_request(
    extract::State(server_state): extract::State<std::sync::Arc<crate::state::State>>,
) -> axum::response::Response {
    let rid = uuid::Uuid::new_v4();
    let span = tracing::info_span!("sync", rid = %rid);
    async {
        let notes = match crate::entities::notes::Entity::find().all(server_state.db_conn()).await {
            Ok(notes) => notes,
            Err(e) => {
                tracing::error!("failed to fetch notes: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        let notes = notes.into_iter().map(note_model_to_api).collect();
        let response = api::server_response::SyncResponse { notes };
        axum::Json(response).into_response()
    }
    .instrument(span)
    .await
}

/// Create a new empty note.
pub async fn create_note_request(
    extract::State(server_state): extract::State<std::sync::Arc<crate::state::State>>,
    extract::Json(request): extract::Json<api::client_request::CreateNoteRequest>,
) -> axum::response::Response {
    let rid = uuid::Uuid::new_v4();
    let span = tracing::info_span!("create_note", rid = %rid);
    async {
        if request.title.is_empty() {
            tracing::error!("Note name is empty");
            return http::StatusCode::BAD_REQUEST.into_response();
        }
        let now = chrono::Utc::now();
        let model = crate::entities::notes::ActiveModel {
            id: sea_orm::ActiveValue::Set(uuid::Uuid::new_v4()),
            title: sea_orm::ActiveValue::Set(request.title),
            content: sea_orm::ActiveValue::Set(String::new()),
            version: sea_orm::ActiveValue::Set(0),
            created_at: sea_orm::ActiveValue::Set(now.into()),
            updated_at: sea_orm::ActiveValue::Set(now.into()),
        };
        let model = match model.insert(server_state.db_conn()).await {
            Ok(model) => model,
            Err(e) => {
                tracing::error!("failed to insert note: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        let response = api::server_response::CreateNoteResponse::Ok {
            note_id: api::NoteId::new(model.id),
        };
        let event = api::server_event::NoteCreatedEvent {
            note: note_model_to_api(model),
        };
        server_state.broadcast(api::server_event::ServerEvent::NoteCreated(event));
        axum::Json(response).into_response()
    }
    .instrument(span)
    .await
}

/// Edit an existing note: version check, apply op, log edit, bump version.
pub async fn edit_note_request(
    extract::State(server_state): extract::State<std::sync::Arc<crate::state::State>>,
    extract::Json(request): extract::Json<api::client_request::EditNoteRequest>,
) -> axum::response::Response {
    let rid = uuid::Uuid::new_v4();
    let span = tracing::info_span!("edit_note", rid = %rid, note_id = %request.note_id);
    async {
        let txn = match server_state.db_conn().begin().await {
            Ok(txn) => txn,
            Err(e) => {
                tracing::error!("failed to begin transaction: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        let note = match crate::entities::notes::Entity::find_by_id(request.note_id).one(&txn).await {
            Ok(Some(note)) => note,
            Ok(None) => return http::StatusCode::NOT_FOUND.into_response(),
            Err(e) => {
                tracing::error!("failed to fetch note: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        /* Optimistic lock: the client must be editing the version we hold. */
        if note.version as u64 != request.base_version {
            let response = api::server_response::EditNoteResponse::Rejected {
                note_id: request.note_id,
                current_version: note.version as u64,
            };
            return axum::Json(response).into_response();
        }
        let new_content = match apply_edit(&note.content, &request.op) {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!("op does not apply: {e}");
                /* Op is out of bounds for the content we hold: treat as a
                stale client, same recovery path as a version mismatch. */
                let response = api::server_response::EditNoteResponse::Rejected {
                    note_id: request.note_id,
                    current_version: note.version as u64,
                };
                return axum::Json(response).into_response();
            }
        };
        let new_version = note.version + 1;
        let now = chrono::Utc::now();
        /* Fetch the latest edit row and merge into it if possible, so
        consecutive keystrokes squash into one growing edit. */
        let note_uuid: uuid::Uuid = request.note_id.into();
        let last_edit = match crate::entities::note_edits::Entity::find()
            .filter(crate::entities::note_edits::Column::NoteId.eq(note_uuid))
            .order_by_desc(crate::entities::note_edits::Column::Version)
            .one(&txn)
            .await
        {
            Ok(last_edit) => last_edit,
            Err(e) => {
                tracing::error!("failed to fetch last edit: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        let merged = last_edit.as_ref().and_then(|last| {
            let last_op: api::NoteEdit = serde_json::from_value(last.op.clone()).ok()?;
            try_merge(&last_op, &request.op)
        });
        /* Insert the edit in the table, attempting to merge with the last edit */
        match (merged, last_edit) {
            (Some(merged_op), Some(last_edit)) => {
                /* Grow the previous row in place: it now covers every
                version from its first up to new_version. */
                let op_json = match serde_json::to_value(&merged_op) {
                    Ok(json) => json,
                    Err(e) => {
                        tracing::error!("failed to serialize merged op: {e}");
                        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                };
                let mut last_edit: crate::entities::note_edits::ActiveModel = last_edit.into();
                last_edit.op = sea_orm::ActiveValue::Set(op_json);
                last_edit.version = sea_orm::ActiveValue::Set(new_version);
                last_edit.created_at = sea_orm::ActiveValue::Set(now.into());
                if let Err(e) = last_edit.update(&txn).await {
                    tracing::error!("failed to update merged edit: {e}");
                    return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
            _ => {
                let op_json = match serde_json::to_value(&request.op) {
                    Ok(json) => json,
                    Err(e) => {
                        tracing::error!("failed to serialize op: {e}");
                        return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                };
                let edit = crate::entities::note_edits::ActiveModel {
                    id: sea_orm::ActiveValue::NotSet,
                    note_id: sea_orm::ActiveValue::Set(request.note_id.into()),
                    version: sea_orm::ActiveValue::Set(new_version),
                    op: sea_orm::ActiveValue::Set(op_json),
                    created_at: sea_orm::ActiveValue::Set(now.into()),
                };
                if let Err(e) = edit.insert(&txn).await {
                    tracing::error!("failed to insert edit: {e}");
                    return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
        let mut note: crate::entities::notes::ActiveModel = note.into();
        note.content = sea_orm::ActiveValue::Set(new_content);
        note.version = sea_orm::ActiveValue::Set(new_version);
        note.updated_at = sea_orm::ActiveValue::Set(now.into());
        if let Err(e) = note.update(&txn).await {
            tracing::error!("failed to update note: {e}");
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        if let Err(e) = txn.commit().await {
            tracing::error!("failed to commit edit: {e}");
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        /* Only broadcast once the transaction is committed. */
        let event = api::server_event::NoteEditedEvent {
            note_id: request.note_id,
            version: new_version as u64,
            op: request.op,
        };
        server_state.broadcast(api::server_event::ServerEvent::NoteEdited(event));
        axum::Json(api::server_response::EditNoteResponse::Ok).into_response()
    }
    .instrument(span)
    .await
}

/// Rename an existing note.
pub async fn rename_note_request(
    extract::State(server_state): extract::State<std::sync::Arc<crate::state::State>>,
    extract::Json(request): extract::Json<api::client_request::RenameNoteRequest>,
) -> axum::response::Response {
    let rid = uuid::Uuid::new_v4();
    let span = tracing::info_span!("rename_note", rid = %rid, note_id = %request.note_id);
    async {
        let txn = match server_state.db_conn().begin().await {
            Ok(txn) => txn,
            Err(e) => {
                tracing::error!("failed to begin transaction: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        let note = match crate::entities::notes::Entity::find_by_id(request.note_id).one(&txn).await {
            Ok(Some(note)) => note,
            Ok(None) => return http::StatusCode::NOT_FOUND.into_response(),
            Err(e) => {
                tracing::error!("failed to fetch note: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        if note.version as u64 != request.base_version {
            /* Rename has no dedicated Rejected variant: reuse Error for now. */
            let response = api::server_response::RenameNoteResponse::Error {
                message: format!("stale version, current is {}", note.version),
            };
            return axum::Json(response).into_response();
        }
        let new_version = note.version + 1;
        let now = chrono::Utc::now();
        let title = request.title;
        let mut note: crate::entities::notes::ActiveModel = note.into();
        note.title = sea_orm::ActiveValue::Set(title.clone());
        note.version = sea_orm::ActiveValue::Set(new_version);
        note.updated_at = sea_orm::ActiveValue::Set(now.into());
        if let Err(e) = note.update(&txn).await {
            tracing::error!("failed to update note: {e}");
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        if let Err(e) = txn.commit().await {
            tracing::error!("failed to commit rename: {e}");
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        let event = api::server_event::NoteRenamedEvent {
            note_id: request.note_id,
            version: new_version as u64,
            title,
        };
        server_state.broadcast(api::server_event::ServerEvent::NoteRenamed(event));
        axum::Json(api::server_response::RenameNoteResponse::Ok).into_response()
    }
    .instrument(span)
    .await
}

/// Delete an existing note.
pub async fn delete_note_request(
    extract::State(server_state): extract::State<std::sync::Arc<crate::state::State>>,
    extract::Json(request): extract::Json<api::client_request::DeleteNoteRequest>,
) -> axum::response::Response {
    let rid = uuid::Uuid::new_v4();
    let span = tracing::info_span!("delete_note", rid = %rid, note_id = %request.note_id);
    async {
        let result = match crate::entities::notes::Entity::delete_by_id(request.note_id)
            .exec(server_state.db_conn())
            .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("failed to delete note: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        if result.rows_affected == 0 {
            return http::StatusCode::NOT_FOUND.into_response();
        }
        let event = api::server_event::NoteDeletedEvent {
            note_id: request.note_id,
        };
        server_state.broadcast(api::server_event::ServerEvent::NoteDeleted(event));
        axum::Json(api::server_response::DeleteNoteResponse::Ok).into_response()
    }
    .instrument(span)
    .await
}

/// Full reload of a single note.
pub async fn resync_note_request(
    extract::State(server_state): extract::State<std::sync::Arc<crate::state::State>>,
    extract::Json(request): extract::Json<api::client_request::ResyncNoteRequest>,
) -> axum::response::Response {
    let rid = uuid::Uuid::new_v4();
    let span = tracing::info_span!("resync_note", rid = %rid, note_id = %request.note_id);
    async {
        let note = match crate::entities::notes::Entity::find_by_id(request.note_id)
            .one(server_state.db_conn())
            .await
        {
            Ok(Some(note)) => note,
            Ok(None) => return http::StatusCode::NOT_FOUND.into_response(),
            Err(e) => {
                tracing::error!("failed to fetch note: {e}");
                return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        let response = api::server_response::ResyncNoteResponse::Ok {
            note: note_model_to_api(note),
        };
        axum::Json(response).into_response()
    }
    .instrument(span)
    .await
}

/// Convert a database note model to the api note.
fn note_model_to_api(model: crate::entities::notes::Model) -> api::Note {
    api::Note {
        id: api::NoteId::new(model.id),
        title: model.title,
        content: model.content,
        version: model.version as u64,
        updated_at: model.updated_at.into(),
    }
}

/// Apply an edit op to a note content, or fail if the op is out of bounds.
fn apply_edit(content: &str, op: &api::NoteEdit) -> anyhow::Result<String> {
    match op {
        api::NoteEdit::Insert { line, col, text } => {
            let index = line_col_to_index(content, *line, *col)?;
            let mut content = content.to_string();
            content.insert_str(index, text);
            Ok(content)
        }
        api::NoteEdit::Delete { line, col, len } => {
            let start = line_col_to_index(content, *line, *col)?;
            /* len is in chars, find the byte index of the char after the range. */
            let end = content[start..]
                .char_indices()
                .nth(*len as usize)
                .map(|(i, _)| start + i)
                .unwrap_or(content.len());
            let removed_chars = content[start..end].chars().count();
            anyhow::ensure!(
                removed_chars == *len as usize,
                "delete of {len} chars at {line}:{col} overruns content"
            );
            let mut content = content.to_string();
            content.replace_range(start..end, "");
            Ok(content)
        }
        api::NoteEdit::ReplaceAll { content } => Ok(content.clone()),
    }
}

/// Translate a (line, col) position into a byte index, in chars, or fail if out of bounds.
fn line_col_to_index(content: &str, line: u32, col: u32) -> anyhow::Result<usize> {
    let mut lines = content.split('\n');
    let mut offset = 0;
    for _ in 0..line {
        let l = lines.next().ok_or_else(|| anyhow::anyhow!("line {line} out of bounds"))?;
        offset += l.len() + 1; /* +1 for the '\n' */
    }
    let l = lines.next().ok_or_else(|| anyhow::anyhow!("line {line} out of bounds"))?;
    let col_byte = l.char_indices().nth(col as usize).map(|(i, _)| i).unwrap_or_else(|| l.len());
    anyhow::ensure!(col as usize <= l.chars().count(), "col {col} out of bounds on line {line}");
    Ok(offset + col_byte)
}

/// Try to merge a new edit into the previous one.
///
/// Two inserts merge when the new one continues exactly at the end of the
/// previous one (typing forward). Two deletes merge when deleting forward
/// at the same spot (delete key) or backward ending where the previous
/// started (backspace). Anything else, including ops spanning lines,
/// does not merge.
fn try_merge(prev: &api::NoteEdit, next: &api::NoteEdit) -> Option<api::NoteEdit> {
    match (prev, next) {
        (
            api::NoteEdit::Insert { line, col, text },
            api::NoteEdit::Insert {
                line: next_line,
                col: next_col,
                text: next_text,
            },
        ) => {
            /* Inserted text with newlines moves the cursor to another line:
            the simple "same line, continuing col" rule no longer applies. */
            if text.contains('\n') || next_text.contains('\n') {
                return None;
            }
            let continues = *next_line == *line && *next_col as usize == *col as usize + text.chars().count();
            if !continues {
                return None;
            }
            let mut text = text.clone();
            text.push_str(next_text);
            Some(api::NoteEdit::Insert {
                line: *line,
                col: *col,
                text,
            })
        }
        (
            api::NoteEdit::Delete { line, col, len },
            api::NoteEdit::Delete {
                line: next_line,
                col: next_col,
                len: next_len,
            },
        ) => {
            if *next_line != *line {
                return None;
            }
            /* Forward delete: deleting again at the same position. */
            if *next_col == *col {
                return Some(api::NoteEdit::Delete {
                    line: *line,
                    col: *col,
                    len: len + next_len,
                });
            }
            /* Backspace: the new delete ends exactly where the previous started. */
            if *next_col + *next_len == *col {
                return Some(api::NoteEdit::Delete {
                    line: *line,
                    col: *next_col,
                    len: len + next_len,
                });
            }
            None
        }
        _ => None,
    }
}
