use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct BaseConfig {
    pub host: String,
    pub port: u32,
}

#[derive(Deserialize, Serialize)]
pub struct WebConfig {
    pub path: String,
    pub dist_path: String,
}

#[derive(Deserialize, Serialize)]
pub struct ProxyConfig {
    pub path: String,
    pub forward_to: String,
}

#[derive(Deserialize, Serialize)]
pub struct ServerConfig {
    pub server: BaseConfig,
    pub web: Option<WebConfig>,
    pub websocket_proxy: Option<ProxyConfig>,
    pub tcp_proxy: Option<ProxyConfig>,
    pub reverse_proxy: Option<ProxyConfig>,
}
