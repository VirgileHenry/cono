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
    ws.on_upgrade(move |socket| handle_socket(socket, receiver))
}

async fn handle_socket(
    mut socket: axum::extract::ws::WebSocket,
    mut receiver: tokio::sync::broadcast::Receiver<api::server_event::ServerEvent>,
) {
    loop {
        /* Only listen to the receiver, we don't expect any messages from the websocket */
        match receiver.recv().await {
            Ok(event) => {
                let json = serde_json::to_string(&event).expect("event serializes");
                if socket.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                    /* Client left, close */
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(amount)) => {
                tracing::warn!("Client lagged behind and lost {amount} messages. Disconnecting.");
                break;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}
