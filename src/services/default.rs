use axum::{
    http::{StatusCode, Uri},
    response::IntoResponse,
    response::Response,
    body::Body,
    Router
};
use crate::ServerContext;

pub fn setup_routes(router: Router<ServerContext>) -> Router<ServerContext> {
    router.fallback(handle_default)
}

async fn handle_default(uri: Uri) -> impl IntoResponse {
    let message = format!("Path '{}' is not set up", uri.path());
    tracing::warn!("{}", message);
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(message))
        .unwrap()
}
