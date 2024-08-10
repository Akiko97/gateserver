mod utils;
mod config;
mod services;

use std::sync::Arc;
use anyhow::Result;
use tracing::Level;
use axum::{
    Router,
    ServiceExt,
    body::Body,
    extract::Request
};
use tokio::{
    sync::Mutex,
    net::{TcpListener, TcpStream}
};
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
    utils::banner();
    utils::init_tracing();
    config::init_config();
    let span = tracing::span!(Level::DEBUG, "main");
    let _ = span.enter();

    // init server context
    let ws_proxy = if let Some(config) = &SERVER_CONFIG.websocket_proxy {
        if let Ok((stream, _)) = connect_async(config.forward_to.as_str()).await {
            Some(Arc::new(Mutex::new(stream)))
        } else {
            tracing::error!("Failed to connect with websocket server, websocket proxy not working");
            None
        }
    } else { None };
    let tcp_proxy = if let Some(config) = &SERVER_CONFIG.tcp_proxy {
        if let Ok(stream) = TcpStream::connect(config.forward_to.as_str()).await {
            Some(Arc::new(Mutex::new(stream)))
        } else {
            tracing::error!("Failed to connect with tcp server, tcp proxy not working");
            None
        }
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
    let app = create_router();
    let app = app.with_state(state);

    // init server
    let addr = format!("0.0.0.0:{}", SERVER_CONFIG.server.port);
    let server = TcpListener::bind(&addr).await?;
    tracing::info!("Server is listening at {addr}");
    axum::serve(server, ServiceExt::<Request>::into_make_service(app)).await?;

    Ok(())
}

fn create_router() -> Router<ServerContext> {
    let mut router = Router::new();
    // setup all routes
    router = services::websocket_proxy::setup_routes(router);
    router = services::tcp_proxy::setup_routes(router);
    router = services::reverse_proxy::setup_routes(router);
    router = services::api::setup_routes(router);
    router = services::web::setup_routes(router);
    router
}
