extern crate regex;
extern crate redis;

use axum::Router;
use string_patterns::*;
mod string_patterns;
mod cache;
mod stats;

use cache::{FlatPage, redis_get_page, redis_set_page};
use std::net::SocketAddr;
use chrono::Duration as ChronoDuration;
use std::time::Duration;
use reqwest::{Client, Error};
use scraper::{Html, Selector, ElementRef};
use html5ever::tree_builder::TreeSink;
use select::document::Document;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::skip_serializing_none;
use base64::{Engine as _, engine::general_purpose};
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
use stats::*;
// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};


fn get_client() -> Client {
    Client::new()
}

pub fn extract_inner_text_length(elem: &ElementRef) -> usize {
    let text_lens: Vec<usize> = elem.text().into_iter().map(|el| {
        let txt = el.trim();
        let mut tl = txt.len();
        if tl < 16 {
            tl = txt.to_owned().strip_non_chars().len();
        }
        tl
    }).collect::<Vec<usize>>();
    let mut inner_text_len: usize = 0;
    for tl in text_lens {
        inner_text_len += tl;
    }
    inner_text_len
}

pub fn extract_best_html(selector_str: &str, html_obj: &Html) -> String {
    let mut best_text = "".to_string();
    let sel = Selector::parse(selector_str);
    if let Ok(selector) = sel {
        let inner = html_obj.select(&selector).into_iter().map(|el| el.html()).collect::<Vec<_>>();
        best_text = inner.join("\n");
    }
    best_text
}

pub fn to_page_key(uri: &str) -> String {
    general_purpose::STANDARD_NO_PAD.encode(uri)
}

pub async fn get_page(uri: &str) -> Result<FlatPage, Error> {
    let client = get_client();
    let result = client.get(uri).send().await;
    match result {
       Ok(req) => if let Ok(html_raw) = req.text().await {
            Ok(FlatPage::new(uri, &html_raw))
        } else {
            Ok(FlatPage::empty())
        },
        Err(error) => Err(error)
    }
}

