use regex::*;
use std::{iter::Iterator, str::{FromStr, Chars}};

pub enum NumericSeparatorSet {
  Point,
  CommaPoint,
  Comma,
  PointComma,
  CommaSpacePoint,
  PointSpaceComma,
}

pub fn replace_non_ascii_letter<'a>(c: char) ->  Option<Chars<'a>> {
  let sample = match c {
    'å' | 'á' | 'à' | 'ä' | 'ã' | 'â' => "a",
    'é' | 'è' | 'ë' | 'ê' => "e",
    'í' | 'ì' | 'ï' | 'î' => "i",
    'ó' | 'ò' | 'ö' | 'ô' | 'õ' => "o",
    'ú' | 'ù' | 'ü' => "u",
    'ñ' => "n",
    'ß' => "ss",
    '∂' => "d",
    'ϴ' => "th",
    _ => ""
  };
  if sample.len() > 0 {
    Some(sample.chars())
  } else {
    None
  }
}

impl NumericSeparatorSet {
  pub fn decimal(&self) -> Vec<char> {
    match *self {
      NumericSeparatorSet::Comma | NumericSeparatorSet::PointComma | NumericSeparatorSet::PointSpaceComma => vec![','],
      _ => vec!['.', '‧'],
    }
  }
  pub fn group(&self) -> Vec<char> {
    match *self {
      NumericSeparatorSet::PointComma => vec!['.', '‧'],
      NumericSeparatorSet::CommaSpacePoint => vec![',', ' ', '_'],
      NumericSeparatorSet::PointSpaceComma => vec!['.', '‧', '_'],
      NumericSeparatorSet::CommaPoint => vec![','],
      _ => vec![],
    }
  }

  pub fn char_set(&self) -> (Vec<char>, Vec<char>) {
    (self.group(), self.decimal())
  }
}

pub trait PatternMatch {

  fn pattern_match_opt(&self, pattern: &str, case_insensitive: bool) -> Option<bool>;

  fn pattern_match(&self, pattern: &str, case_insensitive: bool) -> bool;

  fn pattern_replace(&self, pattern: &str, replacement: &str, case_insensitive: bool) -> Self where Self:Sized;

  fn pattern_replace_opt(&self, pattern: &str, replacement: &str,case_insensitive: bool) -> Option<Self> where Self:Sized;

  fn strip_non_chars(&self) -> Self where Self:Sized;

}

pub trait CharGroupMatch {
  fn has_digits(&self) -> bool;

  fn has_alphanumeric(&self) -> bool;

  fn has_alphabetic(&self) -> bool;
}

pub trait ExtractSegments {

  fn extract_segments(&self, separator: &str) -> Vec<Self> where Self:Sized;

  fn extract_head(&self, separator: &str) -> Self  where Self:Sized;

  fn extract_segment(&self, separator: &str, index: i32) -> Option<Self>  where Self:Sized;

  fn extract_inner_segment(&self, groups: &[(&str, i32)]) -> Option<Self>  where Self:Sized;

  fn extract_tail(&self, separator: &str) -> Self where Self:Sized;

  fn extract_head_pair(&self, separator: &str) -> (Self, Self)  where Self:Sized;

  fn extract_tail_pair(&self, separator: &str) -> (Self, Self)  where Self:Sized;

  fn extract_numeric_segments(&self, separators: NumericSeparatorSet) -> Vec<Self> where Self:Sized;

}

pub trait ToStrings {
  fn to_strings(&self) -> Vec<String>;
}

impl<T: ToString> ToStrings for Vec<T> {
  fn to_strings(&self) -> Vec<String> {
      self.into_iter().map(|s| s.to_string()).collect()
  }
}

impl<T: ToString> ToStrings for [T] {
  fn to_strings(&self) -> Vec<String> {
      self.into_iter().map(|s| s.to_string()).collect::<Vec<String>>()
  }
}

pub trait ParseVec {
    fn parse_vec<T: FromStr>(&self) -> Vec<T>;
}

//
// 
// Parse of vector of strings to to numbers in combination with 
//
impl ParseVec for Vec<String> {
  fn parse_vec<T: FromStr>(&self) -> Vec<T> {
    self.into_iter().map(|n| n.parse::<T>()).filter(|s| s.is_ok()).map(|n| n.ok().unwrap()).collect::<Vec<T>>()
  }
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

impl CharGroupMatch for String {

  fn has_digits(&self) -> bool {
      self.chars().any(|c| char::is_digit(c, 10))
  }

  fn has_alphanumeric(&self) -> bool {
      self.chars().any(char::is_alphanumeric)
  }

  fn has_alphabetic(&self) -> bool {
    self.chars().any(char::is_alphabetic)
  }
}

impl PatternMatch for Vec<String> {

