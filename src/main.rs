mod utils;
mod config;
mod services;
mod commands;

use std::sync::Arc;
use anyhow::{anyhow, Result};
use tracing::Level;
use axum::{
    Router,
    ServiceExt,
    body::Body,
    extract::Request
};
use tokio::{sync::Mutex, net::{TcpListener, TcpStream}};
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use rustyline_async::Readline;
use crate::config::SERVER_CONFIG;

type HttpClient = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[derive(Clone)]
pub struct ServerContext {
    pub ws_proxy: Option<Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    pub tcp_proxy: Option<Arc<Mutex<TcpStream>>>,
    pub reverse_proxy: Option<HttpClient>,
}

// #[tokio::main(flavor = "multi_thread", worker_threads = 16)]
#[tokio::main]
async fn main() -> Result<()> {
    // show banner
    utils::banner();
    // init tracing and command manager
    let rl = Readline::new(String::from(">> ")).ok();
    let guard = utils::init_tracing(rl.as_ref().map(|(_, out)| out.clone()));
    let mut command_mgr = commands::CommandManager::new();
    if let Some((rl, out)) = rl {
        command_mgr.run(rl, out, guard);
    } else {
        // if create readline failed, use this thread to ensure the completeness of file log
        utils::wait_file_log_guard(guard);
    }
    // show info
    utils::info();
    // init config
    config::init_config();
    // start tracing span
    let span = tracing::span!(Level::DEBUG, "main");
    let _ = span.enter();

    // init server context
    let ws_proxy = if let Some(config) = &SERVER_CONFIG.websocket_proxy {
        utils::make_websocket_stream(config).await
    } else { None };
    let tcp_proxy = if let Some(config) = &SERVER_CONFIG.tcp_proxy {
        utils::make_tcp_stream(config).await
    } else { None };
    let reverse_proxy = if let Some(_) = &SERVER_CONFIG.reverse_proxy {
        let client: HttpClient = hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
                .build(HttpConnector::new());
        Some(client)
    } else { None };
    let state = Arc::new(ServerContext {
        ws_proxy,
        tcp_proxy,
        reverse_proxy,
    });
    // using server context also in command manager
    command_mgr.set_context(state.clone());

    // init app
    let app = create_router(state.clone());
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

fn create_router(context: Arc<ServerContext>) -> Router<Arc<ServerContext>> {
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
