/// Identifiers that cannot be used as raw identifiers (`r#name` is rejected by rustc).
/// These need suffix mangling (`name_`) instead of `r#` escaping.
///
/// Source: `Symbol::can_be_raw()` + `Symbol::is_path_segment_keyword()` in
/// `compiler/rustc_span/src/symbol.rs`.
const RAW_REJECTED: &[&str] = &["self", "Self", "super", "crate"];

/// Rust keywords that must be escaped with `r#` prefix.
/// Excludes identifiers in `RAW_REJECTED` which need suffix mangling instead.
///
/// Source: `Keywords` table in `compiler/rustc_span/src/symbol.rs` —
/// `is_used_keyword_always`, `is_unused_keyword_always`, and
/// `is_used_keyword_conditional` / `is_unused_keyword_conditional` sections.
#[rustfmt::skip]
const RUST_KEYWORDS: &[&str] = &[
  // Used keywords (stable)
  "as", "break", "const", "continue", "else", "enum", "extern", "false", "fn", "for", "if", "impl",
  "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static", "struct",
  "trait", "true", "type", "unsafe", "use", "where", "while",
  // Unused keywords (reserved for future use)
  "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized",
  "virtual", "yield",
  // Edition-specific keywords (>= 2018)
  "async", "await", "dyn",
  // Edition-specific reserved (>= 2018/2024)
  "try", "gen",
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
  let mut result = String::new();
  for (i, ch) in name.chars().enumerate() {
    if ch == '_' {
      result.push('_');
    } else if ch.is_uppercase() {
      if i > 0 && !result.ends_with('_') {
        // Don't insert underscore between consecutive uppercase chars
        let prev_upper = name.chars().nth(i - 1).is_some_and(|c| c.is_uppercase());
        let next_lower = name.chars().nth(i + 1).is_some_and(|c| c.is_lowercase());
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
  to_snake_case(name)
    .split('_')
    .filter(|s| !s.is_empty())
    .map(|s| {
      let mut chars = s.chars();
      match chars.next() {
        Some(c) => {
          let mut result = c.to_uppercase().to_string();
          for ch in chars {
            result.push(ch.to_ascii_lowercase());
          }
          result
        }
        None => String::new(),
      }
    })
    .collect()
}

/// Convert a name to SCREAMING_SNAKE_CASE (for constants).
pub fn to_screaming_snake_case(name: &str) -> String {
  to_snake_case(name).to_ascii_uppercase()
}

/// Convert a SCREAMING_SNAKE_CASE name to PascalCase for enum variant names.
///
/// `UNKNOWN` → `Unknown`, `SERVER_STREAM` → `ServerStream`, `UINT32` → `Uint32`
pub fn variant_name(name: &str) -> String {
  name
    .split('_')
    .filter(|s| !s.is_empty())
    .map(|seg| {
      let mut chars = seg.chars();
      match chars.next() {
        Some(c) => {
          let mut result = c.to_uppercase().to_string();
          for ch in chars {
            result.push(ch.to_ascii_lowercase());
          }
          result
        }
        None => String::new(),
      }
    })
    .collect()
}

/// Escape and convert a field name to idiomatic Rust.
pub fn field_name(name: &str) -> String {
  let s = to_snake_case(name);
  // `r#self`, `r#super`, `r#crate` are rejected by rustc — suffix-mangle instead.
  if RAW_REJECTED.contains(&s.as_str()) {
    format!("{}_", s)
  } else {
    escape_keyword(&s)
  }
}

/// Original Bebop field name for serde `rename` when [field_name] mangles `self` / `super`.
pub fn serde_field_rename(name: &str) -> Option<String> {
  let s = to_snake_case(name);
  if RAW_REJECTED.contains(&s.as_str()) {
    Some(s)
  } else {
    None
  }
}

/// Escape and convert a type name to idiomatic Rust.
pub fn type_name(name: &str) -> String {
  let p = to_pascal_case(name);
  // `r#Self` is not a valid raw identifier (rustc rejects it). Bebop string fields are emitted as
  // `alloc::string::String`, so a user-defined type named `String` does not need `String_`.
  match p.as_str() {
    "Self" => "Self_".to_string(),
    _ => escape_keyword(&p),
  }
}

/// Rust enum variant identifier (Bebop member name → PascalCase), avoiding `Self`.
pub fn enum_variant_name(name: &str) -> String {
  let v = variant_name(name);
  if v == "Self" {
    "Self_".to_string()
  } else {
    v
  }
}

/// Convert a const name to idiomatic Rust SCREAMING_SNAKE_CASE.
pub fn const_name(name: &str) -> String {
  to_screaming_snake_case(name)
}

/// Extract the simple type name from a fully-qualified name and convert to PascalCase.
///
/// `"bebop.TypeKind"` → `"TypeKind"`, `"bebop.DefinitionDescriptor"` → `"DefinitionDescriptor"`.
pub fn fqn_to_type_name(fqn: &str) -> String {
  let simple = fqn.rsplit('.').next().unwrap_or(fqn);
  type_name(simple)
}
