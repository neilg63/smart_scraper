
use select::document::Document;
use serde::{Deserialize, Serialize};
use select::predicate::{Attr, Name, Predicate};
use select::node::Node;
use string_patterns::*;

const MIN_MEANINFUL_TEXT_LENGTH: usize = 128;
const MIN_MEANINFUL_TEXT_RATIO: f64 = 0.02;
const MIN_MAIN_TEXT_RATIO: f64 = 0.75;
const IGNORE_TAGS_FOR_CONTENT: [&'static str; 19] = ["script", "style", "object", "li", "a", "p", "span", "td", "th", "tr", "tbody", "thead", "br", "map", "img", "audio", "video", "code", "link"];
// const IGNORE_TAGS: [&'static str; 2] = ["script", "style"];
const MAX_SCAN_DEPTH: usize = 9;

pub fn extract_element_attr(item: &Node, attr_name: &str) -> Option<String> {
  if let Some(attr) = item.attr(attr_name) {
      Some(attr.to_string())
  } else {
      None
  }
}

pub fn extract_href_from_node(item: &Node) -> Option<String> {
  extract_element_attr(item, "href")
}


pub fn extract_title_from_doc<'a>(doc: &'a Document) -> Option<String> {
  if let Some(title_element) = doc.find(Name("title")).next() {
      let title = title_element.text();
      if !title.is_empty() {
          Some(title)
      } else {
          None
      }
  } else {
      None
  }
}

pub fn extract_description_from_doc<'a>(doc: &'a Document) -> Option<String> {
  extract_meta_from_doc(doc, "name", "description")
}

pub fn extract_image_from_doc<'a>(doc: &'a Document) -> Option<String> {
  extract_meta_from_doc(doc, "property", "og:image")
}

pub fn extract_lang_from_doc<'a>(doc: &'a Document) -> Option<String> {
  if let Some(element) = doc.find(Name("html")).next() {
      let text = element.attr("lang");
      if text.is_some() {
          Some(text.unwrap_or("").to_owned().clone())
      } else {
          None
      }
  } else {
      None
  }
}

pub fn extract_meta_from_doc<'a>(doc: &'a Document, ref_field: &str, name: &str) -> Option<String> {
  if let Some(element) = doc.find(Name("meta").and(Attr(ref_field, name))).next() {
      if let Some(text) = element.attr("content") {
          Some(text.to_owned())
      } else {
          None
      }
  } else {
      None
  }
}


pub fn extract_tag_name(item: &Node) -> String {
  item.name().unwrap_or("").to_lowercase()
}


pub fn is_content_element(item: &Node) -> bool {
  let tag_name = extract_tag_name(&item);
  if tag_name.len() > 0 {
      IGNORE_TAGS_FOR_CONTENT.contains(&tag_name.as_str()) == false
  } else {
      false
  }
}

pub fn loop_content_tags(node_items: &mut Vec<PageElement>, parent: &Node, depth: usize) -> usize {
  let mut text_len: usize = 0;
  for item in parent.children() {
      if is_content_element(&item) {
          let new_item = PageElement::new(&item, depth);
          node_items.push(new_item.clone());
          if depth < MAX_SCAN_DEPTH && new_item.has_meaningful_content() {
              loop_content_tags(node_items, &item, depth + 1);
              if depth < 1 {
                  text_len += new_item.text_len;
              }
          }
      }
  }
  text_len
}

pub fn is_local_uri(uri: &str, base_uri: &str) -> bool {
  if uri.starts_with("/") || uri.starts_with("../") {
    true
  } else if uri.starts_with("http://") || uri.starts_with("https://") {
    let mut is_local = uri.starts_with(base_uri);
    let parts = if !is_local {
      base_uri.to_string().to_segments(".")
    } else {
      vec![]
    };
    let num_parts = parts.len();
    
    if num_parts > 1 {
      let last_part = parts.get(num_parts - 1).unwrap();
      let first_part = parts.get(0).unwrap().to_string().to_tail("//");
      let last_is_country_code = last_part.len() == 2;
      // let second_last_part = parts.get(num_parts - 2).unwrap();
      let may_have_subdomains = num_parts > 3 || first_part.as_str() == "www" || (num_parts > 2 && last_is_country_code);
      let separator = if may_have_subdomains {
        "."
      } else {
        "://"
      };
      let base = base_uri.to_owned().to_tail(separator);
      let start_pattern = [r"^https?://([a-z0-9_-]+\.)?",&base].concat();
      is_local = uri.to_owned().pattern_match(&start_pattern, true);
    }
    is_local
  } else {
    false
  }
}

