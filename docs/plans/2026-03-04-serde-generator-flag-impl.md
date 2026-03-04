# Serde Generator Flag Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace conditional `cfg_attr(feature = "serde", ...)` in generated code with a generator-side serde option parsed from the `parameter` field of `CodeGeneratorRequest`. Supports three modes: `Disabled` (no serde code), `Always` (unconditional derives), and `FeatureGated` (cfg_attr with a configurable feature name).

**Architecture:** Add a `SerdeMode` enum to `GeneratorOptions`. Serde mode is parsed from `parameter` (the plugin-specific option string from `--rust_opt=...`), while `Visibility` stays in `host_options`. All serde-emitting code paths use shared `SerdeMode` helpers. An e2e test constructs a `CodeGeneratorRequest` with `parameter` set, pipes it to the binary, and asserts the generated code.

**Tech Stack:** Rust, bebopc-gen-rust code generator

**Option syntax** (via `--rust_opt=...`, stored in `CodeGeneratorRequest.parameter`):
- `serde` → `SerdeMode::Always` (unconditional derives)
- `serde-feature:my_feat` → `SerdeMode::FeatureGated("my_feat")` (cfg_attr wrapped)
- absent → `SerdeMode::Disabled` (no serde code)

Multiple options are comma-separated in `parameter`, e.g. `"serde,other_option"`.

---

### Task 1: Add `SerdeMode` enum and parse from `parameter`

**Files:**
- Modify: `src/generator/mod.rs:55-78` (GeneratorOptions struct, Default impl, parsing)
- Modify: `src/main.rs:80` (pass parameter to options parsing)

**Step 1: Write the failing tests**

In `src/generator/mod.rs`, in the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn serde_defaults_to_disabled() {
  let options = GeneratorOptions::new(None, None).unwrap();
  assert_eq!(options.serde, SerdeMode::Disabled);
}

#[test]
fn parses_serde_from_parameter() {
  let options = GeneratorOptions::new(None, Some("serde")).unwrap();
  assert_eq!(options.serde, SerdeMode::Always);
}

#[test]
fn parses_serde_feature_from_parameter() {
  let options = GeneratorOptions::new(None, Some("serde-feature:my_feat")).unwrap();
  assert_eq!(options.serde, SerdeMode::FeatureGated(String::from("my_feat")));
}

#[test]
fn parses_serde_among_multiple_params() {
  let options = GeneratorOptions::new(None, Some("serde,something_else")).unwrap();
  assert_eq!(options.serde, SerdeMode::Always);
}

#[test]
fn rejects_unknown_parameter() {
  let err = GeneratorOptions::new(None, Some("bogus")).unwrap_err();
  assert!(matches!(err, GeneratorError::InvalidOption(_)));
}

#[test]
fn visibility_still_from_host_options() {
  let mut host_options = HashMap::new();
  host_options.insert(Cow::Borrowed("Visibility"), Cow::Borrowed("crate"));
  let options = GeneratorOptions::new(Some(&host_options), None).unwrap();
  assert_eq!(options.default_visibility, DefaultVisibility::Crate);
}

