mod server_config;

use lazy_static::lazy_static;
pub use server_config::ServerConfig;
pub use server_config::ProxyConfig;

const DEFAULT_CONFIG: &str = include_str!("./server.json");

lazy_static! {
    pub static ref SERVER_CONFIG: ServerConfig = {
        let default = serde_json::from_str(DEFAULT_CONFIG).unwrap();
        load_or_create_config("server.toml", default)
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
