use std::time::Duration;
use axum::{
    Router,
    routing::post,
    response::Response,
    http::StatusCode,
    extract::{State, Request}
};
use tokio::select;
use tokio_tungstenite::{tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use crate::{
    ServerContext,
    config::SERVER_CONFIG
};
use crate::utils::get_body_from_request;

pub fn setup_routes(router: Router<ServerContext>) -> Router<ServerContext> {
    if let Some(config) = &SERVER_CONFIG.websocket_proxy {
        let path = config.path.as_str();

        tracing::info!("Setting up route for Websocket proxy service");
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
    if let Some(ws) = context.ws_proxy {
        let mut ws = ws.lock().await;
        // send request to server
        let body_bytes = get_body_from_request(req).await?;
        let request_message = Message::Binary(body_bytes);
        if let Err(_) = ws.send(request_message).await {
            return Err(StatusCode::BAD_GATEWAY);
        }
        // wait for response (timeout: 1s)
        select! {
            Some(result) = ws.next() => {
                match result {
                    Ok(msg) => {
                        match msg {
                            Message::Text(response_text) => {
                                Ok(Response::builder()
                                    .status(StatusCode::OK)
                                    .header("Content-Type", "application/json")
                                    .body(response_text.into())
                                    .unwrap())
                            }
                            Message::Binary(response_binary) => {
                                Ok(Response::builder()
                                    .status(StatusCode::OK)
                                    .header("Content-Type", "application/json")
                                    .body(response_binary.into())
                                    .unwrap())
                            }
                            _ => {
                                tracing::warn!("Received message is an unsupported WebSocket message type");
                                Err(StatusCode::BAD_GATEWAY)
                            }
                        }
                    }
                    Err(err) => {
                        tracing::error!("WebSocket Connection Error: {}", err);
                        Err(StatusCode::BAD_GATEWAY)
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                tracing::warn!("Websocket server timeout");
                Err(StatusCode::GATEWAY_TIMEOUT)
            }
        }
    } else {
        tracing::error!("Access Websocket proxy endpoint without connecting");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
