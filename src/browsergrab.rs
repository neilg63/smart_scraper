
use std::process::Command;
use chrono::Duration;
use crate::{cache::{FlatPage, redis_get_page, redis_set_page}, page_data::{to_page_key, get_max_page_age_minutes, get_headless_browser_app_exec_path}};

pub fn grab_content_from_headless_browser(uri: &str, secs: u16) -> Option<String> {
  let cmd = get_headless_browser_app_exec_path();
  let secs_param = secs.to_string();
  let args = ["-u",
      uri,
      "-s",
      &secs_param];

    let output = Command::new(&cmd)
      .args(&args)
      .output().unwrap_or_else(|e| {
        panic!("failed to execute process: {}", e)
    });
    if output.status.success() {
      if !output.stdout.is_empty() {
        let result = String::from_utf8_lossy(&output.stdout).into_owned();
        return Some(result);
      }
      
    }
    None
}

pub fn capture_from_headless_browser(uri: &str, secs: u16) -> Option<FlatPage> {
  let key = to_page_key(uri);
  if let Some(mut pd) = redis_get_page(&key, Duration::minutes(get_max_page_age_minutes())) {
      pd.set_cached();
      Some(pd)
  } else {
    if let Some(html_raw) = grab_content_from_headless_browser(uri, secs) {
      let pd = FlatPage::new(uri, &html_raw);
      redis_set_page(&key, &pd.uri, &pd.content);
      Some(pd)
    } else {
      None
    }
  }
}