  ///
  /// Simple regex-compatible match method that will return an optional boolean 
  /// on a vector of strngs. The regex need only be compiled once
  /// - Some(true) means the regex is valid and the string matches
  /// - Some(false) means the regex is valid and the string does not match
  /// - None means the regex is not valid and can this not be evaluated
  /// 
  fn pattern_match_opt(&self, pattern: &str, case_insensitive: bool) -> Option<bool> {
    if let Ok(re) = build_regex(pattern, case_insensitive) {
      let matched = self.into_iter().any(|segment| re.is_match(segment));
      Some(matched)
    } else {
      None
    }
}

  ///
  /// Simpple regex-compatible match method that will return false 
  /// if the pattern does not match the source string or the regex fails
  /// 
  fn pattern_match(&self, pattern: &str, case_insensitive: bool) -> bool {
    self.pattern_match_opt(pattern, case_insensitive).unwrap_or(false)
  }

  ///
  /// Optional regex-enabledd replace method that will return None if the regex fails
  /// 
  fn pattern_replace_opt(&self, pattern: &str, replacement: &str, case_insensitive: bool) -> Option<Vec<String>> {
    if let Ok(re) = build_regex(pattern, case_insensitive) {
      let replacements = self.into_iter().map(|segment| re.replace_all(segment, replacement).to_string()).collect::<Vec<String>>();
      Some(replacements)
    } else {
      None
    }  
  }

  ///
  /// Simple regex-enabledd replace method that will return the same string if the regex fails
  /// 
  fn pattern_replace(&self, pattern: &str, replacement: &str, case_insensitive: bool) -> Vec<String> {
    self.pattern_replace_opt(pattern, replacement, case_insensitive).unwrap_or(self.to_owned())
  }

