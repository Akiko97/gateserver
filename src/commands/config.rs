use crate::config::{SERVER_CONFIG, CONFIG_FILE};
use crate::ServerContext;
use super::ArgSlice;

pub async fn timeout(
    args: ArgSlice<'_>,
    _state: &ServerContext,
) -> Result<String, Box<dyn std::error::Error>> {
    const USAGE: &str = "Usage: config timeout [websocket_proxy|tcp_proxy] [timeout]";

    if args.len() != 2 {
        return Ok(USAGE.to_string());
    }

    let service = args[0];
    let timeout = args[1].parse::<u64>()?;

    match service {
        "websocket_proxy" => {
            if let Some(ref mut config) = SERVER_CONFIG.write().unwrap().websocket_proxy {
                config.timeout = timeout;
                tracing::info!("Timeout for WebSocket proxy had been set to {}", config.timeout);
                Ok(String::from("Successfully updated the timeout config for WebSocket proxy"))
            } else {
                Err("Could not find configuration for websocket_proxy")?
            }
        },
        "tcp_proxy" => {
            if let Some(ref mut config) = SERVER_CONFIG.write().unwrap().tcp_proxy {
                config.timeout = timeout;
                tracing::info!("Timeout for TCP proxy had been set to {}", config.timeout);
                Ok(String::from("Successfully updated the timeout config for TCP proxy"))
            } else {
                Err("Could not find configuration for tcp_proxy")?
            }
        },
        _ => Err("Only `websocket_proxy` and `tcp_proxy` allowed")?,
    }
}

pub async fn save(
    args: ArgSlice<'_>,
    _state: &ServerContext,
) -> Result<String, Box<dyn std::error::Error>> {
    const USAGE: &str = "Usage: config save";

    if args.len() != 0 {
        return Ok(USAGE.to_string());
    }

    match std::fs::write(CONFIG_FILE, toml::to_string(&*SERVER_CONFIG).unwrap()) {
        Ok(_) => {
            tracing::info!("The current configuration has been saved to {CONFIG_FILE}");
            Ok(String::from("Successfully updated the configuration file"))
        },
        Err(err) => Err(err)?,
    }
}

pub async fn show(
    args: ArgSlice<'_>,
    _state: &ServerContext,
) -> Result<String, Box<dyn std::error::Error>> {
    const USAGE: &str = "Usage: config show";

    if args.len() != 0 {
        return Ok(USAGE.to_string());
    }

    let config = {
        let config = SERVER_CONFIG.read().unwrap();
        config.clone()
    };

    match toml::to_string(&config) {
        Ok(result) => Ok(result),
        Err(err) => Err(format!("Could not get current configuration: {}", err))?
    }
}
