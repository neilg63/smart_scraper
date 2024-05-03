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
  pub targets: Option<Vec<TargetConfig>>,
  pub raw: Option<bool>,
  pub related: Option<bool>,
  pub keep_media: Option<bool>,
  pub skip: Option<bool>
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetConfig {
  pub kind: TargetKind,
  pub paths: Vec<String>,
  pub mode: TargetMode,
  pub multiple: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TargetKind {
  MainText,
  Summaries,
  DomainLinks,
  ExternalLinks,
  AllLinks,
  Data
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TargetMode {
  FirstMatch,
  MatchToMinMax,
  MatchAll,
  MatchOne,
}

