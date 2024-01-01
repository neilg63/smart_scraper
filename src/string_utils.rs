use string_patterns::*;

/* pub enum NumericSeparatorSet {
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
} */

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
      if let Ok(new_string) = return_string.pattern_replace_result(pattern, replacement, case_insensitive) {
        return_string = new_string;
      }
    }
    return_string
  }

  fn pattern_replace_pairs(&self, replacement_pairs: &[(&str, &str)]) -> String {
    let mut return_string = self.clone();
    for replacement_pair in replacement_pairs {
      let (pattern, replacement) = *replacement_pair;
      if let Ok(new_string) = return_string.pattern_replace_result(pattern, replacement, false) {
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
      if let Ok(new_strings) = return_strings.pattern_replace_result(pattern, replacement, case_insensitive) {
        return_strings = new_strings;
      }
    }
    return_strings
  }

  fn pattern_replace_pairs(&self, replacement_pairs: &[(&str, &str)]) -> Vec<String> {
    let mut return_strings = self.clone();
    for replacement_pair in replacement_pairs {
      let (pattern, replacement) = *replacement_pair;
      if let Ok(new_string) = return_strings.pattern_replace_result(pattern, replacement, false) {
        return_strings = new_string;
      }
    }
    return_strings
  }
}

mod tests {
  use super::*;

  
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

}

