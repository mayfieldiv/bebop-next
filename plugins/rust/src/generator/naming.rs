/// Rust keywords that must be escaped with `r#` prefix.
const RUST_KEYWORDS: &[&str] = &[
  "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
  "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
  "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
  "use", "where", "while", "yield", // Reserved for future use
  "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try", "typeof",
  "unsized", "virtual",
];

/// Escape a Rust keyword with `r#` prefix if necessary.
pub fn escape_keyword(name: &str) -> String {
  if RUST_KEYWORDS.contains(&name) {
    format!("r#{}", name)
  } else {
    name.to_string()
  }
}

/// Convert a name to snake_case (for field names, function names).
///
/// Handles PascalCase, camelCase, and SCREAMING_SNAKE_CASE inputs.
pub fn to_snake_case(name: &str) -> String {
  // TODO: Implement proper case conversion
  // For now, do a basic PascalCase -> snake_case conversion
  let mut result = String::new();
  for (i, ch) in name.chars().enumerate() {
    if ch == '_' {
      result.push('_');
    } else if ch.is_uppercase() {
      if i > 0 && !result.ends_with('_') {
        // Don't insert underscore between consecutive uppercase chars
        let prev_upper = name.chars().nth(i - 1).map_or(false, |c| c.is_uppercase());
        let next_lower = name.chars().nth(i + 1).map_or(false, |c| c.is_lowercase());
        if !prev_upper || next_lower {
          result.push('_');
        }
      }
      result.push(ch.to_ascii_lowercase());
    } else {
      result.push(ch);
    }
  }
  result
}

/// Convert a name to PascalCase (for type names).
pub fn to_pascal_case(name: &str) -> String {
  // TODO: Implement proper case conversion
  // For now, handle snake_case -> PascalCase
  name
    .split('_')
    .filter(|s| !s.is_empty())
    .map(|s| {
      let mut chars = s.chars();
      match chars.next() {
        Some(c) => {
          let mut result = c.to_uppercase().to_string();
          result.extend(chars);
          result
        }
        None => String::new(),
      }
    })
    .collect()
}

/// Escape and convert a field name to idiomatic Rust.
pub fn field_name(name: &str) -> String {
  escape_keyword(&to_snake_case(name))
}

/// Escape and convert a type name to idiomatic Rust.
pub fn type_name(name: &str) -> String {
  escape_keyword(&to_pascal_case(name))
}

/// Extract the simple type name from a fully-qualified name and convert to PascalCase.
///
/// `"bebop.TypeKind"` → `"TypeKind"`, `"bebop.DefinitionDescriptor"` → `"DefinitionDescriptor"`.
pub fn fqn_to_type_name(fqn: &str) -> String {
  let simple = fqn.rsplit('.').next().unwrap_or(fqn);
  type_name(simple)
}
