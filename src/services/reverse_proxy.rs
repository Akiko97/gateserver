use std::sync::Arc;
use axum::{
    Router,
    routing::get,
    extract::{Request, State},
    http::uri::{PathAndQuery, Uri},
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use hyper::StatusCode;
use crate::{
    ServerContext,
    config::SERVER_CONFIG
};

pub fn setup_routes(router: Router<Arc<ServerContext>>) -> Router<Arc<ServerContext>> {
    if let Some(config) = &SERVER_CONFIG.reverse_proxy {
        let path = config.path.as_str();
        let get_file_path = if path.ends_with("/") {
            format!("{path}*path")
        } else {
            format!("{path}/*path")
        };
        let get_file_path = get_file_path.as_str();

        tracing::info!("Setting up route for Reverse proxy service");
        router
            .route(path, get(forward_to))
            .route(get_file_path, get(forward_to))
    } else {
        router
    }
}

fn insert_base_tag(contents: &mut String, base_href: &str) {
    let base_tag = format!("<base href=\"{}\">", base_href);

    // find the position of <head> tag and insert the <base> tag right after it
    if let Some(head_pos) = contents.find("<head>") {
        let insert_pos = head_pos + "<head>".len();
        contents.insert_str(insert_pos, &base_tag);
    } else if let Some(head_pos) = contents.find("<head ") {
        let insert_pos = contents[head_pos..].find('>').unwrap() + head_pos + 1;
        contents.insert_str(insert_pos, &base_tag);
    } else {
        // if no <head> tag is found, add it to the beginning of the document
        contents.insert_str(0, &format!("<head>{}</head>", base_tag));
    }
}

async fn forward_to(
    State(context): State<Arc<ServerContext>>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    if let Some(config) = &SERVER_CONFIG.reverse_proxy {
        // modify req uri
        let path = req.uri().path();
        let path_query = req
            .uri()
            .path_and_query()
            .map_or(path, PathAndQuery::as_str);
        let path_query = path_query
            .trim_start_matches(config.path.as_str());

        let uri = format!("{}{}", config.forward_to, path_query);

        *req.uri_mut() = Uri::try_from(uri).unwrap();

        // get response
        let mut response = context
            .reverse_proxy.as_ref().unwrap()
            .request(req)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        if let Some(content_type) = response.headers().get("content-type") {
            if content_type.to_str().unwrap_or("").contains("text/html") {
                // parse the HTML content
                let mut buf: Vec<u8> = Vec::new();
                while let Some(Ok(frame)) = response.frame().await {
                    if let Some(chunk) = frame.data_ref() {
                        buf.append(&mut chunk.to_vec());
                    } else { return Err(StatusCode::BAD_REQUEST); }
                }
                let contents = std::str::from_utf8(buf.leak()).unwrap_or("");
                let mut contents = contents.to_string();

                insert_base_tag(&mut contents, config.forward_to.as_str());

                let new_body = axum::body::Body::from(contents);

                // build a new response and copy headers
                let mut builder = Response::builder()
                    .status(response.status());

                for (key, value) in response.headers().iter() {
                    // skip the Content-Length header (we have modified length)
                    if key.as_str() != "content-length" {
                        builder = builder.header(key, value.clone());
                    }
                }

                let new_response = builder
                    .body(new_body)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                return Ok(new_response);
            }
        }

        Ok(response.into_response())
    } else {
        tracing::error!("Access reverse proxy endpoint without setting up");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
