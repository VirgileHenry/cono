/// Represent a single redirection.
///
/// This type is used internally to build the router.
pub struct Redirect {
    pub from: String,
    pub to: String,
}

pub fn create_redirects() -> Vec<Redirect> {
    let span = tracing::info_span!("create_redirects");
    let _guard = span.enter();

    vec![Redirect {
        from: "/index.html".to_string(),
        to: "/".to_string(),
    }]
}
