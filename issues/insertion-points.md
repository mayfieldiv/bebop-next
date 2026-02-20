# Add Insertion Point Markers

- [ ] Add insertion point markers to generated code #rust-plugin 🔽

Both C and Swift emit `// @@bebop_insertion_point(...)` comments throughout generated code. These markers allow downstream plugins or tooling to inject additional code at well-known locations. The Rust plugin emits none.

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

## Implementation
Small, mechanical change across `gen_struct.rs`, `gen_message.rs`, `gen_enum.rs`, `gen_union.rs`, and the top-level file emitter. Low risk.

## References
- Swift: `GenerateStruct.swift`, `GenerateMessage.swift`, `GenerateEnum.swift`, `GenerateUnion.swift`
- C: `generator.c` lines ~1557, 1917, 3397, 3999, etc.
