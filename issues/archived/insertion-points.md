# Add Insertion Point Markers

- [x] Add insertion point markers to generated code #rust-plugin ✅ ✅ 2026-02-19

Both C and Swift emit `// @@bebop_insertion_point(...)` comments throughout generated code. These markers allow downstream plugins or tooling to inject additional code at well-known locations. Rust now emits the same marker family.

## Required Markers (from Swift/C reference)

### File-level
- `// @@bebop_insertion_point(imports)` — after use/extern declarations
- `// @@bebop_insertion_point(eof)` — at end of file

### Per-type scope
- `// @@bebop_insertion_point(struct_scope:TypeName)` — inside struct impl block
- `// @@bebop_insertion_point(message_scope:TypeName)` — inside message impl block
- `// @@bebop_insertion_point(enum_scope:TypeName)` — inside enum impl block

### Encode/Decode
- `// @@bebop_insertion_point(encode_start:TypeName)` — top of encode fn
- `// @@bebop_insertion_point(encode_end:TypeName)` — bottom of encode fn
- `// @@bebop_insertion_point(decode_start:TypeName)` — top of decode fn
- `// @@bebop_insertion_point(decode_end:TypeName)` — bottom of decode fn

### Union-specific
- `// @@bebop_insertion_point(encode_switch:TypeName)` — inside encode match
- `// @@bebop_insertion_point(decode_switch:TypeName)` — inside decode match

## Implemented (Rust)

### File-level
- [x] `imports`
- [x] `eof`

### Type scope
- [x] `struct_scope:TypeName`
- [x] `message_scope:TypeName`
- [x] `enum_scope:TypeName`
- [x] `union_scope:TypeName` (added for parity with Swift and Rust union extensibility)

### Encode/Decode hooks
- [x] `encode_start:TypeName`
- [x] `encode_end:TypeName`
- [x] `decode_start:TypeName`
- [x] `decode_end:TypeName`

### Union-specific hooks
- [x] `encode_switch:TypeName`
- [x] `decode_switch:TypeName`

## Implementation
Implemented in:
- `plugins/rust/src/generator/mod.rs`
- `plugins/rust/src/generator/gen_struct.rs`
- `plugins/rust/src/generator/gen_message.rs`
- `plugins/rust/src/generator/gen_enum.rs`
- `plugins/rust/src/generator/gen_union.rs`

Validation:
- Added unit tests in `plugins/rust/src/generator/mod.rs` to assert marker presence and ordering.

## Follow-up (out of scope)
- Service generation is still a TODO in Rust, so there is currently no generated service body to attach `service_scope:*` markers to.

## References
- Swift: `GenerateStruct.swift`, `GenerateMessage.swift`, `GenerateEnum.swift`, `GenerateUnion.swift`
- C: `generator.c` lines ~1557, 1917, 3397, 3999, etc.