#[test]
fn both_host_options_and_parameter() {
  let mut host_options = HashMap::new();
  host_options.insert(Cow::Borrowed("Visibility"), Cow::Borrowed("crate"));
  let options = GeneratorOptions::new(Some(&host_options), Some("serde")).unwrap();
  assert_eq!(options.default_visibility, DefaultVisibility::Crate);
  assert_eq!(options.serde, SerdeMode::Always);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p bebopc-gen-rust -- serde`
Expected: compile errors — `SerdeMode` type and `new` method don't exist

**Step 3: Implement**

Add `SerdeMode` enum before `GeneratorOptions`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerdeMode {
  /// No serde code emitted.
  Disabled,
  /// Unconditional serde derives (no cfg_attr).
  Always,
  /// Wrap serde derives in `cfg_attr(feature = "<name>", ...)`.
  FeatureGated(String),
}
```

Update `GeneratorOptions`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorOptions {
  pub default_visibility: DefaultVisibility,
  pub serde: SerdeMode,
}
```

Note: remove `Copy` from derives since `SerdeMode` contains a `String`.

Update `Default`:

```rust
impl Default for GeneratorOptions {
  fn default() -> Self {
    Self {
      default_visibility: DefaultVisibility::Public,
      serde: SerdeMode::Disabled,
    }
  }
}
```

Add a new `GeneratorOptions::new` that takes both sources. Keep `from_host_options` as a thin wrapper for backwards compat with existing tests:

```rust
impl GeneratorOptions {
  pub fn new(
    host_options: Option<&HashMap<Cow<'_, str>, Cow<'_, str>>>,
    parameter: Option<&str>,
  ) -> Result<Self, GeneratorError> {
    let mut options = Self::default();

    // Host options: Visibility
    if let Some(host_options) = host_options {
      if let Some(value) = host_options.get("Visibility") {
        options.default_visibility = DefaultVisibility::parse(value.as_ref())?;
      }
    }

    // Plugin parameter: serde, serde-feature:<name>
    if let Some(param) = parameter {
      for token in param.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if token == "serde" {
          options.serde = SerdeMode::Always;
        } else if let Some(feat) = token.strip_prefix("serde-feature:") {
          options.serde = SerdeMode::FeatureGated(feat.to_string());
        } else {
          return Err(GeneratorError::InvalidOption(format!(
            "unknown plugin option: {:?}",
            token
          )));
        }
      }
    }

    Ok(options)
  }

  /// Parse from host_options only (backwards compat for existing callers).
  pub fn from_host_options(
    host_options: Option<&HashMap<Cow<'_, str>, Cow<'_, str>>>,
  ) -> Result<Self, GeneratorError> {
    Self::new(host_options, None)
  }
}
```

Update `src/main.rs` to pass `parameter`:

```rust
// Old:
let generator_options = GeneratorOptions::from_host_options(request.host_options.as_ref())?;

// New:
let generator_options = GeneratorOptions::new(
  request.host_options.as_ref(),
  request.parameter.as_deref(),
)?;
```

Fix any existing test that constructs `GeneratorOptions` literally — add `serde: SerdeMode::Disabled` or use `..Default::default()`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p bebopc-gen-rust`
Expected: all pass

**Step 5: Commit**

```
feat(gen): add SerdeMode enum, parse serde option from parameter
```

---

### Task 2: Add serde attribute helpers and update file header emission

**Files:**
- Modify: `src/generator/mod.rs:535` (serde import line in `generate()`)

**Step 1: Write the failing tests**

In `src/generator/mod.rs` tests:

```rust
#[test]
fn serde_always_emits_unconditional_import() {
  let schema = SchemaDescriptor {
    path: Some(Cow::Borrowed("test.bop")),
    definitions: Some(vec![]),
    ..Default::default()
  };
  let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
  let output = RustGenerator::with_options(
    None,
    GeneratorOptions { serde: SerdeMode::Always, ..Default::default() },
  )
  .generate(&schema, &[], &analysis)
  .expect("generator should succeed");

  assert!(output.contains("use bebop_runtime::serde;\n"));
  assert!(!output.contains("cfg(feature"));
}

#[test]
fn serde_feature_gated_emits_cfg_import() {
  let schema = SchemaDescriptor {
    path: Some(Cow::Borrowed("test.bop")),
    definitions: Some(vec![]),
    ..Default::default()
  };
  let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
  let output = RustGenerator::with_options(
    None,
    GeneratorOptions {
      serde: SerdeMode::FeatureGated(String::from("my-serde")),
      ..Default::default()
    },
  )
  .generate(&schema, &[], &analysis)
  .expect("generator should succeed");

  assert!(output.contains("#[cfg(feature = \"my-serde\")]\nuse bebop_runtime::serde;\n"));
}

#[test]
fn serde_disabled_omits_import() {
  let schema = SchemaDescriptor {
    path: Some(Cow::Borrowed("test.bop")),
    definitions: Some(vec![]),
    ..Default::default()
  };
  let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
  let output = RustGenerator::with_options(
    None,
    GeneratorOptions { serde: SerdeMode::Disabled, ..Default::default() },
  )
  .generate(&schema, &[], &analysis)
  .expect("generator should succeed");

  assert!(!output.contains("bebop_runtime::serde"));
}
```

**Step 2: Implement**

Add helper methods on `SerdeMode`:

