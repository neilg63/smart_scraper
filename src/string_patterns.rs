use regex::*;

// pub struct ReplacementSet<'a>( &'a str, &'a str, bool);

pub trait PatternMatch {

  fn pattern_match_opt(&self, pattern: &str, case_insensitive: bool) -> Option<bool>;

  fn pattern_match(&self, pattern: &str, case_insensitive: bool) -> bool;

  fn pattern_replace(&self, pattern: &str, replacement: &str, case_insensitive: bool) -> Self where Self:Sized;

  fn pattern_replace_opt(&self, pattern: &str, replacement: &str,case_insensitive: bool) -> Option<Self> where Self:Sized;

  fn strip_non_chars(&self) -> Self where Self:Sized;
}

pub trait ExtractSegments {

  fn extract_segments(&self, separator: &str) -> Vec<Self> where Self:Sized;

  fn extract_head(&self, separator: &str) -> Self  where Self:Sized;

  fn extract_segment(&self, separator: &str, index: i32) -> Option<Self>  where Self:Sized;

  fn extract_tail(&self, separator: &str) -> Self where Self:Sized;

  fn extract_head_pair(&self, separator: &str) -> (Self, Self)  where Self:Sized;

  fn extract_tail_pair(&self, separator: &str) -> (Self, Self)  where Self:Sized;

}

pub trait PatternMatchMany {
  fn pattern_match_many(&self, patterns: &[&str], case_insensitive: bool) -> bool;
  fn pattern_match_many_insensitive(&self, patterns: &[&str]) -> bool;
  fn pattern_match_many_sensitive(&self, patterns: &[&str]) -> bool;
  fn pattern_match_many_mixed(&self, pattern_sets: &[(&str, bool)]) -> bool;
  ///
  /// string matches all conditional patterns which may be positive / negative and case insensitive or not
  /// 
  fn pattern_match_many_conditional(&self, pattern_sets: &[(bool, &str, bool)]) -> bool;
  fn pattern_replace_pairs(&self, replacement_sets: &[(&str, &str)]) -> Self where Self: Sized;
  fn pattern_replace_sets(&self, replacement_sets: &[(&str, &str, bool)]) -> Self where Self: Sized;
}

fn build_regex(pattern: &str, case_insensitive: bool) -> Result<Regex, Error> {
  let mut parts: Vec<&str> = vec![];
  if case_insensitive {
    parts.push("(?i)");
  }
  parts.push(pattern);
  let regex_str = parts. concat();
  Regex::new(&regex_str)
}

impl PatternMatch for String {

  ///
  /// Simple regex-compatible match method that will return an optional boolean 
  /// - Some(true) means the regex is valid and the string matches
  /// - Some(false) means the regex is valid and the string does not match
  /// - None means the regex is not valid and can this not be evaluated
  /// 
  fn pattern_match_opt(&self, pattern: &str, case_insensitive: bool) -> Option<bool> {
    if let Ok(re) = build_regex(pattern, case_insensitive) {
      Some(re.is_match(self))
    } else {
      None
    }
}

  ///
  /// Simpple regex-compatible match method that will return false 
  /// if the pattern does not match the source string or the regex fails
  /// 
  fn pattern_match(&self, pattern: &str, case_insensitive: bool) -> bool {
      if let Ok(re) = build_regex(pattern, case_insensitive) {
        re.is_match(self)
      } else {
        false
      }
  }

  ///
  /// Optional regex-enabledd replace method that will return None if the regex fails
  /// 
  fn pattern_replace_opt(&self, pattern: &str, replacement: &str, case_insensitive: bool) -> Option<String> {
    if let Ok(re) = build_regex(pattern, case_insensitive) {
      Some(re.replace_all(self, replacement).to_string())
    } else {
      None
    }  
  }

  ///
  /// Simple regex-enabledd replace method that will return the same string if the regex fails
  /// 
  fn pattern_replace(&self, pattern: &str, replacement: &str, case_insensitive: bool) -> String {
    self.pattern_replace_opt(pattern, replacement, case_insensitive).unwrap_or(self.to_owned())
  }

  fn strip_non_chars(&self) -> String {
    self.chars().into_iter().filter(|c| c.is_alphanumeric()).collect::<String>()
  }

}

impl PatternMatchMany for String {

