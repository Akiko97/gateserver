use std::sync::Arc;
use axum::{
    Router,
    routing::post,
    response::Response,
    http::StatusCode,
    extract::{State, Request}
};
use crate::ServerContext;

pub fn setup_routes(router: Router<Arc<ServerContext>>) -> Router<Arc<ServerContext>> {
    tracing::info!("Setting up route for API service");
    router
        .route("/api", post(test_api))
}

async fn test_api(
    State(_context): State<Arc<ServerContext>>,
    _req: Request,
) -> Result<Response, StatusCode> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body("{}".into())
        .unwrap())
}
