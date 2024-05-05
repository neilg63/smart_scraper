use string_patterns::*;

pub fn expand_css_path(path: &str) -> String {
  if path.pattern_match_cs(r#"(\w+):?\(\d+\)"#) {
      path.to_string().pattern_replace_cs(r#"(\w+)(:(nth-child)?|\s+)?\((\d+)\)"#, "$1:nth-child($4)")
  } else {
    path.to_string()
  }
}

#[cfg(test)]
mod tests {
  use crate::expand_path::expand_css_path;

  #[test]
  fn test_css_path_rewrite() {
    let source_path = "#oiltb tr:(7) td:nth-child(2)".to_string();
    let target_path = "#oiltb tr:nth-child(7) td:nth-child(2)".to_string();
    let corrected_path = expand_css_path(&source_path);
    assert_eq!(corrected_path, target_path);

    let source_path = "#oiltb tr:(4) td (3)".to_string();
    let target_path = "#oiltb tr:nth-child(4) td:nth-child(3)".to_string();
    let corrected_path = expand_css_path(&source_path);
    assert_eq!(corrected_path, target_path);
  }
}
