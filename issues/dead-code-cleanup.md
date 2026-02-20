# Remove Dead Code in type_mapper.rs

- [x] Remove dead non-Cow generation paths in `type_mapper.rs` #rust-plugin #cleanup ⏫

Three functions in `plugins/rust/src/generator/type_mapper.rs` are marked `#[allow(dead_code)]` with the comment "Kept for non-Cow generation paths during ongoing refactor":

- `rust_type()` (line ~141) — non-Cow type mapper
- `read_expression()` (line ~205) — non-Cow read expression generator
- `write_expression()` (line ~295) — non-Cow write expression generator

The codebase has fully migrated to `_cow` variants (`rust_type_cow`, `read_expression_cow`, `write_expression_cow`). The non-Cow functions are dead code. The refactor is complete — these should be removed.

## Action
Delete the three `#[allow(dead_code)]` functions. Verify `cargo check` still passes.

## Done
- Removed `rust_type()`, `read_expression()`, and `write_expression()` from `plugins/rust/src/generator/type_mapper.rs`.
- Updated stale doc text that referenced the removed `rust_type()` helper.
