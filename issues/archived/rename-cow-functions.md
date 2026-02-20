# Rename _cow Functions After Dead Code Removal

- [x] Rename `_cow` suffixed functions to their canonical names #rust-plugin #cleanup 🆔 rename-cow ⛔ dead-code 🔽 ✅ 2026-02-19

After [[dead-code-cleanup]] removes the old non-Cow functions, the `_cow` suffix on the surviving functions becomes meaningless:

- `rust_type_cow()` → `rust_type()`
- `read_expression_cow()` → `read_expression()`
- `write_expression_cow()` → `write_expression()`
- `size_expression_cow()` → `size_expression()` (if similarly suffixed)

The `_cow` suffix was a disambiguation during the refactor. Once the old functions are gone, rename to the cleaner names.

## Scope
`plugins/rust/src/generator/type_mapper.rs` — function renames + update all call sites across the generator modules.

## Done
- Renamed `rust_type_cow()` to `rust_type()`.
- Renamed `read_expression_cow()` to `read_expression()`.
- Renamed `write_expression_cow()` to `write_expression()`.
- Updated all call sites in `gen_struct.rs` and `gen_message.rs`.