```rust
impl SerdeMode {
  pub fn emit_derive(&self, output: &mut String) {
    match self {
      SerdeMode::Disabled => {}
      SerdeMode::Always => {
        output.push_str("#[derive(serde::Serialize, serde::Deserialize)]\n");
      }
      SerdeMode::FeatureGated(feat) => {
        output.push_str(&format!(
          "#[cfg_attr(feature = \"{}\", derive(serde::Serialize, serde::Deserialize))]\n",
          feat
        ));
      }
    }
  }

  pub fn emit_field_attr(&self, output: &mut String, attr: &str) {
    match self {
      SerdeMode::Disabled => {}
      SerdeMode::Always => {
        output.push_str(&format!("  #[serde({})]\n", attr));
      }
      SerdeMode::FeatureGated(feat) => {
        output.push_str(&format!(
          "  #[cfg_attr(feature = \"{}\", serde({}))]\n",
          feat, attr
        ));
      }
    }
  }

  pub fn emit_type_attr(&self, output: &mut String, attr: &str) {
    match self {
      SerdeMode::Disabled => {}
      SerdeMode::Always => {
        output.push_str(&format!("#[serde({})]\n", attr));
      }
      SerdeMode::FeatureGated(feat) => {
        output.push_str(&format!(
          "#[cfg_attr(feature = \"{}\", serde({}))]\n",
          feat, attr
        ));
      }
    }
  }

  pub fn is_enabled(&self) -> bool {
    !matches!(self, SerdeMode::Disabled)
  }
}
```

Replace line 535 in `generate()`:

```rust
match &self.options.serde {
  SerdeMode::Disabled => {}
  SerdeMode::Always => {
    output.push_str("use bebop_runtime::serde;\n");
  }
  SerdeMode::FeatureGated(feat) => {
    output.push_str(&format!(
      "#[cfg(feature = \"{}\")]\nuse bebop_runtime::serde;\n",
      feat
    ));
  }
}
```

**Step 3: Run tests, commit**

Run: `cargo test -p bebopc-gen-rust`

```
feat(gen): add SerdeMode helpers and update serde import emission
```

---

### Task 3: Update gen_struct.rs to use SerdeMode helpers

**Files:**
- Modify: `src/generator/gen_struct.rs:73,80-83`

**Step 1: Write the failing test**

```rust
#[test]
fn serde_always_emits_unconditional_struct_derive() {
  let payload = DefinitionDescriptor {
    kind: Some(DefinitionKind::Struct),
    name: Some(Cow::Borrowed("Payload")),
    fqn: Some(Cow::Borrowed("test.Payload")),
    struct_def: Some(StructDef {
      fields: Some(vec![FieldDescriptor {
        name: Some(Cow::Borrowed("id")),
        r#type: Some(scalar_type(TypeKind::Int32)),
        index: Some(0),
        ..Default::default()
      }]),
      fixed_size: Some(4),
      ..Default::default()
    }),
    ..Default::default()
  };

  let schema = SchemaDescriptor {
    path: Some(Cow::Borrowed("test.bop")),
    definitions: Some(vec![payload]),
    ..Default::default()
  };
  let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
  let output = RustGenerator::with_options(
    None,
    GeneratorOptions { serde: SerdeMode::Always, ..Default::default() },
  )
  .generate(&schema, &[], &analysis)
  .expect("generator should succeed");

  assert!(
    output.contains("#[derive(serde::Serialize, serde::Deserialize)]\n#[derive(Debug"),
    "should emit unconditional serde derive, got:\n{}", output
  );
  assert!(!output.contains("cfg_attr"), "should not contain cfg_attr");
}

#[test]
fn serde_disabled_omits_struct_derive() {
  let payload = DefinitionDescriptor {
    kind: Some(DefinitionKind::Struct),
    name: Some(Cow::Borrowed("Payload")),
    fqn: Some(Cow::Borrowed("test.Payload")),
    struct_def: Some(StructDef {
      fields: Some(vec![FieldDescriptor {
        name: Some(Cow::Borrowed("id")),
        r#type: Some(scalar_type(TypeKind::Int32)),
        index: Some(0),
        ..Default::default()
      }]),
      fixed_size: Some(4),
      ..Default::default()
    }),
    ..Default::default()
  };

  let schema = SchemaDescriptor {
    path: Some(Cow::Borrowed("test.bop")),
    definitions: Some(vec![payload]),
    ..Default::default()
  };
  let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
  let output = RustGenerator::with_options(
    None,
    GeneratorOptions { serde: SerdeMode::Disabled, ..Default::default() },
  )
  .generate(&schema, &[], &analysis)
  .expect("generator should succeed");

  assert!(!output.contains("serde"), "should not contain any serde references");
}
```

**Step 2: Implement**

Line 73 → `options.serde.emit_derive(output);`

