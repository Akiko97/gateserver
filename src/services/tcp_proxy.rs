use std::time::Duration;
use axum::{
    Router,
    routing::post,
    response::Response,
    http::StatusCode,
    extract::{State, Request}
};
use tokio::select;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::{
    ServerContext,
    config::SERVER_CONFIG
};
use crate::utils::get_body_from_request;

pub fn setup_routes(router: Router<ServerContext>) -> Router<ServerContext> {
    if let Some(config) = &SERVER_CONFIG.tcp_proxy {
        let path = config.path.as_str();

        tracing::info!("Setting up route for TCP proxy service");
        router
            .route(path, post(forward_to))
    } else {
        router
    }
}

async fn forward_to(
    State(context): State<ServerContext>,
    req: Request,
) -> Result<Response, StatusCode> {
    if let Some(tcp) = context.tcp_proxy {
        let mut tcp = tcp.lock().await;
        // send request to server
        let body_bytes = get_body_from_request(req).await?;
        if let Err(_) = tcp.write_all(body_bytes.as_slice()).await {
            return Err(StatusCode::BAD_GATEWAY);
        }
        // wait for response (timeout: 1s)
        let mut buffer = vec![0; 1024];
        select! {
            result = tcp.read(&mut buffer) => {
                match result {
                    Ok(n) if n > 0 => {
                        let msg = &buffer[..n];
                        let msg = msg.to_vec();
                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "application/json")
                            .body(msg.into())
                            .unwrap())
                    }
                    Ok(n) => {
                        tracing::error!("TCP Connection Error: received message include {} byte(s)", n);
                        Err(StatusCode::BAD_GATEWAY)
                    }
                    Err(err) => {
                        tracing::error!("TCP Connection Error: {}", err);
                        Err(StatusCode::BAD_GATEWAY)
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                tracing::warn!("TCP server timeout");
                Err(StatusCode::GATEWAY_TIMEOUT)
            }
        }
    } else {
        tracing::error!("Access TCP proxy endpoint without connecting");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
