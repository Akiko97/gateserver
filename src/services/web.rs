use std::path::Path;
use std::sync::Arc;
use axum::{
    Router,
    routing::get,
    response::Response,
    http::{StatusCode, Uri},
    extract::State,
    body::Body
};
use tokio::{
    fs::File,
    io::AsyncReadExt
};
use crate::{
    ServerContext,
    config::SERVER_CONFIG
};

const NOT_FOUND: &str = include_str!("./not_found.html");

pub fn setup_routes(router: Router<Arc<ServerContext>>) -> Router<Arc<ServerContext>> {
    if let Some(config) = &SERVER_CONFIG.read().unwrap().web {
        let path = config.path.as_str();
        let get_file_path = if path.ends_with("/") {
            format!("{path}")
        } else {
            format!("{path}/")
        };
        let get_file_path = format!("{get_file_path}*path");
        let get_file_path = get_file_path.as_str();

        tracing::info!("Setting up route for Web service");
        router
            .route(path, get(get_file))
            .route(get_file_path, get(get_file))
    } else {
        router
    }
}

fn is_file_exist(path: &str) -> bool {
    Path::new(path).exists()
}

fn is_virtual_route(path: &str) -> bool {
    !path.contains('.')
}

async fn get_file(
    State(_): State<Arc<ServerContext>>,
    uri: Uri,
) -> Result<Response, StatusCode> {
    let (web_path, dist_path, spa_support) = if let Some(config) = &SERVER_CONFIG.read().unwrap().web {
        (config.path.clone(), config.dist_path.clone(), config.spa_support)
    } else { ("".to_string(), "".to_string(), false) };
    let (web_path, dist_path) = (web_path.as_str(), dist_path.as_str());
    let path = uri.path()
        .trim_start_matches(web_path)
        .trim_start_matches("/")
        .to_string();
    let path_str = path.as_str();
    let file_path = format!("{dist_path}/{path}");
    let file_path = file_path.as_str();
    match file_path {
        // empty, access index page
        "dist/" => serve_file_by_path(format!("{dist_path}/index.html").as_str()).await,
        // file, access an exist file
        _ if is_file_exist(file_path) => serve_file_by_path(file_path).await,
        // not a file, SPA route
        _ if is_virtual_route(path_str) && spa_support => serve_file_by_path(format!("{dist_path}/index.html").as_str()).await,
        // 404
        _ => {
            let message = format!("Not found file {}", file_path);
            tracing::error!(message);
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(NOT_FOUND.replace("%MESSAGE%", message.as_str())))
                .unwrap())
        },
    }
}

async fn serve_file_by_path(file_path: &str) -> Result<Response, StatusCode> {
    let mut file = match File::open(file_path).await {
        Ok(file) => file,
        Err(_) => {
            tracing::error!("Not found file {}", file_path);
            return Err(StatusCode::NOT_FOUND);
        },
    };

    let mut contents = Vec::new();
    if let Err(_) = file.read_to_end(&mut contents).await {
        tracing::error!("Error in reading file {}", file_path);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime_type.as_ref())
        .body(contents.into())
        .unwrap())
}
