extern crate regex;
extern crate redis;

mod string_patterns;
mod cache;
mod stats;
mod page_data;


use axum::Router;
use page_data::*;
use stats::{extract_base_uri, concat_full_uri};
use std::net::SocketAddr;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::skip_serializing_none;
use axum::{
    response::IntoResponse,
    http::{header, HeaderValue, StatusCode},
    routing::{get, post},
    extract,
    Json,
};
use tower_http::{
    limit::RequestBodyLimitLayer,
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
    timeout::TimeoutLayer,
    cors::CorsLayer
};
// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[skip_serializing_none]
#[derive(Deserialize, Clone)]
pub struct QueryParams {
    uri: Option<String>,
    full: Option<u8>,
    elements: Option<u8>,
    target: Option<String>,
}
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostParams {
    uri: Option<String>,
    full: Option<bool>,
    elements: Option<bool>,
    links: Option<bool>,
    target: Option<String>,
    raw: Option<bool>,
    related: Option<bool>,
}

pub async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

pub async fn page_data_response(params: extract::Query<QueryParams>) -> impl IntoResponse {
    let mut response = json!({
        "valid": false,
    });
    if let Some(uri) = params.uri.clone() {
        let strip_extra = params.full.unwrap_or(0) < 1;
        let show_elements = params.elements.unwrap_or(1) > 0;
        let target = params.target.clone();
        let show_mode = ShowMode::new(show_elements, true);
        let page_data_response = fetch_page_data(&uri, show_mode, strip_extra, target, false).await;
        response = json!(page_data_response)
    }
    (StatusCode::OK, Json(response))
}

pub async fn page_data_response_post(params: extract::Json<PostParams>) -> impl IntoResponse {
    let mut response = json!({
        "valid": false,
    });
    if let Some(uri) = params.uri.clone() {
        let strip_extra = params.full.unwrap_or(false) == false;
        let show_elements = params.elements.unwrap_or(true);
        let show_links = params.elements.unwrap_or(true);
        let target = params.target.clone();
        let show_raw = params.raw.unwrap_or(false);
        let fetch_related = params.related.unwrap_or(false);
        let base_uri = extract_base_uri(&uri);

        let show_mode = ShowMode::new(show_elements, show_links);
        let mut page_data_response = fetch_page_data(&uri, show_mode, strip_extra, target, show_raw).await;
        if fetch_related {
            let show_mode = ShowMode::new(false, false);
            for dl in page_data_response.domain_links() {
                let new_uri = concat_full_uri(&dl, &base_uri);
                let result_set = fetch_page_data(&new_uri, show_mode, strip_extra, None, false).await;
                page_data_response.add_related(result_set);
            }
        }
        response = json!(page_data_response);
    }
    (StatusCode::OK, Json(response))
}

#[tokio::main]
async fn main() {
   
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/page-stats", get(page_data_response).post(page_data_response_post))
        .layer(CorsLayer::permissive())
        // timeout requests after 5 minutes, returning 408 status code
        .layer(TimeoutLayer::new(Duration::from_secs(300)))
        // don't allow request bodies larger than 1024 bytes, returning 413 status code
        .layer(RequestBodyLimitLayer::new(1024))
        .layer(TraceLayer::new_for_http())
        .layer(SetResponseHeaderLayer::if_not_present(
            header::SERVER,
            HeaderValue::from_static("rust-axum"),
        ));
    let app = app.fallback(handler_404);
    let env_port = if let Ok(port_ref) = dotenv::var("PORT") { port_ref } else { "3000".to_owned() };
    let port = if let Ok(p) = u16::from_str_radix(&env_port, 10) { p } else { 3000 };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

