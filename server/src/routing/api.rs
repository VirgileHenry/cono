use axum::extract;
use axum::http;
use axum::response::IntoResponse;
use sea_orm::ActiveModelTrait;
use sea_orm::EntityTrait;
use sea_orm::TransactionTrait;
use tracing::Instrument;

/// Register all api method for v1
pub fn register_api(
    router: axum::Router<std::sync::Arc<crate::state::State>>,
) -> axum::Router<std::sync::Arc<crate::state::State>> {
    let router = router.route("/api/v1/sync", axum::routing::get(sync_request));
    let router = router.route("/api/v1/create", axum::routing::post(create_note_request));
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
        let event = api::server_event::NoteCreatedEvent {
            note: note_model_to_api(model),
        };
        server_state.broadcast(api::server_event::ServerEvent::NoteCreated { event });
        axum::Json(api::server_response::CreateNoteResponse::Ok).into_response()
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
        server_state.broadcast(api::server_event::ServerEvent::NoteRenamed { event });
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
        server_state.broadcast(api::server_event::ServerEvent::NoteDeleted { event });
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
        id: model.id,
        title: model.title,
        content: model.content,
        version: model.version as u64,
        updated_at: model.updated_at.into(),
    }
}