pub async fn fetch_page(uri: &str) -> Option<FlatPage> {
    let key = to_page_key(uri);
    if let Some(pd) = redis_get_page(&key, ChronoDuration::hours(3)) {
        Some(pd)
    } else {
        if let Ok(pd ) = get_page(&uri).await {
            redis_set_page(&key, &pd.uri, &pd.content);
            Some(pd)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    #[serde(rename = "sourceHtmlLength")]
    pub source_len: usize,
    #[serde(rename = "strippedHtmlLength")]
    pub stripped_len: usize,
    #[serde(rename = "compactHtmlLength")]
    pub compact_len: usize,
    pub cached: bool,
    #[serde(rename = "bestText", skip_serializing_if = "Option::is_none")]
    pub best_text: Option<String>,
    #[serde(rename = "compactTextLength")]
    pub compact_text_len: usize,
}

impl PageInfo {
    pub fn new(source_len: usize, stripped_len: usize, compact_len: usize, cached: bool, best_text_match: &str, compact_text_len: usize) -> Self {
        let best_text = if best_text_match.len() > 1 {
            Some(best_text_match.to_owned())
        } else {
            None
        };
        PageInfo {
            source_len,
            stripped_len,
            compact_len,
            cached,
            best_text,
            compact_text_len
        }
    }

    pub fn empty() -> Self {
        PageInfo {
            source_len: 0,
            stripped_len: 0,
            compact_len: 0,
            cached: false,
            best_text: None,
            compact_text_len: 0
        }
    }
}

pub async fn fetch_page_stats(uri: &str, show_elements: bool, strip_extra: bool, target: Option<String>, show_raw: bool) -> Option<PageResultSet> {
    // let uri = "https://en.wikipedia.org/wiki/The_Day_the_Music_Died";
    let has_target = target.is_some();
    //let mut node_items: Vec<PageElement> = vec![];
    if let Some(pd) = fetch_page(uri).await {
        let html_raw = pd.content;
        let repl_pairs = [
            (r#"\s\s+"#, " "),
            (r#"^\s+"#, ""),
            (r#"\n"#, ""),
            (r#"<\!--.*?-->"#, ""),
            (r#"\s+style="[^"]*?""#, ""),
            (r#">\s*class=[a-z0-9_-]+[^\w]*?<"#, "><"),
        ];
        let html = html_raw.pattern_replace_pairs(&repl_pairs);
        

        let mut html_obj = Html::parse_fragment(html.as_str());
        /*  let mut fragment = Html::parse_fragment(&html);
        let selector = Selector::parse("img,style,script").unwrap();
        let elemments = fragment.select(&selector).into_iter().map(|el| el.as()).collect::<Vec<Node>>();
        println!("{}", fragment.html());; */
        let mut stripped_len: usize = 0;
        let mut stripped_html = "".to_string();
        let mut compact_html = "".to_string();
        let mut compact_text_len: usize = 0;
        let mut best_text = "".to_string();
        // println!("start post processing");
        if let Some(tg) = target {
            best_text = extract_best_html(tg.as_str(), &html_obj);
            let inner_html_obj = Html::parse_fragment(&best_text);
            compact_text_len = extract_inner_text_length(&inner_html_obj.root_element());
            stripped_len = compact_text_len;
            
        }
        
        if let Ok(sel) = Selector::parse("script,style,link") {
            let ids = html_obj.select(&sel).into_iter().map(|el| el.id()).collect::<Vec<_>>();
            for id in ids {
                html_obj.remove_from_parent(&id);
            }
            stripped_html = html_obj.html();
            stripped_len = stripped_html.len();
            if !has_target && strip_extra {
                if let Ok(sel) = Selector::parse("img,video,audio,object,figure,iframe") {
                    let ids = html_obj.select(&sel).into_iter().map(|el| el.id()).collect::<Vec<_>>();
                    for id in ids {
                        html_obj.remove_from_parent(&id);
                    }
                }
                if let Ok(sel) = Selector::parse("div,span,a") {
                    for elem in html_obj.clone().select(&sel) {
                        let inner_text_len = extract_inner_text_length(&elem);
                        if inner_text_len < 1 {
                            html_obj.remove_from_parent(&elem.id());
                        }
                    }
                
                }
            }
            if !has_target {
                compact_html = html_obj.html();
                compact_text_len = extract_inner_text_length(&html_obj.root_element());
            }
        }
        
        let source_len =  html.len();
        let compact_len = if has_target {
            best_text.len()
        } else {
            compact_html.len()
        };
        let p_stats = if show_elements || !has_target {
            let ref_html = if has_target { stripped_html.as_str() } else { compact_html.as_str() };
            let doc = Document::from(ref_html);
            let ps = PageStats::new(&doc, &uri);
            Some(ps)
        } else {
            None
        };
      /*   println!("{}", json!(ps.to_overview()));
        println!("{}", json!(ps.top_text_elements()));
        println!("{}", json!(ps.top_menu_elements())); */
        //

                
        // println!("\n\n{}\n", compact_html);
        /* 
        if let Ok(sel) = Selector::parse(".elementor-text-editor") {
            println!("\nhtml obj:\n{:?}\n", html_obj.select(&sel).into_iter().collect::<Vec<ElementRef>>());
        } */
        //let top_menu_links = ps.top_menu_elements();
        if !has_target {
            if let Some(ps) = p_stats.clone() {
                if let Some(best_text_element) = ps.best_content_match() {
                    let str_sel = best_text_element.selector();
                    best_text = extract_best_html(&str_sel, &html_obj);
                }
            }
        }
        //println!("end post processing");
        
        let pi = PageInfo::new(source_len, stripped_len, compact_len, pd.cached, &best_text, compact_text_len);
        let raw = if show_raw { Some(html) } else { None };
        let overview = if let Some(ps) = p_stats {
            Some(ps.to_result(show_elements))
        } else {
            None
        };
        Some(PageResultSet::new(overview, Some(pi), raw))
    } else {
        None
    }
}

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
    target: Option<String>,
    raw: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Clone)]
pub struct PageResultSet {
    stats: Option<PageOverviewResult>,
    content: Option<PageInfo>,
    raw: Option<String>,
}

impl PageResultSet {
    pub fn new(stats: Option<PageOverviewResult>, content: Option<PageInfo>, raw: Option<String>) -> Self {
        PageResultSet {
            stats,
            content,
            raw
        }
    }
}

pub async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

pub async fn page_stats(params: extract::Query<QueryParams>) -> impl IntoResponse {
    let mut response = json!({
        "valid": false,
    });
    if let Some(uri) = params.uri.clone() {
        let strip_extra = params.full.unwrap_or(0) < 1;
        let show_elements = params.elements.unwrap_or(1) > 0;
        let target = params.target.clone();
        let page_stats = fetch_page_stats(&uri, show_elements, strip_extra, target, false).await;
        response = json!(page_stats)
    }
    (StatusCode::OK, Json(response))
}

pub async fn page_stats_post(params: extract::Json<PostParams>) -> impl IntoResponse {
    let mut response = json!({
        "valid": false,
    });
    if let Some(uri) = params.uri.clone() {
        let strip_extra = params.full.unwrap_or(false) == false;
        let show_elements = params.elements.unwrap_or(true);
        let target = params.target.clone();
        let show_raw = params.raw.unwrap_or(false);
        let page_stats = fetch_page_stats(&uri, show_elements, strip_extra, target, show_raw).await;
        response = json!(page_stats)
    }
    (StatusCode::OK, Json(response))
}

#[tokio::main]
async fn main() {
   
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/page-stats", get(page_stats).post(page_stats_post))
        .layer(CorsLayer::permissive())
        // timeout requests after 10 secs, returning 408 status code
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
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

