/// The server state is the main data storage of the server.
///
/// It contains the entry points to the two backends, myr and solemn.
pub struct State {
    /// Structure holding all the routes, which are all the statically served files.
    routes: crate::routing::Routes,

    /// Connection to the database
    db_conn: sea_orm::DatabaseConnection,

    /// Fan-out for server events: anything sent here reaches every connected client.
    events: tokio::sync::broadcast::Sender<api::server_event::ServerEvent>,
}

impl State {
    pub async fn new(config: &crate::config::Config) -> anyhow::Result<Self> {
        use tracing::Instrument;

        async {
            let routes = crate::routing::Routes::new(&config.serve_path)?;
            let db_conn = sea_orm::Database::connect(&config.database_url).await?;
            let (events, _) = tokio::sync::broadcast::channel(256);
            Ok(Self { events, db_conn, routes })
        }
        .instrument(tracing::info_span!("create_state"))
        .await
    }

    pub fn routes(&self) -> &crate::routing::Routes {
        &self.routes
    }

    /// Broadcast a server event to all connected websockets.
    pub fn broadcast(&self, event: api::server_event::ServerEvent) {
        /* Err just means "no clients connected right now", ignore. */
        let _ = self.events.send(event);
    }

    /// Susbcribe a new websocket to the broadcast channel.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<api::server_event::ServerEvent> {
        self.events.subscribe()
    }

    /// Get the connection to the database
    pub fn db_conn(&self) -> &(impl sea_orm::ConnectionTrait + sea_orm::TransactionTrait) {
        &self.db_conn
    }
}
