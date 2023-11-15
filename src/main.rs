extern crate regex;
extern crate redis;
use reqwest::Client;
use scraper::{Html, Selector, ElementRef};
use html5ever::tree_builder::TreeSink;
use select::document::Document;
use select::node::Node;
use select::predicate::{Attr, Class, Name, Predicate};
use serde::{Deserialize, Serialize};
use serde_json::json;
use string_patterns::*;


mod string_patterns;

fn get_client() -> Client {
    Client::new()
}

const IGNORE_TAGS_FOR_CONTENT: [&'static str; 12] = ["script", "style", "object", "li", "a", "p", "span", "td", "th", "tr", "tbody", "thead"];
// const IGNORE_TAGS: [&'static str; 2] = ["script", "style"];
const MAX_SCAN_DEPTH: usize = 9;
const MIN_MEANINFUL_TEXT_LENGTH: usize = 256;

/* pub fn pattern_replace(text: &str, regex: &str, repl: &str) -> String {
    let re = Regex::new(regex).unwrap();    
    re.replace_all(text, repl).into_owned()
}

pub trait RegexReplace {
    fn pattern_replace(&self, pattern: &str, repl: &str) -> String;
}

impl RegexReplace for String {
    
    fn pattern_replace(&self, pattern: &str, repl: &str) -> String {
        pattern_replace(&self, pattern, repl)
    }
} */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageElement<'a> {
    pub depth: usize,
    #[serde(rename = "tagNames")]
    pub tag_name: &'a str,
    #[serde(rename = "classNames", skip_serializing_if = "Vec::is_empty")]
    pub class_names: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<&'a str>,
    #[serde(rename = "textLen")]
    pub text_len: usize,
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

impl<'a> PageElement<'_> {
    pub fn new(item: &Node<'a>, depth: usize) -> PageElement<'a> {
        let list_links = item.find(Name("li").descendant(Name("a"))).collect::<Vec<_>>().len();

        let num_links = item.find(Name("a")).collect::<Vec<_>>().len();
        let num_paras = item.find(Name("p")).collect::<Vec<_>>().len();
        let num_headings = item.find(Name("h1").or(Name("h2")).or(Name("h3")).or(Name("h4")).or(Name("h5")).or(Name("h6"))).collect::<Vec<_>>().len();
        
        let class_opt = item.attr("class");
        let id_opt = item.attr("id");
        let class_names = if class_opt.is_some() { class_opt.unwrap_or("").split(" ").filter(|s| s.trim().len() > 0).collect::<Vec<&str>>() } else { vec![] };
        
        let repl_pairs = [
            (r"\{.*?\}", ""),
            (r"[\{\}\(\)]+", ""),
            (r"[,;\.-]+", " "),
            (r"\s+", " ")
        ];

        let text_len = &item.text().pattern_replace_pairs(&repl_pairs).trim().len();
        let tag_name: &'a str = item.name().unwrap_or("");
        PageElement { 
            depth,
            tag_name,
            class_names,
            id: id_opt,
            text_len: *text_len,
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
        self.text_len > MIN_MEANINFUL_TEXT_LENGTH && self.fraction > 0.25f64
    }

    pub fn set_fraction(&mut self, total_text_len: usize) {
        self.fraction = self.text_len as f64 / total_text_len as f64;
    }

    pub fn weighted_num_links(&self) -> usize {
        self.list_links * 3 + self.num_links + ((self.num_links * 200) as f64 / self.text_len as f64) as usize
    }

}

#[derive(Debug, Clone, Serialize)]
pub struct PageStats<'a> {
    #[serde(rename="textLen")]
    pub text_len: usize,
    pub lang: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub elements: Vec<PageElement<'a>>,    
}

#[derive(Debug, Clone, Serialize)]
pub struct PageOverview {
    pub lang: Option<String>,
    #[serde(rename = "textLen")]
    pub text_len: usize,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
}

impl<'a> PageStats<'_> {
    pub fn new(doc: &'a Document) -> PageStats<'a>{
        let mut elements: Vec<PageElement> = vec![];
        let title: Option<String> = extract_title_from_doc(doc);
        let lang = extract_lang_from_doc(doc);
        let description: Option<String>  = extract_description_from_doc(doc);
        let image: Option<String>  = extract_image_from_doc(doc);
        let mut text_len = 0;

        if let Some(body) = doc.find(Name("body")).next() {
            text_len = loop_content_tags(&mut elements, &body, 0);
        }
        for element in elements.iter_mut() {
            element.set_fraction(text_len);
        }
        PageStats { 
            text_len,
            lang,
            title,
            description,
            image,
            elements
        }
    }

    pub fn to_overview(&self) -> PageOverview {
        PageOverview {
            text_len: self.text_len,
            lang: self.lang.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            image: self.image.clone(),
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
            Some(text.unwrap_or("").to_owned())
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

pub fn loop_content_tags<'a>(node_items: &mut Vec<PageElement<'a>>, parent: &Node<'a>, depth: usize) -> usize {
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

#[tokio::main]
async fn main() {

    let uri = "https://en.wikipedia.org/wiki/The_Day_the_Music_Died";
    let client = get_client();
    use scraper::Html;
    let result = client.get(uri).send().await;
    //let mut node_items: Vec<PageElement> = vec![];
    if let Ok(req) = result {
        if let Ok(html_raw) = req.text().await {
            let repl_pairs = [
                (r#"<\!--.*?-->"#, ""),
                (r#"\s\s+"#, " "),
                (r#"^\s+"#, ""),
                (r#"\n"#, "")
            ];
            let html = html_raw.pattern_replace_pairs(&repl_pairs);
            let doc = Document::from(html.as_str());

            let mut html_obj = Html::parse_fragment(html.as_str());
           /*  let mut fragment = Html::parse_fragment(&html);
            let selector = Selector::parse("img,style,script").unwrap();
            let elemments = fragment.select(&selector).into_iter().map(|el| el.as()).collect::<Vec<Node>>();
            println!("{}", fragment.html());; */
            let ps = PageStats::new(&doc);
            println!("{}", json!(ps.to_overview()));
            println!("{}", json!(ps.top_text_elements()));
            println!("{}", json!(ps.top_menu_elements()));
            let source_len =  html_obj.html().len();
            
            if let Ok(sel) = Selector::parse("script,style,link") {
                let ids = html_obj.select(&sel).into_iter().map(|el| el.id()).collect::<Vec<_>>();
                for id in ids {
                    html_obj.remove_from_parent(&id);
                }
                let stripped_html = html_obj.html();
                    let stripped_len = stripped_html.len();
                    println!("\nsource: {}, stripped: {}\n", source_len, stripped_len);
                   println!("\n\n{}\n", stripped_html);
           /*      if let Ok(body_sel) = Selector::parse("body") {
                    if let Some(el_ref) = html_obj.select(&body_sel).next() {
                        let text = el_ref.text().into_iter().collect::<String>();
                        
                    }
                    
                } */
                
            }/* 
            if let Ok(sel) = Selector::parse(".elementor-text-editor") {
                println!("\nhtml obj:\n{:?}\n", html_obj.select(&sel).into_iter().collect::<Vec<ElementRef>>());
            } */
            
        } else {
            println!("Cannot extract HTML");
        }
    } else {
        println!("Could not fetch the resource");
    }

    
    
}
