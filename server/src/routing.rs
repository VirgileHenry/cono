//! The routing module creates and manages all the routes for the Oglor server.

mod api;
mod redirects;
mod static_response;
mod websocket;

pub async fn create(state: std::sync::Arc<crate::state::State>) -> anyhow::Result<axum::Router> {
    let span = tracing::info_span!("create_router");
    let _guard = span.enter();

    /* Create the router with the server state */
    let router: axum::Router<std::sync::Arc<crate::state::State>> = axum::Router::new();

    let router = api::register_api(router);
    let router = websocket::register_ws(router);

    /* Other routes are redirected to the static files router */
    let mut router = router;
    for (route_index, static_file) in state.routes().routes.iter().enumerate() {
        for route in static_file.routes() {
            /* Because windows is needs a special attention =) */
            let route = route.replace("\\", "/");
            let handler = axum::routing::get({
                let h = StaticRouteHandler { route_index };
                move |state| h.handle(state)
            });
            tracing::info!("Creating route: {} -> {}", route, static_file.path());
            router = router.route(&route, handler);
        }
    }

    /* Add all redirections (manually created in the create_redirect function) */
    let mut router = router;
    for redirect in redirects::create_redirects() {
        tracing::info!("Creating redirection: {} -> {}", redirect.from, redirect.to);
        let redirect_response = axum::response::Redirect::permanent(&redirect.to);
        router = router.route(&redirect.from, axum::routing::get(redirect_response))
    }

    let router: axum::Router<()> = router.with_state(state);

    Ok(router)
}

pub struct Routes {
    routes: Vec<static_response::StaticFile>,
}

impl Routes {
    pub fn new(serve_path: &std::path::Path) -> anyhow::Result<Routes> {
        let span = tracing::info_span!("create_routes");
        let _guard = span.enter();

        let absolute_serve_path = std::path::absolute(serve_path)?;

        /* Read all the files in the serve directory */
        let mut path_stack = vec![absolute_serve_path.clone()];
        let mut files = Vec::new();

        /* While there are some directories to explore on the dir stack, explore them */
        while let Some(path) = path_stack.pop() {
            /* If the item is a directory, read the entire content and put it on the stack */
            if path.is_dir() {
                let read_dir_iter = match std::fs::read_dir(&path) {
                    Ok(iter) => iter,
                    Err(e) => {
                        tracing::warn!("Failed to read directory at {path:?}: {e}");
                        continue;
                    }
                };
                for entry in read_dir_iter.into_iter() {
                    match entry {
                        Ok(entry) => path_stack.push(entry.path()),
                        Err(e) => tracing::warn!("Failed to read directory entry: {e}"),
                    }
                }
            }
            /* If the item is a file, store it in the files stack */
            else if path.is_file() {
                files.push(path);
            }
            /* Symlinks are not supported yet */
            else if path.is_symlink() {
                tracing::warn!("Symlink found in serve path and will not be explored: {path:?}");
            }
        }

        let routes = files
            .into_iter()
            .filter_map(|file| static_response::StaticFile::new(&absolute_serve_path, &file))
            .collect();

        Ok(Self { routes })
    }
}

#[derive(Clone)]
struct StaticRouteHandler {
    route_index: usize,
}

impl StaticRouteHandler {
    async fn handle(
        self,
        axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::state::State>>,
    ) -> impl axum::response::IntoResponse {
        state.routes().routes[self.route_index].to_response()
    }
}
