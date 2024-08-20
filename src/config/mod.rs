mod server_config;

use lazy_static::lazy_static;
use std::sync::RwLock;
pub use server_config::ServerConfig;
pub use server_config::ProxyConfig;

const DEFAULT_CONFIG: &str = include_str!("./server.json");
pub const CONFIG_FILE: &str = "server_config.toml";

lazy_static! {
    pub static ref SERVER_CONFIG: RwLock<ServerConfig> = {
        let default = serde_json::from_str(DEFAULT_CONFIG).unwrap();
        RwLock::new(load_or_create_config(CONFIG_FILE, default))
    };
}

fn load_or_create_config(path: &str, default: ServerConfig) -> ServerConfig {
    std::fs::read_to_string(path).map_or_else(
        | _ | {
            std::fs::write(path, toml::to_string(&default).unwrap()).unwrap();
            default
        },
        | data | {
            toml::from_str(&data.as_str()).unwrap()
        }
    )
}

pub fn init_config() {
    let _config = &*SERVER_CONFIG;
}
