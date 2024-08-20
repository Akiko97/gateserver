use crate::config::SERVER_CONFIG;
use crate::ServerContext;
use super::ArgSlice;
use crate::utils::{create_websocket_stream, create_tcp_stream};

pub async fn reconnect(
    args: ArgSlice<'_>,
    state: &ServerContext,
) -> Result<String, Box<dyn std::error::Error>> {
    const USAGE: &str = "Usage: net reconnect [websocket_proxy|tcp_proxy]";

    if args.len() != 1 {
        return Ok(USAGE.to_string());
    }

    let service = args[0];

    match service {
        "websocket_proxy" => {
            let config = {
                let guard = SERVER_CONFIG.read().unwrap();
                guard.websocket_proxy.clone()
            };
            if let (Some(config), Some(ws)) = (config, &state.ws_proxy) {
                match create_websocket_stream(config.forward_to.clone()).await {
                    Some(new_ws) => {
                        let mut ws = ws.lock().await;
                        *ws = new_ws;
                        tracing::info!("Reconnected to Websocket server");
                        Ok(String::from("Successfully reconnected to Websocket server"))
                    },
                    None => {
                        Err("Failed to reconnect to Websocket server")?
                    }
                }
            } else {
                Err("Could not find configuration or connection for websocket_proxy")?
            }
        },
        "tcp_proxy" => {
            let config = {
                let guard = SERVER_CONFIG.read().unwrap();
                guard.tcp_proxy.clone()
            };
            if let (Some(ref config), Some(tcp)) = (config, &state.tcp_proxy) {
                match create_tcp_stream(config.forward_to.clone()).await {
                    Some(new_tcp) => {
                        let mut tcp = tcp.lock().await;
                        *tcp = new_tcp;
                        tracing::info!("Reconnected to TCP server");
                        Ok(String::from("Successfully reconnected to TCP server"))
                    },
                    None => {
                        Err("Failed to reconnect to TCP server")?
                    }
                }
            } else {
                Err("Could not find configuration or connection for tcp_proxy")?
            }
        },
        _ => Err("Only `websocket_proxy` and `tcp_proxy` allowed")?,
    }
}
