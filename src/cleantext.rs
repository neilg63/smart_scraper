use string_patterns::*;

pub fn clean_raw_html(html: &str) -> String {
  let repl_pairs = [
        (r#"\s\s+"#, " "), // remove multiple spaces
        (r#"^\s+"#, ""),// remove all spaces within otherwise empty lines
        (r#"\n"#, ""), // remove remaining new line breaks
        (r#"<\!--.*?-->"#, ""), // comment tags
        (r#"\s+style="[^"]*?""#, ""), // inline styles (often added programmatically)
        (r#"\s+style='[^']*?'"#, ""), // inline styles alternative with single quotes (less common)
    /*     (r#"\s+data(-\w+)+=("[^"]*?"|'[^']*?')"#, ""), // remove data-prefixed attributes that may be used client-side effects
        (r#"\s+data(-\w+)+(\s+|>)"#, "$1"), // remove data-prefixed attributes that may be used client-side effects */
        // (r#">\s*class=[a-z0-9_-]+[^\w]*?<"#, "><"),
    ];
    html.to_owned().pattern_replace_pairs(&repl_pairs, true)
}

pub fn strip_literal_tags(text: &str) -> String {
  text.to_owned().pattern_replace(r"</?\w[^>]*?>", "", true).trim().to_owned()
}