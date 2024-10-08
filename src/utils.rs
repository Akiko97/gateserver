use std::sync::Arc;
use std::time::Duration;
use axum::{
    extract::Request,
    http::StatusCode
};
use tokio::{
    net::TcpStream,
    sync::Mutex,
    select
};
use http_body_util::BodyExt;
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tracing_subscriber::{
    EnvFilter,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    fmt::MakeWriter
};
use rustyline_async::SharedWriter;
use tracing_appender::non_blocking::WorkerGuard;
use crate::config::{ProxyConfig, SERVER_CONFIG};

pub fn banner() {
    println!(r#"
  ________        __           _________
 /  _____/_____ _/  |_  ____  /   _____/ ______________  __ ___________
/   \  ___\__  \\   __\/ __ \ \_____  \_/ __ \_  __ \  \/ // __ \_  __ \
\    \_\  \/ __ \|  | \  ___/ /        \  ___/|  | \/\   /\  ___/|  | \/
 \______  (____  /__|  \___  >_______  /\___  >__|    \_/  \___  >__|
        \/     \/          \/        \/     \/                 \/
    "#);
}

pub fn info() {
    tracing::info!("Author: {}", env!("CARGO_PKG_AUTHORS"));
    tracing::info!("Current version: {}", env!("CARGO_PKG_VERSION"));
    tracing::info!("File log is {}", if SERVER_CONFIG.read().unwrap().server.file_log {
        "enabled"
    } else {
        "disabled"
    });
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        tracing::warn!("RUST_LOG is set to `{}`, but gateserver will not use it. Please use the config file to set the log level.", rust_log);
    }
}

struct GateServerWriter {
    out: Option<SharedWriter>,
}

impl<'a> MakeWriter<'a> for GateServerWriter {
    type Writer = Box<dyn std::io::Write>;

    fn make_writer(&'a self) -> Self::Writer {
        match &self.out {
            None => Box::new(std::io::stdout()),
            Some(out) => Box::new(out.clone()),
        }
    }
}

pub fn init_tracing(out: Option<SharedWriter>) -> WorkerGuard {
    #[cfg(target_os = "windows")]
    ansi_term::enable_ansi_support().unwrap();

    // env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let file_appender = tracing_appender::rolling::daily("logs", "server.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let console_log = tracing_subscriber::fmt::layer()
        .with_writer(GateServerWriter { out })
        .with_target(false)
        .with_thread_names(true);
    let file_log = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_names(true)
        .with_file(true);
    if SERVER_CONFIG.read().unwrap().server.file_log {
        tracing_subscriber::registry()
            .with(console_log)
            .with(file_log)
            .with(EnvFilter::try_from(SERVER_CONFIG.read().unwrap().server.log_level.as_str()).unwrap_or_else(|_| {EnvFilter::from("info")}))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(console_log)
            .with(EnvFilter::try_from(SERVER_CONFIG.read().unwrap().server.log_level.as_str()).unwrap_or_else(|_| {EnvFilter::from("info")}))
            .init();
    }
    guard
}

pub async fn get_body_from_request(mut req: Request) -> Result<Vec<u8>, StatusCode> {
    let mut body_bytes = Vec::new();
    while let Some(Ok(frame)) = req.body_mut().frame().await {
        if let Some(chunk) = frame.data_ref() {
            body_bytes.extend_from_slice(&mut chunk.to_vec());
        } else { return Err(StatusCode::BAD_REQUEST); }
    }
    Ok(body_bytes)
}

pub async fn create_websocket_stream(uri: String) -> Option<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    match connect_async(uri.as_str()).await {
        Ok((stream, _)) => Some(stream),
        Err(err) => {
            tracing::debug!("Creating Websocket connection error: {}", err);
            None
        }
    }
}

pub async fn make_websocket_stream(config: &ProxyConfig) -> Option<Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>> {
    let mut ws_proxy = None;
    for tried_num in 0..3 {
        ws_proxy = select! {
                Some(stream) = create_websocket_stream(config.forward_to.clone()) => {
                    Some(Arc::new(Mutex::new(stream)))
                },
                _ = tokio::time::sleep(Duration::from_millis(2000)) => {
                    tracing::warn!("Failed to connect with Websocket server '{}', remaining attempts: {}",
                        config.forward_to,
                        2 - tried_num);
                    if tried_num < 2 { continue; }
                    tracing::error!("Failed to connect with Websocket server '{}', Websocket proxy not working",
                        config.forward_to);
                    None
                }
            };
        if ws_proxy.is_some() {
            break;
        }
    }
    ws_proxy
}

pub async fn create_tcp_stream(uri: String) -> Option<TcpStream> {
    match TcpStream::connect(uri.as_str()).await {
        Ok(stream) => Some(stream),
        Err(err) => {
            tracing::debug!("Creating TCP connection error: {}", err);
            None
        }
    }
}

pub async fn make_tcp_stream(config: &ProxyConfig) -> Option<Arc<Mutex<TcpStream>>> {
    let mut tcp_proxy = None;
    for tried_num in 0..3 {
        tcp_proxy = select! {
                Some(stream) = create_tcp_stream(config.forward_to.clone()) => {
                    Some(Arc::new(Mutex::new(stream)))
                },
                _ = tokio::time::sleep(Duration::from_millis(2000)) => {
                    tracing::warn!("Failed to connect with TCP server '{}', remaining attempts: {}",
                        config.forward_to,
                        2 - tried_num);
                    if tried_num < 2 { continue; }
                    tracing::error!("Failed to connect with TCP server '{}', TCP proxy not working",
                        config.forward_to);
                    None
                }
            };
        if tcp_proxy.is_some() {
            break;
        }
    }
    tcp_proxy
}

pub fn debug_print_bytes(bytes: &Vec<u8>, source: &str) {
    if let Ok(msg) = String::from_utf8(bytes.clone()) {
        tracing::debug!("Received message from {} ({} bytes): {}",
            source,
            bytes.len(),
            msg);
    } else {
        tracing::debug!("Received message from {}({} bytes): <ERROR IN STRINGIFY>",
            source,
            bytes.len());
    }
}

pub fn wait_file_log_guard(guard: WorkerGuard) {
    tracing::warn!("Fail to create Readline, console log with std::io::Stdout...");
    tokio::spawn(async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to listen for Ctrl+C, log files maybe incomplete: {}", err);
            return;
        }
        tracing::info!("Received Ctrl+C, shutting down...");
        drop(guard);
        tracing::info!("All logs should be flushed now.");
        std::process::exit(0);
    });
}
