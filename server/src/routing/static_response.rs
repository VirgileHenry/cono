/// Representation of a static file served by the server.
/// Those files are then stored in the routes map to be efficiently sent back to the client.
pub struct StaticFile {
    path: String,
    route: String,
    content: axum::body::Bytes,
    mime: &'static str,
}

impl StaticFile {
    pub fn new(absolute_serve_path: &std::path::Path, file: &std::path::Path) -> Option<Self> {
        let route_path = match file.strip_prefix(absolute_serve_path) {
            Ok(route) => route,
            Err(e) => {
                tracing::warn!("Failed to compute route: {e}");
                return None;
            }
        };
        let route = match route_path.to_str() {
            Some(route) => route.to_string(),
            None => {
                tracing::warn!("Route contains invalid UTF-8 value: {route_path:?}");
                return None;
            }
        };

        let content = match std::fs::read(&file) {
            Ok(content) => axum::body::Bytes::from(content),
            Err(e) => {
                tracing::warn!("Failed to read file content for route {route}: {e}");
                return None;
            }
        };
        let mime = mime_type(&route);

        let route = match route.as_str() {
            "index.html" => "/".to_string(),
            route => {
                /* regex to replace __path__ with {path} */
                match regex::Regex::new("__([a-z]+)__") {
                    Ok(re) => {
                        /* It's getting messy here, but special case for __tag__.html -> {tag} */
                        let route = match route.strip_suffix("__.html") {
                            Some(stripped) => format!("{stripped}__"),
                            None => route.to_string(),
                        };
                        let route = re.replace_all(&route, |cap: &regex::Captures<'_>| {
                            let (_, [element]) = cap.extract();
                            format!("{{{element}}}")
                        });
                        format!("/{route}")
                    }
                    Err(e) => {
                        tracing::warn!("Failed to compile route path replacer regex: {e}");
                        format!("/{route}")
                    }
                }
            }
        };

        Some(Self {
            path: file.to_string_lossy().to_string(),
            route,
            content,
            mime,
        })
    }

    pub fn routes(&self) -> impl Iterator<Item = String> {
        /* Routes to HTML files have different access */
        if let Some(stripped) = self.route.strip_suffix(".html") {
            vec![
                stripped.to_string(),       /* /my/route */
                format!("{stripped}.html"), /* /my/route.html */
            ]
            .into_iter()
        } else {
            vec![self.route.clone()].into_iter()
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn to_response(&self) -> axum::response::Response {
        use axum::response::IntoResponse;
        (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, self.mime)],
            self.content.clone(),
        )
            .into_response()
    }
}

fn mime_type(file_path: &str) -> &'static str {
    match file_path.split('.').last() {
        Some("html") => "text/html",
        Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("txt") => "text/plain",
        Some("json") => "application/json",
        Some("ico") => "image/x-icon",
        Some("svg") => "image/svg+xml",
        Some("jpg") => "image/jpeg",
        Some("jpeg") => "image/jpeg",
        _ => {
            tracing::warn!("Failed to resolve MIME type for {file_path}");
            "application/octet-stream"
        }
    }
}
