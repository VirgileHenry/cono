pub async fn delete_old_updates(cleanup_state: std::sync::Arc<crate::state::State>) {
    use sea_orm::ColumnTrait;
    use sea_orm::EntityTrait;
    use sea_orm::QueryFilter;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
    loop {
        interval.tick().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(4);
        match crate::entities::note_edits::Entity::delete_many()
            .filter(crate::entities::note_edits::Column::CreatedAt.lt(cutoff))
            .exec(cleanup_state.db_conn())
            .await
        {
            Ok(res) => tracing::info!("trimmed {} old edits", res.rows_affected),
            Err(e) => tracing::error!("edit trim failed: {e}"),
        }
    }
}
