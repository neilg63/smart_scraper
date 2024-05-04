use chrono::Duration;
use core::time::Duration as StdDuration;
use scraper::{Html, Selector, ElementRef};
use html5ever::tree_builder::TreeSink;
use serde_with::skip_serializing_none;
use crate::cache::{FlatPage, redis_get_page, redis_set_page};
use crate::cleantext::{clean_raw_html, strip_literal_tags};
use crate::expand_path::expand_css_path;
use simple_string_patterns::*;
use string_patterns::*;
use crate::stats::*;
use crate::params::{TargetConfig,TargetKind};
use base64::{Engine as _, engine::general_purpose};
use reqwest::{Client, Error};
use select::document::Document;
use serde::{Deserialize, Serialize};
use serde_json::{json,Value};
use simple_string_patterns::*;
use crate::is_truthy::*;


const MAX_PAGE_AGE_MINS_DEFAUTLT: i64 = 1440;
const HEADLESS_BROWSER_APP_EXEC_PATH_DEFAUTLT: &'static str = "/var/www/mini-puppeteer/scraper";
const MAX_TIMEOUT_SECS: u64 = 15;

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
          tl = txt.to_owned().strip_non_alphanum().len();
      }
      tl
  }).collect::<Vec<usize>>();
  let mut inner_text_len: usize = 0;
  for tl in text_lens {
      inner_text_len += tl;
  }
  inner_text_len
}

pub fn extract_html_as_vec(selector_str: &str, html_obj: &Html) -> Vec<String> {
    let mut elements: Vec<String> = vec![];
    let sel = Selector::parse(selector_str);
    if let Ok(selector) = sel {
        elements = html_obj.select(&selector).into_iter().map(|el| el.html()).collect::<Vec<_>>();
    }
    elements
  }

pub fn extract_best_html(selector_str: &str, html_obj: &Html) -> String {
  let inner = extract_html_as_vec(selector_str, html_obj);
  if inner.len() > 0 {
    inner.join("\n")
  } else {
    "".to_string()
  }
}

pub fn to_page_key(uri: &str) -> String {
  general_purpose::STANDARD_NO_PAD.encode(uri)
}

pub async fn get_page(uri: &str) -> Result<FlatPage, Error> {
  let client = get_client();
  let result = client.get(uri).timeout(StdDuration::from_secs(MAX_TIMEOUT_SECS)).send().await;
  match result {
     Ok(req) => if let Ok(html_raw) = req.text().await {
          Ok(FlatPage::new(uri, &html_raw, false))
      } else {
          Ok(FlatPage::empty())
      },
      Err(error) => Err(error)
  }
}

