mod utils;
mod config;
mod services;

use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Result};
use tracing::Level;
use axum::{
    Router,
    ServiceExt,
    body::Body,
    extract::Request
};
use tokio::{sync::Mutex, net::{TcpListener, TcpStream}, select};
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use crate::config::SERVER_CONFIG;

type HttpClient = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[derive(Clone)]
pub struct ServerContext {
    pub ws_proxy: Option<Arc<Mutex< WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    pub tcp_proxy: Option<Arc<Mutex<TcpStream>>>,
    pub reverse_proxy: Option<HttpClient>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // init
    utils::init_tracing();
    utils::banner();
    config::init_config();
    let span = tracing::span!(Level::DEBUG, "main");
    let _ = span.enter();

    // init server context
    let ws_proxy = if let Some(config) = &SERVER_CONFIG.websocket_proxy {
        let mut ws_proxy = None;
        for tried_num in 0..3 {
            ws_proxy = select! {
                Ok((stream, _)) = connect_async(config.forward_to.as_str()) => {
                    Some(Arc::new(Mutex::new(stream)))
                },
                _ = tokio::time::sleep(Duration::from_millis(2000)) => {
                    tracing::warn!("Failed to connect with Websocket server, remaining attempts: {}", 2 - tried_num);
                    if tried_num < 2 { continue; }
                    tracing::error!("Failed to connect with Websocket server, Websocket proxy not working");
                    None
                }
            };
            if ws_proxy.is_some() {
                break;
            }
        }
        ws_proxy
    } else { None };
    let tcp_proxy = if let Some(config) = &SERVER_CONFIG.tcp_proxy {
        let mut tcp_proxy = None;
        for tried_num in 0..3 {
            tcp_proxy = select! {
                Ok(stream) = TcpStream::connect(config.forward_to.as_str()) => {
                    Some(Arc::new(Mutex::new(stream)))
                },
                _ = tokio::time::sleep(Duration::from_millis(2000)) => {
                    tracing::warn!("Failed to connect with TCP server, remaining attempts: {}", 2 - tried_num);
                    if tried_num < 2 { continue; }
                    tracing::error!("Failed to connect with TCP server, TCP proxy not working");
                    None
                }
            };
            if tcp_proxy.is_some() {
                break;
            }
        }
        tcp_proxy
    } else { None };
    let reverse_proxy = if let Some(_) = &SERVER_CONFIG.reverse_proxy {
        let client: HttpClient = hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
                .build(HttpConnector::new());
        Some(client)
    } else { None };
    let state = ServerContext {
        ws_proxy,
        tcp_proxy,
        reverse_proxy,
    };

    // init app
    let app = create_router(&state);
    let app = app.with_state(state);

    // init server
    let addr = format!("0.0.0.0:{}", SERVER_CONFIG.server.port);
    let server = match TcpListener::bind(&addr).await {
        Ok(server) => server,
        Err(err) => {
            let error_msg = format!("Failed to bind TCP listener: {}", err);
            tracing::error!("{error_msg}");
            return Err(anyhow!("{error_msg}"));
        }
    };

    tracing::info!("Server is listening at {addr}");
    axum::serve(server, ServiceExt::<Request>::into_make_service(app)).await?;

    Ok(())
}

fn create_router(context: &ServerContext) -> Router<ServerContext> {
    let mut router = Router::new();
    // setup all routes
    if context.ws_proxy.is_some() {
        router = services::websocket_proxy::setup_routes(router);
    }
    if context.tcp_proxy.is_some() {
        router = services::tcp_proxy::setup_routes(router);
    }
    if context.reverse_proxy.is_some() {
        router = services::reverse_proxy::setup_routes(router);
    }
    router = services::api::setup_routes(router);
    if SERVER_CONFIG.web.is_some() {
        router = services::web::setup_routes(router);
    }
    services::default::setup_routes(router)
}