pub fn extract_base_uri(uri: &str) -> String {
  let mut base_uri = uri.to_owned().clone();
  if let Some((head, tail)) = uri.split_once("://") {
    if let Some((domain, _end)) = tail.split_once("/") {
      base_uri = vec![head,"://", domain].concat();
    }
  }
  base_uri
}

pub fn concat_full_uri(uri: &str, base_uri: &str) -> String {
  if uri.starts_with("http://") || uri.starts_with("https://") {
    uri.to_owned()
  } else {
    [base_uri, uri].concat()
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageElement {
    pub depth: usize,
    #[serde(rename = "tagName")]
    pub tag_name: String,
    #[serde(rename = "classNames", skip_serializing_if = "Vec::is_empty")]
    pub class_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "textLen")]
    pub text_len: usize,
    #[serde(rename = "linkTextLen")]
    pub link_text_len: usize,
    #[serde(rename = "listLinks")]
    pub list_links: usize,
    #[serde(rename = "numLinks")]
    pub num_links: usize,
    #[serde(rename = "numParas")]
    pub num_paras: usize,
    #[serde(rename = "numHeadings")]
    pub num_headings: usize,
    pub fraction: f64
}

impl  PageElement {
    pub fn new(item: &Node, depth: usize) -> PageElement {
        let list_links = item.find(Name("li").descendant(Name("a"))).collect::<Vec<_>>().len();
        let link_elems = item.find(Name("a")).collect::<Vec<_>>();
        let num_links = link_elems.len();
        let link_text_len = link_elems.into_iter().map(|el| el.text().len()).fold(0, |a,b| a + b);
        
        let num_paras = item.find(Name("p")).collect::<Vec<_>>().len();
        let num_headings = item.find(Name("h1").or(Name("h2")).or(Name("h3")).or(Name("h4")).or(Name("h5")).or(Name("h6"))).collect::<Vec<_>>().len();
        
        let class_opt = extract_element_attr(item, "class");
        let id_opt = extract_element_attr(item, "id");
        let class_names: Vec<String> = if class_opt.is_some() { class_opt.unwrap_or("".to_owned()).split(" ").filter(|s| s.trim().len() > 0).map(|s| s.to_string()).collect::<Vec<String>>() } else { vec![] };
        
        let repl_pairs = [
            (r"\{.*?\}", ""),
            (r"[\{\}\(\)]+", ""),
            (r"[,;\.-]+", " "),
            (r"\s+", " ")
        ];

        let text_len = &item.text().pattern_replace_pairs(&repl_pairs).trim().len();
        let tag_name: &str = item.name().unwrap_or("");
        PageElement { 
            depth,
            tag_name: tag_name.to_string(),
            class_names,
            id: id_opt,
            text_len: *text_len,
            link_text_len,
            list_links,
            num_links,
            num_paras,
            num_headings,
            fraction: 0f64
        }
    }

    pub fn has_meaningful_content(&self) -> bool {
        self.text_len > MIN_MEANINFUL_TEXT_LENGTH || self.num_links > 0
    }

    pub fn has_meaningful_text(&self) -> bool {
        self.text_len > MIN_MEANINFUL_TEXT_LENGTH && self.fraction > MIN_MEANINFUL_TEXT_RATIO && self.plain_text_ratio() > 0.5
    }

    pub fn is_main_text_element(&self) -> bool {
      self.text_len >= MIN_MEANINFUL_TEXT_LENGTH && self.fraction > MIN_MAIN_TEXT_RATIO
    }

    pub fn set_fraction(&mut self, total_text_len: usize) {
        self.fraction = self.text_len as f64 / total_text_len as f64;
    }

    pub fn weighted_num_links(&self) -> usize {
        self.list_links * 3 + self.num_links + ((self.num_links * 200) as f64 / self.text_len as f64) as usize
    }

    pub fn plain_text_ratio(&self) -> f64 {
      1f64 - (self.link_text_len as f64 / self.text_len as f64)
    }

    pub fn selector(&self) -> String {
        let mut parts = vec![self.tag_name.clone()];
        if let Some(id) = self.id.clone() {
            parts.push("#".to_string());
            parts.push(id);
        }
        if self.class_names.len() > 0 {
            parts.push(".".to_string());
            let cls_string = self.class_names.clone().join(".");
            parts.push(cls_string);
        }
        parts.concat()
    }

}