pub async fn fetch_page(uri: &str, skip_cache: bool) -> Option<FlatPage> {
  let key = to_page_key(uri);
  let age_mins = if skip_cache {
    1
  } else {
    get_max_page_age_minutes()
  };
  if let Some(mut pd) = redis_get_page(&key, Duration::minutes(age_mins)) {
      pd.set_cached();
      Some(pd)
  } else {
      if let Ok(pd ) = get_page(&uri).await {
          redis_set_page(&key, &pd.uri, &pd.content, false);
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
    related: Vec<PageResultSet>,
    valid: bool
}

impl PageResultSet {
    pub fn new(stats: Option<PageOverviewResult>, content: Option<PageInfo>, raw: Option<String>) -> Self {
        PageResultSet {
            stats,
            content,
            raw,
            related: vec![],
            valid: true
        }
    }

    pub fn empty() -> Self {
        PageResultSet {
            stats: None,
            content: None,
            raw: None,
            related: vec![],
            valid: false
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

#[skip_serializing_none]
#[derive(Debug, Serialize, Clone)]
pub struct Snippet {
  pub content: Option<Value>,
  pub key: Option<String>,
  pub kind: Option<TargetKind>,
  pub path: String,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub matches: Vec<Value>,
}

impl Snippet {
    pub fn new(source_text: &str, path: &str) -> Self {
        let content = if source_text.len() > 0 {
            Some(json!(source_text.to_string()))
        } else {
            None
        };
        Snippet {
            content,
            key: None,
            kind: None,
            path: path.to_string(),
            matches: vec![]
        }
    }

    pub fn new_item(source_texts: &[String], path: &str, key_str: &str, multiple: bool, kind: Option<TargetKind>) -> Self {
        
        let data_type = if let Some(tk) = kind {
          match tk {
            TargetKind::Boolean => 1,
            TargetKind::Integer => 2,
            TargetKind::Float => 3,
            _ => 0
          }
        } else {
          0
        };
        let matched_items =  source_texts.to_vec()
          .into_iter().map(|t| match data_type {
              1 => json!(t.is_truthy()),
              2 => if let Some(n) = t.to_first_number::<i64>() {
                json!(n)
              } else {
                json!(t)
              },
              3 => if let Some(n) = t.to_first_number::<f64>() {
                json!(n)
              } else {
                json!(t)
              },
              _ => Value::String(t),
          }).collect::<Vec<Value>>();
        let matches = if multiple {
          matched_items.clone()
        } else {
          vec![]
        };
        let key = if key_str.len() > 0 {
            Some(key_str.to_string())
        } else {
          None
        };
        let content = if !multiple  {
            matched_items.get(0).map(|v| v.to_owned())
        } else {
          None
        };
        Snippet {
          content,
          key,
          kind,
          path: path.to_string(),
          matches
        }
    }

    pub fn has_content(&self) -> bool {
      self.content.is_some() || self.matches.len() > 0
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Clone)]
pub struct ContentResultSet {
    pub stats: Option<PageOverviewResult>,
    contents: Vec<Snippet>,
    cached: bool,
    valid: bool
}

impl ContentResultSet {
    pub fn new(stats: Option<PageOverviewResult>, snippets: Vec<Snippet>, cached: bool) -> Self {
        let valid = snippets.iter().any(|sn| sn.has_content());
        ContentResultSet {
            stats,
            contents: snippets,
            cached,
            valid
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

fn strip_extra_tags(html_obj: &mut Html) {
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

// Build a PageInfo object with the best matched HTML text
pub fn build_page_content_data(uri: &str, html_raw: &str, mode: ShowMode, strip_extra: bool, target: Option<String>, show_raw: bool, cached: bool) -> PageResultSet {
  let has_target = target.is_some();
  let show_elements = mode.show_elements();
  let show_links = mode.show_links();

  let html = clean_raw_html(html_raw);

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

  
  if let Ok(sel) = Selector::parse("script,style,link,noscript") {
      let ids = html_obj.select(&sel).into_iter().map(|el| el.id()).collect::<Vec<_>>();
      for id in ids {
          html_obj.remove_from_parent(&id);
      }
      stripped_html = html_obj.html();
      stripped_len = stripped_html.len();
      if strip_extra {
        strip_extra_tags(&mut html_obj);
      }
      if !has_target {
          compact_html = html_obj.html();
          compact_text_len = extract_inner_text_length(&html_obj.root_element());
      }
  }

  if let Some(tg) = target {
    let (header_target, content_target) = tg.to_head_tail("/");
    let has_header_target = header_target.len() > 1;
    
    best_text = extract_best_html(&content_target, &html_obj);
    if has_header_target {
      let header = extract_best_html(&header_target, &html_obj);
      if header.len() > 1 {
        best_text = [r#"<div class="content-wrapper">"#, &header, &best_text,"</div>"].concat();
      }
    }
    let inner_html_obj = Html::parse_fragment(&best_text);
    compact_text_len = extract_inner_text_length(&inner_html_obj.root_element());
    stripped_len = compact_text_len;
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
  if !has_target {
      if let Some(ps) = p_stats.clone() {
          if let Some(best_text_element) = ps.best_content_match() {
              let str_sel = best_text_element.selector();
              best_text = extract_best_html(&str_sel, &html_obj);
          }
      }
  }
  let compact_text_len = best_text.len();
  let pi = PageInfo::new(source_len, stripped_len, compact_len, cached, &best_text, compact_text_len);
  let raw = if show_raw { Some(html) } else { None };
  let overview = if let Some(ps) = p_stats.clone() {
      Some(ps.to_result(show_links))
  } else {
      None
  };
  PageResultSet::new(overview, Some(pi), raw)
}

// Build content results with multiple targets
pub fn build_page_content_items(uri: &str, html_raw: &str, targets: &[String], items: &[TargetConfig], cached: bool) -> ContentResultSet {
  let num_targets = targets.len();
  let num_items = items.len();
  let has_targets = num_targets > 0;
  let has_items = num_items > 0;

  let html = clean_raw_html(html_raw);
  let source_len = html_raw.len();
  let mut html_obj = Html::parse_fragment(html.as_str());
  let mut stripped_len: usize = 0;
  let mut stripped_html = "".to_string();
  
  
  let mut snippets: Vec<Snippet> = Vec::with_capacity(num_targets + num_items);

  
  if let Ok(sel) = Selector::parse("script,style,link,noscript") {
    let ids = html_obj.select(&sel).into_iter().map(|el| el.id()).collect::<Vec<_>>();
    for id in ids {
      html_obj.remove_from_parent(&id);
    }
    stripped_html = html_obj.html();
    stripped_len = stripped_html.len();
  }

  if has_targets {
    for target in targets {
      let txt = extract_best_html(&target, &html_obj);
      snippets.push(Snippet::new(&txt, &expand_css_path(&target)));
    }
  }

  if has_items {
    let strip_rgx = build_regex(r#"</?\w+[^>]*?>"#, true).unwrap();
    for item in items.to_vec() {
      let key_str = item.key.unwrap_or("".to_string());
      let multiple = item.multiple.unwrap_or(false);
      let kind = item.kind;
      let plain = item.plain.unwrap_or(false);
      let re_opt = if let Some(pat) = item.pattern {
        if let Ok(rgx) = build_regex(&pat, true) {
          Some(rgx)
        } else {
          None
        }
      } else {
        None
      };
      for path in item.paths {
        let txts = extract_html_as_vec(&expand_css_path(&path), &html_obj);
          if txts.len() > 0 {
            if let Some(re) = re_opt.clone() {
              let plain_txts = txts.iter().map(|txt| strip_rgx.replace_all(txt, "").to_string()).collect::<Vec<String>>();
              let mut filtered_txts = vec![];
              let mut index: usize = 0;
              for p_txt in plain_txts  {
                if re.is_match(&p_txt) {
                  let txt_opt = if plain {
                    Some(p_txt)
                  } else {
                    txts.get(index).map(|txt| txt.to_owned())
                  };
                  if let Some(txt) = txt_opt {
                    filtered_txts.push(txt);
                  }
                }
                index += 1;
              }
              if filtered_txts.len() > 0 {
                snippets.push(Snippet::new_item(&filtered_txts, &path, &key_str, multiple, kind));
              }
            } else {
              snippets.push(Snippet::new_item(&txts, &path, &key_str, multiple, kind));
            }
          }
        }
    }
  }
  

  let p_stats = if stripped_html.len() > 0 {
      let doc = Document::from(stripped_html.as_str());
      let ps = PageStats::new(&doc, &uri, false);
      Some(ps)
  } else {
      None
  };
  
  let overview = if let Some(ps) = p_stats.clone() {
      Some(ps.to_result(false))
  } else {
      None
  };
  ContentResultSet::new(overview, snippets, cached)
}

pub async fn fetch_page_data(uri: &str, mode: ShowMode, strip_extra: bool, target: Option<String>, show_raw: bool, skip_cache: bool) -> PageResultSet {
  //let mut node_items: Vec<PageElement> = vec![];
  if let Some(pd) = fetch_page(uri, skip_cache).await {
    build_page_content_data(uri, &pd.content, mode, strip_extra, target, show_raw, pd.cached)
  } else {
    PageResultSet::empty()
  }
}

fn is_javascript_link(title: &str, uri: &str) -> bool {
    let patterns = [r"\{", r"\}"];
    let title_suspect = title.to_owned().pattern_match_all(&patterns, true);
    if !title_suspect {
        uri.to_owned().pattern_match_all(&patterns, true)
    } else {
        title_suspect
    }
}

pub async fn fetch_page_links(uri: &str) -> Vec<LinkItem> {
    let mut links: Vec<LinkItem> = Vec::new();
    //let mut node_items: Vec<PageElement> = vec![];
    if let Some(pd) = fetch_page(uri, false).await {
        let html_raw = pd.content;
        let html = clean_raw_html(&html_raw);
        let base_uri = extract_base_uri(uri);
        let html_obj = Html::parse_fragment(html.as_str());
        let a_selection = Selector::parse("a");
        if let Ok(selector) = a_selection {
            for row in html_obj.select(&selector).into_iter() {
                if let Some(href) = row.attr("href") {
                    let title = row.text().collect::<String>();
                    let title = strip_literal_tags(&title);
                    if title.len() > 0 && !is_javascript_link(&title, href) && uri.starts_with("#") == false {
                        let local =  is_local_uri(href, &base_uri);
                        if links.iter().any(|lk| lk.uri == href) == false {
                            links.push(LinkItem::new(href, &title, "", local))
                        }
                    }
                }
            }
        }
    } 
    links
  }