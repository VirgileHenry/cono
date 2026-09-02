const PORT_ENV_VAR: &str = "PORT";
const SERVE_PATH_ENV_VAR: &str = "SERVE_PATH";
const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";

/// Configuration for the server.
///
/// All configuration paramaters are pulled from the environment variables.
pub struct Config {
    pub port: u16,
    pub serve_path: std::path::PathBuf,
    pub database_url: String,
}

impl Config {
    pub fn new() -> anyhow::Result<Config> {
        let span = tracing::info_span!("create_config");
        let _guard = span.enter();

        /* Pull the port from the env var and parse it to u16 */
        let port: u16 = std::env::var(PORT_ENV_VAR)?.parse()?;

        /* Pull the serve directory from the env var and check path existence */
        let serve_path = std::env::var(SERVE_PATH_ENV_VAR)?;
        let serve_path = std::path::PathBuf::from(serve_path);

        /* Pull the card data path from the env var and check path existence */
        let database_url = std::env::var(DATABASE_URL_ENV_VAR)?;

        Ok(Self {
            port,
            serve_path,
            database_url,
        })
    }
}