#[derive(Debug, Clone, Serialize)]
pub struct PageStats {
    uri: String,
    #[serde(rename="textLen")]
    pub text_len: usize,
    pub lang: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub elements: Vec<PageElement>,
    #[serde(rename="domainLinks")]
    pub domain_links: Vec<String>,
    #[serde(rename="numLinks")]
    pub num_links: usize,
    #[serde(rename="numDomainLinks")]
    pub num_domain_links: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageOverview {
    uri: String,
    pub lang: Option<String>,
    #[serde(rename = "textLen")]
    pub text_len: usize,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    #[serde(rename="numLinks")]
    pub num_links: usize,
    #[serde(rename="numDomainLinks")]
    pub num_domain_links: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PageOverviewResult {
    Full(PageStats),
    Basic(PageOverview)
}

impl PageStats {
    pub fn new(doc: &Document, uri: &str, fetch_related_links: bool) -> PageStats {
        let mut elements: Vec<PageElement> = vec![];
        let base_uri = extract_base_uri(uri);
        let title: Option<String> = extract_title_from_doc(doc);
        let lang = extract_lang_from_doc(doc);
        let description: Option<String>  = extract_description_from_doc(doc);
        let image: Option<String>  = extract_image_from_doc(doc);
        let mut text_len = 0;
        let mut domain_links: Vec<String> = vec![];
        let mut num_links: usize = 0;
        let mut num_domain_links: usize = 0;
        if let Some(body) = doc.find(Name("body")).next() {
            text_len = loop_content_tags(&mut elements, &body, 0);
        }
        for element in elements.iter_mut() {
            element.set_fraction(text_len);
        }
        if fetch_related_links {
            for elem in doc.find(Name("a")).into_iter() {
              if let Some(href) = extract_href_from_node(&elem) {
                num_links += 1;
                if is_local_uri(&href, &base_uri) && uri.starts_with('#') == false && uri.len() > 1 {
                  if !domain_links.contains(&href) {
                    domain_links.push(href);
                    num_domain_links += 1;
                  }
                }
              }
          }
        }
        for element in elements.iter_mut() {
            element.set_fraction(text_len);
        }
        elements.sort_by(|a, b| b.text_len.cmp(&a.text_len) );
        PageStats { 
            uri: uri.to_owned(),
            text_len,
            lang,
            title,
            description,
            image,
            elements,
            domain_links,
            num_links,
            num_domain_links,
        }
    }

/*     pub fn empty() -> PageStats {
        PageStats {
            uri: "".to_owned(),
            text_len: 0,
            lang: None,
            title: None,
            description: None,
            image: None,
            elements: vec![],
            domain_links: vec![],
            num_links: 0,
            num_domain_links: 0,
        }
    } */

    pub fn to_overview(&self) -> PageOverview {
        PageOverview {
            uri: self.uri.clone(),
            text_len: self.text_len,
            lang: self.lang.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            image: self.image.clone(),
            num_links: self.num_links,
            num_domain_links: self.num_domain_links,
        }
    }

    pub fn to_result(&self, full: bool) -> PageOverviewResult {
        if full {
            PageOverviewResult::Full(self.to_owned())
        } else {
            PageOverviewResult::Basic(self.to_overview())
        }
    }

    pub fn top_text_elements(&self) -> Vec<PageElement> {
        let mut elements = self.elements.clone().into_iter()
            .filter(|ns| ns.has_meaningful_text()) 
            .collect::<Vec<PageElement>>();
        elements.sort_by(|a, b| b.text_len.cmp(&a.text_len));
        elements
    }

    pub fn top_menu_elements(&self) -> Vec<PageElement> {
        let mut elements = self.elements.clone().into_iter()
            .filter(|ns| ns.text_len >= 16 && ns.list_links > 1)
            .collect::<Vec<PageElement>>();
        elements.sort_by(|a, b| b.weighted_num_links().cmp(&a.weighted_num_links()));
        elements
    }

    pub fn best_content_match(&self) -> Option<PageElement> {
        let mut text_elements = self.elements.clone().into_iter()
            .filter(|ns| ns.is_main_text_element())
            .collect::<Vec<PageElement>>();
        if text_elements.len() > 0 {
            text_elements.sort_by(|a, b| a.text_len.cmp(&b.text_len));
            if let Some(elem) = text_elements.first() {
              if elem.has_meaningful_text() {
                Some(elem.clone())
              } else {
                let mut text_elements = self.elements.clone().into_iter()
                    .filter(|ns| ns.has_meaningful_text())
                    .collect::<Vec<PageElement>>();
                if text_elements.len() > 0 {
                  text_elements.sort_by(|a, b| b.text_len.cmp(&a.text_len));
                  if let Some(elem) = text_elements.first() {
                    Some(elem.clone())
                  } else {
                    None
                  }
                } else {
                  None
                }
              }
            } else {
                None
            }
        } else {
          if let Some(elem) = self.elements.first() {
            Some(elem.clone())
            } else {
                None
            }
        }
    }

}