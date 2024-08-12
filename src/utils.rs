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
use crate::config::ProxyConfig;

pub fn banner() {
    println!(r#"
MM'"""""`MM            dP
M' .mmm. `M            88
M  MMMMMMMM .d8888b. d8888P .d8888b.
M  MMM   `M 88'  `88   88   88ooood8
M. `MMM' .M 88.  .88   88   88.  ...
MM.     .MM `88888P8   dP   `88888P'
MMMMMMMMMMM
MP""""""`MM
M  mmmmm..M
M.      `YM .d8888b. 88d888b. dP   .dP .d8888b. 88d888b.
MMMMMMM.  M 88ooood8 88'  `88 88   d8' 88ooood8 88'  `88
M. .MMM'  M 88.  ... 88       88 .88'  88.  ... 88
Mb.     .dM `88888P' dP       8888P'   `88888P' dP
MMMMMMMMMMM
    "#);
    tracing::info!("Author: {}", env!("CARGO_PKG_AUTHORS"));
    tracing::info!("Current version: {}", env!("CARGO_PKG_VERSION"));
}

pub fn init_tracing() {
    #[cfg(target_os = "windows")]
    ansi_term::enable_ansi_support().unwrap();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
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

pub async fn make_websocket_stream(config: &ProxyConfig) -> Option<Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>> {
    let mut ws_proxy = None;
    for tried_num in 0..3 {
        ws_proxy = select! {
                Ok((stream, _)) = connect_async(config.forward_to.as_str()) => {
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

pub async fn make_tcp_stream(config: &ProxyConfig) -> Option<Arc<Mutex<TcpStream>>> {
    let mut tcp_proxy = None;
    for tried_num in 0..3 {
        tcp_proxy = select! {
                Ok(stream) = TcpStream::connect(config.forward_to.as_str()) => {
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
