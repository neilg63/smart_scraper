use serde_json::json;
use axum::{
    response::IntoResponse,
    http::StatusCode,
    extract,
    Json,
};
use crate::browsergrab::capture_from_headless_browser;
use crate::{page_data::*, params::*};
use crate::stats::{extract_base_uri, concat_full_uri};
// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};



const RELATED_SCAN_LIMIT: usize = 64;

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
            let mut counter: usize = 0;
            for dl in page_data_response.domain_links() {
                if counter < RELATED_SCAN_LIMIT {
                  let new_uri = concat_full_uri(&dl, &base_uri);
                  let result_set = fetch_page_data(&new_uri, show_mode, strip_extra, None, false).await;
                  page_data_response.add_related(result_set);
                  counter += 1;
                }
            }
        }
        response = json!(page_data_response);
    }
    (StatusCode::OK, Json(response))
}


pub async fn page_content_response_post(params: extract::Json<PostParams>) -> impl IntoResponse {
  let mut response = json!({
      "valid": false,
  });
  if let Some(uri) = params.uri.clone() {
      let show_links = params.elements.unwrap_or(true);
      let target = params.target.clone();
      

      let show_mode = ShowMode::new(false, show_links);
      let page_data_response = fetch_page_data(&uri, show_mode, true, target, false).await;
      
      response = json!(page_data_response);
  }
  (StatusCode::OK, Json(response))
}


pub async fn page_links_response_post(params: extract::Json<PostParams>) -> impl IntoResponse {
  let mut response = json!({
      "valid": false,
  });
  if let Some(uri) = params.uri.clone() {
    let links = fetch_page_links(&uri).await;
    
    response = json!({ "links": links });
  }
  (StatusCode::OK, Json(response))
}

pub async fn fetch_page_from_browser(params: extract::Json<PostParams>) -> impl IntoResponse {
  let mut response = json!({
      "valid": false,
  });
  if let Some(uri) = params.uri.clone() {
    let output = capture_from_headless_browser(&uri, 5);
    if let Some(pd) = output {
      response = json!({ "valid": true,"content": pd.content, "ts": pd.ts, "cached": pd.cached, "uri": pd.uri });
    }
  }
  (StatusCode::OK, Json(response))
}