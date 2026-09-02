mod config;
mod entities;
mod routing;
mod state;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    /* Initialize a global tracing subscriber based on the RUST_LOG env var */
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info,sqlx=warn"))
        .init();
    tracing::info!("Hello, Brainstorm!");

    /* Call the inner main function once the logging is ready */
    let main_result = run().await;

    /* Handle shutdown */
    match main_result {
        Ok(_) => {}
        Err(err) => {
            /* Log the fatal error that caused the server to stop */
            tracing::error!("{err}");
            std::process::exit(-1);
        }
    }
}

async fn run() -> anyhow::Result<()> {
    /* Initialize: get the server config */
    let span = tracing::info_span!("init");
    let guard = span.enter();

    let config = config::Config::new()?;
    let state = state::State::new(&config).await?;
    let router = routing::create(state).await?;
    tracing::info!("Successefuly initialized server.");
    drop(guard);

    /* Starting the server! */
    tracing::info!("Starting server...");
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.port));
    axum_server::bind(addr).serve(router.into_make_service()).await?;

    Ok(())
}
