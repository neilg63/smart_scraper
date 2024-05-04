use simple_string_patterns::*;

pub trait IsTruthy where Self:SimpleMatch {
  fn is_truthy(&self) -> Option<bool>;

  fn smart_cast_bool(&self, default_value: bool) -> bool {
    self.is_truthy().unwrap_or(default_value)
  }
}
/* 
pub trait IsTruthyCustom where Self:IsTruthy {
  fn is_truthy_custom(&self, true_rules: &[StringBounds], false_rules: &[StringBounds], apply_standard: bool) -> Option<bool>;

  fn smart_cast_bool_custom(&self, true_rules: &[StringBounds], false_rules: &[StringBounds], apply_standard: bool, default_value: bool) -> bool {
    self.is_truthy_custom(true_rules, false_rules, apply_standard).unwrap_or(default_value)
  }
} */

impl IsTruthy for str {
  fn is_truthy(&self) -> Option<bool> {
    let test_str = self.trim().to_lowercase();
    match test_str.as_str() {
      "0" | "-1" | "false" | "no" | "not" | "none" | "n" | "f" | "" => Some(false),
      "1" | "2" | "ok" | "okay" |"y" | "yes" | "true" | "t" => Some(true),
      _ => if test_str.is_numeric() {
        if let Some(fnum) = test_str.to_first_number::<f64>() {
          Some(fnum > 0f64)
        } else {
          None
        }
      } else if test_str.starts_with_ci_alphanum("tru") {
        Some(true)
      } else if test_str.starts_with_ci_alphanum("fals") {
        Some(false)
      } else {
        None
      }
    }
  }
}

/* impl IsTruthyCustom for str {
  fn is_truthy_custom(&self, true_rules: &[StringBounds], false_rules: &[StringBounds], apply_standard: bool) -> Option<bool> {
    let mut truthy: Option<bool> = None;
    if apply_standard {
      if let Some(result) = self.is_truthy() {
        truthy = Some(result);
      }
    }
    if truthy.is_none() {
      if self.match_all_conditional(true_rules) {
        truthy = Some(true);
      } else if self.match_all_conditional(false_rules) {
        truthy = Some(false);
      }
    }
    truthy
  }
} */
