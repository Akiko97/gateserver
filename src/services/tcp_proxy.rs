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
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::MutexGuard
};
use crate::{
    ServerContext,
    config::SERVER_CONFIG
};
use crate::utils::{get_body_from_request, debug_print_bytes, create_tcp_stream};

pub fn setup_routes(router: Router<Arc<ServerContext>>) -> Router<Arc<ServerContext>> {
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
    State(context): State<Arc<ServerContext>>,
    req: Request,
) -> Result<Response, StatusCode> {
    if let (Some(config), Some(tcp)) = (&SERVER_CONFIG.tcp_proxy, &context.tcp_proxy) {
        let body_bytes = get_body_from_request(req).await?;
        debug_print_bytes(&body_bytes, "HTTP");
        let mut tcp = tcp.lock().await;
        match handler(&mut tcp, body_bytes.clone(), config.timeout).await {
            Ok(response) => Ok(response),
            Err(err) if err == StatusCode::BAD_GATEWAY => {
                tracing::warn!("Failure when connecting to TCP server, try to reconnect");
                match create_tcp_stream(config.forward_to.clone()).await {
                    Some(new_tcp) => {
                        *tcp = new_tcp;
                        tracing::info!("Reconnected to TCP server");
                        handler(&mut tcp, body_bytes, config.timeout).await
                    }
                    None => {
                        tracing::error!("Failed to reconnect to TCP server");
                        Err(StatusCode::BAD_GATEWAY)
                    }
                }
            },
            Err(err) => Err(err),
        }
    } else {
        tracing::error!("Access TCP proxy endpoint without setting up");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn handler(tcp: &mut MutexGuard<'_, TcpStream>, body_bytes: Vec<u8>, timeout: u64) -> Result<Response, StatusCode> {
    // send request to server
    if let Err(err) = tcp.write_all(body_bytes.as_slice()).await {
        tracing::error!("Sending HTTP request to TCP server error: {}", err);
        return Err(StatusCode::BAD_GATEWAY);
    }
    // wait for response (timeout: 1s)
    let mut buffer = vec![0; 4096];
    select! {
        result = tcp.read(&mut buffer) => {
            match result {
                Ok(n) if n > 0 => {
                    let msg = &buffer[..n];
                    let msg = msg.to_vec();
                    debug_print_bytes(&msg, "TCP");
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
        _ = tokio::time::sleep(Duration::from_millis(timeout)) => {
            tracing::warn!("TCP server timeout");
            Err(StatusCode::GATEWAY_TIMEOUT)
        }
    }
}
