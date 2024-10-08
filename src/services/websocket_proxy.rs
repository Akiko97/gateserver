use std::sync::Arc;
use axum::{
    Router,
    routing::post,
    response::Response,
    http::StatusCode,
    extract::{State, Request}
};
use tokio::{
    select,
    time::Duration,
    net::TcpStream,
    sync::MutexGuard
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use futures_util::{StreamExt, SinkExt};
use crate::{
    ServerContext,
    config::SERVER_CONFIG
};
use crate::utils::{get_body_from_request, debug_print_bytes, create_websocket_stream};

pub fn setup_routes(router: Router<Arc<ServerContext>>) -> Router<Arc<ServerContext>> {
    if let Some(config) = &SERVER_CONFIG.read().unwrap().websocket_proxy {
        let path = config.path.as_str();

        tracing::info!("Setting up route for Websocket proxy service");
        router
            .route(path, post(forward_to))
    } else {
        router
    }
}

async fn forward_to(
    State(context): State<Arc<ServerContext>>,
    req: Request,
) -> Result<Response, StatusCode> {
    let config = {
        let guard = SERVER_CONFIG.read().unwrap();
        guard.websocket_proxy.clone()
    };
    if let (Some(config), Some(ws)) = (config, &context.ws_proxy) {
        let body_bytes = get_body_from_request(req).await?;
        debug_print_bytes(&body_bytes, "HTTP");
        let mut ws = ws.lock().await;
        match handler(&mut ws, body_bytes.clone(), config.timeout).await {
            Ok(response) => Ok(response),
            Err(err) if err == StatusCode::BAD_GATEWAY => {
                tracing::warn!("Failure when connecting to Websocket server, try to reconnect");
                match create_websocket_stream(config.forward_to.clone()).await {
                    Some(new_ws) => {
                        *ws = new_ws;
                        tracing::info!("Reconnected to Websocket server");
                        handler(&mut ws, body_bytes, config.timeout).await
                    },
                    None => {
                        tracing::error!("Failed to reconnect to Websocket server");
                        Err(StatusCode::BAD_GATEWAY)
                    }
                }
            },
            Err(err) => Err(err),
        }
    } else {
        tracing::error!("Access Websocket proxy endpoint without setting up");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn handler(ws: &mut MutexGuard<'_, WebSocketStream<MaybeTlsStream<TcpStream>>>, body_bytes: Vec<u8>, timeout: u64) -> Result<Response, StatusCode> {
    // send request to server
    let request_message = Message::Binary(body_bytes);
    if let Err(err) = ws.send(request_message).await {
        tracing::error!("Sending HTTP request to Websocket proxy error: {}", err);
        return Err(StatusCode::BAD_GATEWAY);
    }
    // wait for response (timeout: 1s)
    select! {
        Some(result) = ws.next() => {
            match result {
                Ok(msg) => {
                    match msg {
                        Message::Text(response_text) => {
                            let response_text_bin = response_text.clone().into_bytes();
                            debug_print_bytes(&response_text_bin, "Websocket");
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header("Content-Type", "application/json")
                                .body(response_text.into())
                                .unwrap())
                        }
                        Message::Binary(response_binary) => {
                            debug_print_bytes(&response_binary, "Websocket");
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
        _ = tokio::time::sleep(Duration::from_millis(timeout)) => {
            tracing::warn!("Websocket server timeout");
            Err(StatusCode::GATEWAY_TIMEOUT)
        }
    }
}