  fn pattern_match_many(&self, patterns: &[&str], case_insensitive: bool) -> bool {
    let mut num_matched = 0usize;
    let num_patterns = patterns.len();
    for pattern in patterns {
      if self.pattern_match(pattern, case_insensitive) {
        num_matched += 1;
      }
    }
    num_matched == num_patterns
  }
  fn pattern_match_many_insensitive(&self, patterns: &[&str]) -> bool {
    self.pattern_match_many(patterns, true)
  }

  fn pattern_match_many_sensitive(&self, patterns: &[&str]) -> bool {
    self.pattern_match_many(patterns, false)
  }

  fn pattern_match_many_mixed(&self, pattern_sets: &[(&str, bool)]) -> bool {
    let mut num_matched = 0usize;
    let num_patterns = pattern_sets.len();
    for pair in pattern_sets {
      let (pattern, case_insensitive) = *pair;
      if self.pattern_match(pattern, case_insensitive) {
        num_matched += 1;
      }
    }
    num_matched == num_patterns
  }

  fn pattern_match_many_conditional(&self, pattern_sets: &[(bool, &str, bool)]) -> bool {
    let mut num_matched = 0usize;
    let num_patterns = pattern_sets.len();
    for pattern_set in pattern_sets {
      let (is_positive, pattern, case_insensitive) = *pattern_set;
      let is_matched = self.pattern_match(pattern, case_insensitive);
      if is_matched == is_positive {
        num_matched += 1;
      }
    }
    num_matched == num_patterns
  }

  fn pattern_replace_sets(&self, replacement_sets: &[(&str, &str, bool)]) -> String {
    let mut return_string = self.clone();
    for replacement_set in replacement_sets {
      let (pattern, replacement, case_insensitive) = *replacement_set;
      if let Some(new_string) = return_string.pattern_replace_opt(pattern, replacement, case_insensitive) {
        return_string = new_string;
      }
    }
    return_string
  }

  fn pattern_replace_pairs(&self, replacement_pairs: &[(&str, &str)]) -> String {
    let mut return_string = self.clone();
    for replacement_pair in replacement_pairs {
      let (pattern, replacement) = *replacement_pair;
      if let Some(new_string) = return_string.pattern_replace_opt(pattern, replacement, false) {
        return_string = new_string;
      }
    }
    return_string
  }
}



impl ExtractSegments for String {
  fn extract_segments(&self, separator: &str) -> Vec<String> {
    let splitter = self.split(separator);
    splitter.into_iter().map(|s| s.to_string()).collect::<Vec<String>>()
  }

  fn extract_head(&self, separator: &str) -> String {
    if let Some((head, _tail)) = self.split_once(separator) {
      head.to_string()
    } else {
      self.to_owned()
    }
  }

  fn extract_tail(&self, separator: &str) -> String {
    let parts = self.extract_segments(separator);
    if parts.len() > 0 {
      parts.last().unwrap_or(self).to_owned()
    } else {
      self.to_owned()
    }
  }

  fn extract_segment(&self, separator: &str, index: i32) -> Option<String> {
    let parts = self.extract_segments(separator);
    let target_index = if index >= 0 { index as usize } else { (0 - index) as usize };
    if target_index < parts.len() {
      if let Some(segment) = parts.get(target_index) {
        Some(segment.to_owned())
      } else {
        None
      }
    } else {
      None
    }
  }

  fn extract_head_pair(&self, separator: &str) -> (String, String) {
    if let Some((head, tail)) = self.split_once(separator) {
      (head.to_string(), tail.to_string())
    } else {
      ("".to_owned(), self.to_owned())
    }
  }

  fn extract_tail_pair(&self, separator: &str) -> (String, String) {
    let parts = self.extract_segments(separator);
    let mut head = "".to_string();
    if parts.len() > 0 {
      let tail = parts.last().unwrap_or(self).to_owned();
      let num_parts = parts.len();
      if num_parts > 1 {
        let mut head_parts: Vec<&str> = vec![];
        let head_end = num_parts - 1;
        for i in 0..head_end {
          if let Some(part) = parts.get(i) {
            head_parts.push(part);
          }
        }
        head = head_parts.join(separator);
      }
      (tail, head)
    } else {
      (self.to_owned(), head)
    }
  }
}