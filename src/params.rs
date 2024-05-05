use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize, Clone)]
pub struct QueryParams {
  pub uri: Option<String>,
  pub full: Option<u8>,
  pub elements: Option<u8>,
  pub target: Option<String>,
}
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostParams {
  pub uri: Option<String>,
  pub full: Option<bool>,
  pub elements: Option<bool>,
  pub links: Option<bool>,
  pub target: Option<String>,
  pub targets: Option<Vec<String>>,
  pub items: Option<Vec<TargetConfig>>,
  pub raw: Option<bool>,
  pub related: Option<bool>,
  pub keep_media: Option<bool>,
  pub skip: Option<bool>
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetConfig {
  pub kind: Option<TargetKind>,
  pub path: Option<String>,
  pub paths: Option<Vec<String>>,
  pub key: Option<String>,
  pub multiple: Option<bool>,
  pub pattern: Option<String>,
  pub plain: Option<bool>,
  pub numeric: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum TargetKind {
  #[serde(rename = "main_text")]
  MainText,
  #[serde(rename = "summaries")]
  Summaries,
  #[serde(rename = "domain_links")]
  DomainLinks,
  #[serde(rename = "external_links")]
  ExternalLinks,
  #[serde(rename = "all_links")]
  AllLinks,
  #[serde(rename = "info")]
  Info,
  #[serde(rename = "data")]
  Data,
  #[serde(rename = "float")]
  Float,
  #[serde(rename = "int")]
  Integer,
  #[serde(rename = "bool")]
  Boolean
}

/* #[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TargetMode {
  FirstMatch,
  MatchToMinMax,
  MatchAll,
  MatchOne,
} */

