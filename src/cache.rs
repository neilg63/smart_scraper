use redis::{Commands, RedisResult, Connection, Client};
use chrono::{Local, Duration};
use serde::{Serialize, Deserialize};

pub fn  redis_client() -> RedisResult<Connection> {
  let client = Client::open("redis://127.0.0.1/")?;
  client.get_connection()
}

pub fn get_timestamp() -> i64 {
  let dt = Local::now();
  dt.timestamp()
}

pub fn seconds_ago(ts: i64) -> i64 {
  let now_ts = get_timestamp();
  now_ts - ts
}

fn redis_get_opt_string(key: &str) -> Option<String> {
  if let Ok(mut connection) =  redis_client() {
      let result: String = connection.get(key.to_owned()).unwrap_or("".to_owned());
      Some(result)
  } else {
      None
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatPage {
  pub uri: String,
  pub content: String,
  pub ts: i64,
  pub cached: bool,
}

impl FlatPage {
  pub fn new(uri: &str, content: &str) -> Self {
    FlatPage { 
      uri: uri.to_string(),
      content: content.to_string(),
      ts: get_timestamp(),
      cached: false
    }
  }

  pub fn empty() -> Self {
    FlatPage { 
      uri: "".to_string(),
      content: "".to_string(),
      ts: 0,
      cached: false
    }
  }

  pub fn is_empty(&self) -> bool {
    self.content.trim().len() < 1
  }

  pub fn retrieved_age(&self) -> i64 {
    let current_ts = get_timestamp();
    current_ts - self.ts
  }

  pub fn set_cached(&mut self) {
    self.cached = true
  }

}

pub fn  redis_set_page(key: &str, uri: &str, content: &str) -> Option<FlatPage> {
  if let Ok(mut connection) =  redis_client() {
      let stored_object = FlatPage::new(uri, content);
      match serde_json::to_string(&stored_object) {
        Ok(value) => match connection.set::<String,String,String>(key.to_string(), value) {
          Ok(_result) => Some(stored_object),
          Err(_error) => None,
        },
        Err(_error) => None,
      }
  } else {
    None
  }
}

pub fn redis_get_page(key: &str, age: Duration) -> Option<FlatPage> {
  if let Some(result) = redis_get_opt_string(key) {
      if result.len() > 0 {
          let mut data: FlatPage = serde_json::from_str(&result).unwrap_or(FlatPage::empty());
          let max_secs = age.num_seconds();
          if data.retrieved_age() < max_secs {
            data.set_cached();
            Some(data)
          } else {
            None
          }
      } else {
          None
      }
  } else {
      None
  }
}