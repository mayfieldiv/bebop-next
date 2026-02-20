# Rust Plugin Production Readiness

- [x] Fix lifetime analysis forward-reference bug with fixpoint resolution #rust-plugin 🔺
- [x] Add host options/parameter framework (`Visibility=public|crate`) and thread through generators #rust-plugin 🔺
- [x] Add feature-gated serde integration in runtime + generated code #rust-plugin 🔺
- [x] Document deprecated message field encode/decode behavior divergence in generator output #rust-plugin 🔽
- [x] Add Rust Criterion benchmark suite and cross-language benchmark harness/reporting #rust-plugin 🔽
- [x] Expand integration coverage for forward refs, deprecated fields, integer map keys, deep nesting, empty collections, and serde round-trips #rust-plugin 🔺
- [x] Eliminate benchmark post-generation patching by fixing generator constructor/derive output and formatting generation scripts #rust-plugin 🔺
- [x] Harden generated Rust against schema type-name shadowing (`String`, `Vec`, `Box`, `Cow`, `HashMap`, `Uuid`) by emitting qualified paths and collision-safe preamble imports #rust-plugin 🔺 ✅ 2026-02-20
- [x] Enforce `no_implicit_prelude` compatibility in generated Rust without wildcard prelude imports (extern crates + anonymous trait imports only) #rust-plugin 🔺 ✅ 2026-02-20
- [x] Fully qualify prelude-derived generated symbols (`Result`, `Option`, `Ok`/`Err`, `Some`/`None`, `TryFrom`, `Into`) to avoid schema-name collisions #rust-plugin 🔺 ✅ 2026-02-20