Lines 79-84 →
```rust
if options.serde.is_enabled() && type_mapper::is_byte_array_cow_field(meta.td) {
  options.serde.emit_field_attr(output, "borrow");
  options.serde.emit_field_attr(output, "with = \"bebop_runtime::serde_cow_bytes\"");
}
```

**Step 3: Run tests, commit**

```
feat(gen_struct): use SerdeMode helpers for serde emission
```

---

### Task 4: Update gen_message.rs to use SerdeMode helpers

**Files:**
- Modify: `src/generator/gen_message.rs:122,128-133`

**Step 1: Implement**

Line 122 → `options.serde.emit_derive(output);`

Lines 128-133 →
```rust
if options.serde.is_enabled() && type_mapper::is_byte_array_cow_field(meta.td) {
  options.serde.emit_field_attr(output, "borrow");
  options.serde.emit_field_attr(output, "with = \"bebop_runtime::serde_cow_bytes\"");
}
```

**Step 2: Run tests, commit**

```
feat(gen_message): use SerdeMode helpers for serde emission
```

---

### Task 5: Update gen_enum.rs — add `options` to `EnumCtx` and use SerdeMode helpers

**Files:**
- Modify: `src/generator/gen_enum.rs:12-24,75-87,112,239,393`

**Step 1: Implement**

Add `options: &'a GeneratorOptions` field to `EnumCtx`. Thread it through from `generate()`.

Replace lines 112, 239, 393:
```rust
ctx.options.serde.emit_derive(ctx.output);
```

**Step 2: Run tests, commit**

```
feat(gen_enum): thread options into EnumCtx, use SerdeMode helpers
```

---

### Task 6: Update gen_union.rs to use SerdeMode helpers

**Files:**
- Modify: `src/generator/gen_union.rs:80-81,97`

**Step 1: Implement**

Lines 80-81 →
```rust
options.serde.emit_derive(output);
options.serde.emit_type_attr(output, "tag = \"type\", content = \"value\"");
```

Line 97 →
```rust
if is_forward_compatible {
  options.serde.emit_field_attr(output, "skip");
  output.push_str("  Unknown(u8, alloc::borrow::Cow<'buf, [u8]>),\n");
}
```

**Step 2: Run tests, commit**

```
feat(gen_union): use SerdeMode helpers for serde emission
```

---

### Task 7: E2e binary test

**Files:**
- Create: `tests/e2e.rs` (in the bebopc-gen-rust crate)

This test constructs a `CodeGeneratorRequest`, serializes it, pipes it to the compiled `bebopc-gen-rust` binary, decodes the `CodeGeneratorResponse`, and asserts the generated code.

**Step 1: Write the e2e tests**

