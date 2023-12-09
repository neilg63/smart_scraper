use chrono::Duration;
use scraper::{Html, Selector, ElementRef};
use html5ever::tree_builder::TreeSink;
use serde_with::skip_serializing_none;
use crate::cache::{FlatPage, redis_get_page, redis_set_page};
use crate::cleantext::clean_raw_html;
use crate::string_patterns::*;
use crate::stats::*;
use base64::{Engine as _, engine::general_purpose};
use reqwest::{Client, Error};
use select::document::Document;
use serde::{Deserialize, Serialize};
use crate::string_patterns::*;

const MAX_PAGE_AGE_MINS_DEFAUTLT: i64 = 1440;
const HEADLESS_BROWSER_APP_EXEC_PATH_DEFAUTLT: &'static str = "/var/www/mini-puppeteer/scraper";

fn get_client() -> Client {
    Client::new()
}

pub fn get_max_page_age_minutes() -> i64 {
  if let Ok(max_mins_str) = dotenv::var("MAX_PAGE_AGE_MINS") {
    if let Ok(max_age) = max_mins_str.parse::<u16>() {
      max_age as i64  
    } else {
      MAX_PAGE_AGE_MINS_DEFAUTLT
    }
  } else {
    MAX_PAGE_AGE_MINS_DEFAUTLT
  }
}

pub fn get_headless_browser_app_exec_path() -> String {
    if let Ok(app_path) = dotenv::var("HEADLESS_BROWSER_APP_EXEC_PATH") {
      app_path
    } else {
      HEADLESS_BROWSER_APP_EXEC_PATH_DEFAUTLT.to_owned()
    }
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
  if let Some(mut pd) = redis_get_page(&key, Duration::minutes(get_max_page_age_minutes())) {
      pd.set_cached();
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

#[skip_serializing_none]
#[derive(Debug, Serialize, Clone)]
pub struct PageResultSet {
    pub stats: Option<PageOverviewResult>,
    content: Option<PageInfo>,
    raw: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    related: Vec<PageResultSet>
}

impl PageResultSet {
    pub fn new(stats: Option<PageOverviewResult>, content: Option<PageInfo>, raw: Option<String>) -> Self {
        PageResultSet {
            stats,
            content,
            raw,
            related: vec![]
        }
    }

    pub fn empty() -> Self {
        PageResultSet {
            stats: None,
            content: None,
            raw: None,
            related: vec![]
        }
    }

    pub fn domain_links(&self) -> Vec<String> {
        if let Some(stats) = self.stats.clone() {
            match stats {
                PageOverviewResult::Full(p_stats) => p_stats.domain_links,
                _ => vec![]
            }
       } else {
        vec![]
       }
    }

    pub fn add_related(&mut self, result_set: PageResultSet) {
        self.related.push(result_set);
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
  }/* 

  pub fn empty() -> Self {
      PageInfo {
          source_len: 0,
          stripped_len: 0,
          compact_len: 0,
          cached: false,
          best_text: None,
          compact_text_len: 0
      }
  } */
}

#[derive(Debug,Copy,Clone)]
pub enum ShowMode {
    ElementsAndLinks,
    LinksOnly,
    ContentOnly
}

impl ShowMode {
    pub fn new(show_elements: bool, show_links: bool) -> ShowMode {
        if show_elements && show_links {
            ShowMode::ElementsAndLinks
        } else if show_links {
            ShowMode::LinksOnly
        } else {
            ShowMode::ContentOnly
        }
    }

    pub fn show_elements(&self) -> bool {
        match self {
            ShowMode::ElementsAndLinks => true,
            _ => false,
        }
    }

    pub fn show_links(&self) -> bool {
        match self {
            ShowMode::ContentOnly => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkItem {
    uri: String,
    title: String,
    summary: String,
    local: bool
}

impl LinkItem {
    pub fn new(uri: &str, title: &str, summary: &str, local: bool) -> LinkItem {
        LinkItem {
            uri: uri.to_owned(),
            title: title.to_owned(),
            summary: summary.to_owned(),
            local
        }
    }
}

pub async fn fetch_page_data(uri: &str, mode: ShowMode, strip_extra: bool, target: Option<String>, show_raw: bool) -> PageResultSet {
  let has_target = target.is_some();
  let show_elements = mode.show_elements();
  let show_links = mode.show_links();
  //let mut node_items: Vec<PageElement> = vec![];
  if let Some(pd) = fetch_page(uri).await {
      let html_raw = pd.content;
      let html = clean_raw_html(&html_raw);
      

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
      
      if let Ok(sel) = Selector::parse("script,style,link,noscript") {
          let ids = html_obj.select(&sel).into_iter().map(|el| el.id()).collect::<Vec<_>>();
          for id in ids {
              html_obj.remove_from_parent(&id);
          }
          stripped_html = html_obj.html();
          stripped_len = stripped_html.len();
          if !has_target && strip_extra {
            if let Ok(sel) = Selector::parse("img,video,audio,object,figure,iframe,svg,path") {
              let ids = html_obj.select(&sel).into_iter().map(|el| el.id()).collect::<Vec<_>>();
              for id in ids {
                  html_obj.remove_from_parent(&id);
              }
            }
            // remove empty tags
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
          let ps = PageStats::new(&doc, &uri, show_links);
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
      let compact_text_len = best_text.len();
      let pi = PageInfo::new(source_len, stripped_len, compact_len, pd.cached, &best_text, compact_text_len);
      let raw = if show_raw { Some(html) } else { None };
      let overview = if let Some(ps) = p_stats.clone() {
          Some(ps.to_result(show_links))
      } else {
          None
      };
      PageResultSet::new(overview, Some(pi), raw)
  } else {
    PageResultSet::empty()
  }
}

fn is_javascript_link(title: &str, uri: &str) -> bool {
    let patterns = [r"\{", r"\}"];
    let title_suspect = title.to_owned().pattern_match_many(&patterns, true);
    if !title_suspect {
        uri.to_owned().pattern_match_many(&patterns, true)
    } else {
        title_suspect
    }
}

pub async fn fetch_page_links(uri: &str) -> Vec<LinkItem> {
    let mut links: Vec<LinkItem> = Vec::new();
    //let mut node_items: Vec<PageElement> = vec![];
    if let Some(pd) = fetch_page(uri).await {
        let html_raw = pd.content;
        let html = clean_raw_html(&html_raw);
        let base_uri = extract_base_uri(uri);
        let html_obj = Html::parse_fragment(html.as_str());
        let a_selection = Selector::parse("a");
        if let Ok(selector) = a_selection {
            for row in html_obj.select(&selector).into_iter() {
                if let Some(href) = row.attr("href") {
                    let title = row.text().collect::<String>();
                    let title = title.trim();
                    if title.len() > 0 && !is_javascript_link(title, href) {
                        let local =  is_local_uri(href, &base_uri);
                        if links.iter().any(|lk| lk.uri == href) == false {
                            links.push(LinkItem::new(href, title, "", local))
                        }
                    }
                }
            }
        }
    } 
    links
  }