  fn strip_non_chars(&self) -> Vec<String> {
    self.into_iter().map(|segment| segment.strip_non_chars()).collect()
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

  ///
  /// Replaces multiple sets of patterns with replacements and boolean case sensitivity 
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

impl PatternMatchMany for Vec<String> {

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

  fn pattern_replace_sets(&self, replacement_sets: &[(&str, &str, bool)]) -> Vec<String> {
    let mut return_strings = self.clone();
    for replacement_set in replacement_sets {
      let (pattern, replacement, case_insensitive) = *replacement_set;
      if let Some(new_strings) = return_strings.pattern_replace_opt(pattern, replacement, case_insensitive) {
        return_strings = new_strings;
      }
    }
    return_strings
  }

  fn pattern_replace_pairs(&self, replacement_pairs: &[(&str, &str)]) -> Vec<String> {
    let mut return_strings = self.clone();
    for replacement_pair in replacement_pairs {
      let (pattern, replacement) = *replacement_pair;
      if let Some(new_string) = return_strings.pattern_replace_opt(pattern, replacement, false) {
        return_strings = new_string;
      }
    }
    return_strings
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
    let num_parts = parts.len();
    let target_index = if index >= 0 { index as usize } else { (num_parts as i32 + index) as usize };
    if target_index < num_parts {
      if let Some(segment) = parts.get(target_index) {
        Some(segment.to_owned())
      } else {
        None
      }
    } else {
      None
    }
  }

  fn extract_inner_segment(&self, groups: &[(&str, i32)]) -> Option<String> {
    if groups.len() > 0 {
      let mut matched: Option<String> = None;
      let mut current_string = self.clone();
      for group in groups {
        if current_string.len() > 0 {
          let (separator, index) = group;
          matched = current_string.extract_segment(*separator, *index);
          current_string = matched.clone().unwrap_or("".to_string());
        }
      }
      matched
    } else {
      None
    }
  }

  /// 
  /// Extract a tupe of the head and remainder, like split_once but returns Strings
  fn extract_head_pair(&self, separator: &str) -> (String, String) {
    if let Some((head, tail)) = self.split_once(separator) {
      (head.to_string(), tail.to_string())
    } else {
      ("".to_owned(), self.to_owned())
    }
  }

  /// 
  /// Extract a tupe of the tail and remainder, like split_once in reverse and returning Strings
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

  fn extract_numeric_segments(&self, separators: NumericSeparatorSet) -> Vec<String> {
    let mut segments: Vec<String>  = vec![];
    let (group_separators, decimal_separators) = separators.char_set();
    let mut current_segment = String::new();
    let has_group_separator = group_separators.len() > 0;
    let mut prev_char = ' ';
    for c in self.chars() {
      if c.is_digit(10) {
        if current_segment.is_empty() && prev_char == '.' {
          current_segment.push('0');
          current_segment.push('.');
        }
        current_segment.push(c);
      } else if decimal_separators.contains(&c) {
        current_segment.push('.'); // convert decimal separator to lower dot (period) for compatibility with parse methods
      } else if !has_group_separator || !group_separators.contains(&c)  {
        if current_segment.has_digits() {
          if current_segment.ends_with('.') {
            current_segment.pop();
          }
          if current_segment.starts_with('.') {
            current_segment.insert(0,'0');
          }
          segments.push(current_segment.clone());
        }
        current_segment = String::new();
      }
      prev_char = c;
    }
    if current_segment.has_digits() {
      if current_segment.ends_with('.') {
        current_segment.pop();
      }
      if current_segment.starts_with('.') {
        current_segment.insert(0,'0');
      }
      segments.push(current_segment.clone());
    }
    segments
  }

}

mod tests {
  use crate::string_patterns::*;

  #[test]
  fn test_pattern_match() {
    let text = "The cat caught a mouse in the cattle ranch".to_string();
    let pattern = r"\bmouse\b";
    assert!(text.pattern_match(&pattern, true));
  }

  #[test]
  fn test_pattern_not_match() {
    let text = "The cat caught a mouse in the cattle ranch".to_string();
    let pattern = r"\bzebre\b";
    assert!(text.pattern_match(&pattern, true) == false);
  }

  #[test]
  fn test_pattern_match_many() {
    let texts = ["The cat caught a mouse in the shed.", "The cat caught a bird in the park."].to_strings();
    let pattern = r"\bmouse\b";
    assert!(texts.pattern_match(&pattern, true));
  }
  
  #[test]
  fn test_replace_many() {
    let text = "The cat caught a mouse in the cattle ranch".to_string();
    let target_text = "The lion caught a zebra in the cattle ranch".to_string();
    let sets = vec![(r"\bcat\b", "lion", true), (r"\bmouse\b", "zebra", true)];
    assert_eq!(text.pattern_replace_sets(&sets), target_text);
  }

  #[test]
  fn test_muiltiple_replace_many() {
    let texts = vec![
      "The cat caught a mouse in the cattle ranch".to_string(),
      "The mousetrap did not stop the cat".to_string()
    ];
    let target_texts = vec![
      "The lion caught a zebra in the cattle ranch".to_string(),
      "The mousetrap did not stop the lion".to_string()
    ];
    let sets = vec![(r"\bcat\b", "lion", true), (r"\bmouse\b", "zebra", true)];
    assert_eq!(texts.pattern_replace_sets(&sets), target_texts);
  }

  #[test]
  fn test_tail_pair() {
    let source = "/path/with/many/parts".to_string();
    let result = ("parts".to_string(), "/path/with/many".to_string());
    assert_eq!(source.extract_tail_pair("/"), result);
  }

  #[test]
  fn test_extract_segment() {
    let source = "/path/with/many/segments/and/words".to_string();
    let result = "segments".to_string();
    assert_eq!(source.extract_segment("/", 4).unwrap_or("invalid".to_string()), result);
  }

  #[test]
  fn test_extract_inner_segment() {
    let source = "/path/with/many/segments/image-name.jpg".to_string();
    let result = "image-name".to_string();
    let groups = &[("/", -1), (".", 0)];
    assert_eq!(source.extract_inner_segment(groups).unwrap_or("invalid".to_string()), result);
  }

  #[test]
  fn test_strip_non_chars() {
    let source = "I went to the café for lunch in Zürich. Привет".to_string();
    let result = "IwenttothecaféforlunchinZürichПривет".to_string();
    assert_eq!(source.strip_non_chars(), result);
  }

  #[test]
  fn test_extract_numeric_strings() {
    let source = "I spent £4.80 on 2 ham sandwiches".to_string();
    let result = ["4.80", "2"].to_strings();
    assert_eq!(source.extract_numeric_segments(NumericSeparatorSet::Point), result);
    let source = "In one shop I paid £2.75. Later I saw the same product on sale for £2.00.".to_string();
    let result = ["2.75", "2.00"].to_strings();
    assert_eq!(source.extract_numeric_segments(NumericSeparatorSet::Point), result);
    let source = "Ho pagato 3.299,90€, ma dopo ho visto lo stesso prodotto in vendita per €2.500,00".to_string();
    let result = ["3299.90", "2500.00"].to_strings();
    assert_eq!(source.extract_numeric_segments(NumericSeparatorSet::PointComma), result);
    let source = "A quarter can be represented as .25".to_string();
    let result = ["0.25"].to_strings();
    assert_eq!(source.extract_numeric_segments(NumericSeparatorSet::Point), result);
  }

  #[test]
  fn test_extract_numeric_strings_parse() {
    let source = "In one shop I paid £2.75. Later I saw the same product on sale for £2.00 or .75 of a pound less".to_string();
    let result = vec![2.75f64, 2.0f64, 0.75f64];
    assert_eq!(source.extract_numeric_segments(NumericSeparatorSet::Point).parse_vec::<f64>(), result);
  }

  #[test]
  fn test_parse_vec() {
    let source = ["7.89", "book", "9000"].to_strings();
    let result = vec![7.89f64, 9000f64];
    assert_eq!(source.parse_vec::<f64>(), result);
  }

}