```rust
//! End-to-end tests that invoke the bebopc-gen-rust binary via stdin/stdout.

use std::borrow::Cow;
use std::process::Command;

use bebop_runtime::{BebopDecode, BebopEncode, BebopReader, BebopWriter};

// Import the generated protocol types (they live in the same crate)
#[path = "../src/generated/mod.rs"]
mod generated;
use generated::*;

fn invoke_generator(request: &CodeGeneratorRequest) -> CodeGeneratorResponse<'static> {
  let mut writer = BebopWriter::new();
  request.encode(&mut writer);
  let input_bytes = writer.into_bytes();

  let binary = env!("CARGO_BIN_EXE_bebopc-gen-rust");
  let output = Command::new(binary)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .and_then(|mut child| {
      use std::io::Write;
      child.stdin.take().unwrap().write_all(&input_bytes)?;
      child.wait_with_output()
    })
    .expect("failed to run bebopc-gen-rust");

  assert!(output.status.success(), "generator failed: {}", String::from_utf8_lossy(&output.stderr));

  let mut reader = BebopReader::new(&output.stdout);
  CodeGeneratorResponse::decode(&mut reader)
    .expect("failed to decode response")
    .into_owned()
}

fn make_request(parameter: Option<&str>) -> CodeGeneratorRequest<'static> {
  let schema_path = "/test/test_types.bop";
  CodeGeneratorRequest {
    files_to_generate: Some(vec![Cow::Owned(schema_path.to_string())]),
    parameter: parameter.map(|s| Cow::Owned(s.to_string())),
    compiler_version: None,
    schemas: Some(vec![SchemaDescriptor {
      path: Some(Cow::Owned(schema_path.to_string())),
      definitions: Some(vec![
        DefinitionDescriptor {
          kind: Some(DefinitionKind::Struct),
          name: Some(Cow::Borrowed("Point")),
          fqn: Some(Cow::Borrowed("test.Point")),
          struct_def: Some(StructDef {
            fields: Some(vec![
              FieldDescriptor {
                name: Some(Cow::Borrowed("x")),
                r#type: Some(TypeDescriptor {
                  kind: Some(TypeKind::Float32),
                  ..Default::default()
                }),
                index: Some(0),
                ..Default::default()
              },
            ]),
            fixed_size: Some(4),
            ..Default::default()
          }),
          ..Default::default()
        },
      ]),
      ..Default::default()
    }]),
    host_options: None,
  }
}

#[test]
fn e2e_serde_always() {
  let request = make_request(Some("serde"));
  let response = invoke_generator(&request);

  assert!(response.error.is_none(), "generator error: {:?}", response.error);
  let files = response.files.expect("should have generated files");
  assert_eq!(files.len(), 1);
  let code = files[0].content.as_deref().expect("file should have content");

  assert!(
    code.contains("#[derive(serde::Serialize, serde::Deserialize)]"),
    "should contain unconditional serde derive"
  );
  assert!(
    !code.contains("cfg_attr"),
    "should not contain cfg_attr"
  );
  assert!(
    code.contains("use bebop_runtime::serde;"),
    "should contain unconditional serde import"
  );
}

#[test]
fn e2e_serde_feature_gated() {
  let request = make_request(Some("serde-feature:my_feat"));
  let response = invoke_generator(&request);

  assert!(response.error.is_none());
  let files = response.files.expect("should have generated files");
  let code = files[0].content.as_deref().unwrap();

  assert!(
    code.contains("#[cfg_attr(feature = \"my_feat\", derive(serde::Serialize, serde::Deserialize))]"),
    "should contain feature-gated serde derive"
  );
  assert!(
    code.contains("#[cfg(feature = \"my_feat\")]\nuse bebop_runtime::serde;"),
    "should contain feature-gated serde import"
  );
}

#[test]
fn e2e_serde_disabled() {
  let request = make_request(None);
  let response = invoke_generator(&request);

  assert!(response.error.is_none());
  let files = response.files.expect("should have generated files");
  let code = files[0].content.as_deref().unwrap();

  assert!(
    !code.contains("serde"),
    "should not contain any serde references when disabled"
  );
}
```

**Step 2: Run the e2e tests**

Run: `cargo test -p bebopc-gen-rust --test e2e`
Expected: all 3 pass

**Step 3: Commit**

```
test: add e2e binary tests for SerdeMode (Always/FeatureGated/Disabled)
```

---

### Task 8: Update generated files and benchmarks/integration-tests

**Files:**
- Modify: `benchmarks/src/benchmark_types.rs` — update serde attrs to unconditional (Always mode)
- Modify: `benchmarks/Cargo.toml` — remove any `[lints.rust]` workaround if present
- Modify: `integration-tests/src/test_types.rs` — update serde attrs to unconditional (Always mode)

**Step 1: Update generated files**

In `benchmarks/src/benchmark_types.rs`:
- Replace `#[cfg(feature = "serde")]\nuse bebop_runtime::serde;` → `use bebop_runtime::serde;`
- Replace all `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` → `#[derive(serde::Serialize, serde::Deserialize)]`
- Replace all `#[cfg_attr(feature = "serde", serde(...))]` → `#[serde(...)]`

Do the same in `integration-tests/src/test_types.rs`.

**Step 2: Run the full test suite**

Run: `bash test.sh` (from `plugins/rust/`)
Expected: "All checks passed."

**Step 3: Commit**

```
chore: regenerate files with Always serde mode
```

---

### Task 9: Final verification

**Step 1: Run the full test.sh**

Run: `bash test.sh`
Expected: "All checks passed." — no unexpected_cfgs warnings

**Step 2: Verify no hardcoded cfg_attr(feature = "serde") remains in generator**

Run: `grep -rn 'cfg_attr.*serde\|cfg.*feature.*serde' src/generator/`
Expected: no matches (all cfg_attr emission is now dynamic via SerdeMode helpers)

**Step 3: Verify no cfg_attr(feature = "serde") remains in generated files**

Run: `grep -rn 'cfg_attr.*serde\|cfg.*feature.*serde' benchmarks/src/ integration-tests/src/`
Expected: no matches
