# Plan: Struct Layout Optimization

**Status:** Draft — No Change Needed (with future spec consideration noted)
**Depends on:** None
**Blocks:** None

## Problem Statement

The Rust plugin emits struct fields in schema declaration order. The C plugin reorders fields by alignment (descending) to minimize padding. The question is whether the Rust plugin should do the same.

## Current Code Analysis

### `gen_struct.rs:76-86` — Field Emission Order

```rust
for (f, meta) in fields.iter().zip(&field_metas) {
  emit_doc_comment(output, &f.documentation);
  emit_deprecated(output, &f.decorators);
  // ... serde attributes ...
  output.push_str(&format!("  {} {}: {},\n", vis, meta.fname, meta.cow_type));
}
```

Fields are emitted in the order they appear in `struct_def.fields`, which is the schema declaration order.

### Wire Format vs Memory Layout

**Critical distinction:** The Bebop wire format for structs encodes fields in **declaration order** with no padding or alignment gaps. The wire format is always:

```
field_0_bytes | field_1_bytes | field_2_bytes | ...
```

No matter what order the Rust struct fields are in memory, the `encode()` and `decode()` implementations read/write fields in declaration order:

```rust
// gen_struct.rs:209-213 — encode in declaration order
for meta in &field_metas {
  let write_stmt = type_mapper::write_expression(meta.td, &format!("self.{}", meta.fname), ...);
  output.push_str(&format!("    {};\n", write_stmt));
}

// gen_struct.rs:247-253 — decode in declaration order
for meta in &field_metas {
  let read_expr = type_mapper::read_expression(meta.td, "reader", analysis)?;
  output.push_str(&format!("    let {} = {}?;\n", meta.fname, read_expr));
}
```

### Rust's Default Layout

Rust's default struct layout (`repr(Rust)`) already allows the compiler to reorder fields for optimal padding. From the Rust Reference:

> The default representation... does not make any guarantees about the layout.

This means `rustc` **already reorders fields** to minimize padding in the compiled binary. The declaration order in source code is irrelevant to the compiled layout.

### When It Matters

Field order in the source code affects:
1. **Readability** — declaration order matches the schema, which is meaningful
2. **Derive macros** — `Debug` output follows declaration order
3. **Construction order** — struct literal `MyStruct { a, b, c }` uses any order
4. **Drop order** — fields are dropped in declaration order (rarely matters for Bebop types)

Field order in the source code does NOT affect:
1. **Memory layout** — compiler reorders freely
2. **Wire format** — encode/decode always use declaration order regardless
3. **Performance** — compiler handles alignment optimization

## Analysis: Is This Even a Problem?

### Comparison with C

The C plugin sorts fields by alignment because C has `struct` layout guarantees — fields are laid out in declaration order with platform-specific padding. If a C struct has `{ char a; int64_t b; char c; }`, the compiler inserts 7 bytes of padding after `a` and 7 after `c`, wasting 14 bytes. Reordering to `{ int64_t b; char a; char c; }` eliminates most padding.

**This does not apply to Rust.** The Rust compiler already handles this automatically.

### When `#[repr(C)]` Is Used

If the generated struct used `#[repr(C)]`, field order would matter. But the Rust plugin does NOT emit `#[repr(C)]` on structs — only on enums (`#[repr(u32)]` etc.). So this is irrelevant.

## Spec-Level Consideration: Ordered Fields for Cross-Language Memory Blitting

There's an interesting idea that goes beyond per-language code generation: what if the Bebop **spec itself** mandated that struct fields be ordered by decreasing alignment size? This would mean:

- The wire format would have fields in alignment-optimal order (largest first)
- On little-endian platforms, the wire format for a struct of fixed-size scalars would match the `repr(C)` memory layout with no padding (since fields are already alignment-sorted)
- **Every** language implementation could potentially blit the wire bytes directly into a struct without any field reordering or copying

### What This Would Enable

For a struct like `{ x: float64, y: float64, id: uint32, flags: uint8 }` with fields already in decreasing-size order:

| Field | Size | Offset |
|---|---|---|
| x (f64) | 8 | 0 |
| y (f64) | 8 | 8 |
| id (u32) | 4 | 16 |
| flags (u8) | 1 | 20 |

The wire format bytes at offsets 0-20 are identical to the `repr(C)` memory layout (assuming no trailing padding). On little-endian platforms, a C program could cast the buffer pointer directly to a struct pointer. A Rust program could use `repr(C)` and do the same (or use `bytemuck::from_bytes`). Swift, Go, and other languages with C-compatible struct layouts could also participate.

### Challenges

1. **Wire format change:** This would be a breaking change to the Bebop wire format. Existing encoded data has fields in declaration order (which may not be alignment-sorted). This makes it unlikely as a retroactive change, but could apply to a new struct encoding mode.

2. **Trailing padding:** Even with alignment-sorted fields, the `repr(C)` struct may have trailing padding to satisfy the struct's overall alignment. The wire format would need to either include this padding (wasting bytes) or accept that the last few bytes need special handling.

3. **Variable-size fields:** This only works for structs composed entirely of fixed-size scalars. If a struct contains strings, arrays, or other variable-length types, the wire format can't be memory-mapped regardless of field order.

4. **Pointer-sized fields:** Types like arrays and strings become pointers in the in-memory representation but are variable-length in the wire format. Alignment sorting doesn't help for these.

5. **Cross-platform compatibility:** Big-endian platforms would still need byte-swapping. The blitting optimization is inherently little-endian-only (or big-endian-only if the spec chose big-endian, but LE is far more common).

### Verdict

This is an interesting idea for a future spec extension (e.g., a `packed struct` or `blittable struct` annotation) that could opt into alignment-ordered fields with a guaranteed memory-compatible wire format. It's not something the Rust plugin should unilaterally implement, since:

- It would require spec-level coordination across all plugins
- The benefit only applies to all-scalar structs on LE platforms
- The current wire format is already defined and can't be changed retroactively

If this becomes a spec feature, the Rust plugin would emit `#[repr(C)]` on such structs and could use direct memory mapping for encode/decode.

## Recommendation: No Change Needed

The Rust compiler already optimizes struct layout. Reordering fields in the generated source would:

1. **Not improve memory layout** — compiler already does this
2. **Hurt readability** — fields would no longer match schema order
3. **Complicate the generator** — need alignment lookup table, sort logic
4. **Not affect wire format** — encode/decode already use declaration order
5. **Diverge from schema intent** — schema authors choose field order for documentation purposes

## If We Did It Anyway (What It Would Look Like)

For completeness, here's what the change would require if we needed it (e.g., for `#[repr(C)]` structs in a future FFI mode):

### `gen_struct.rs` — Sort Field Metadata

```rust
// After collecting field_metas (line 44-58):
let mut display_order: Vec<(usize, &StructFieldMeta)> = field_metas.iter().enumerate().collect();
display_order.sort_by(|a, b| {
  let align_a = alignment_of_type(a.1.td);
  let align_b = alignment_of_type(b.1.td);
  align_b.cmp(&align_a)  // descending alignment
});

// Use display_order for struct field emission
// But keep field_metas in original order for encode/decode
```

### `type_mapper.rs` — Alignment Lookup

```rust
fn alignment_of_type(td: &TypeDescriptor) -> usize {
  match td.kind {
    Some(TypeKind::Bool) | Some(TypeKind::Byte) | Some(TypeKind::Int8) => 1,
    Some(TypeKind::Uint16) | Some(TypeKind::Int16) | Some(TypeKind::Float16) | Some(TypeKind::Bfloat16) => 2,
    Some(TypeKind::Uint32) | Some(TypeKind::Int32) | Some(TypeKind::Float32) => 4,
    Some(TypeKind::Uint64) | Some(TypeKind::Int64) | Some(TypeKind::Float64) => 8,
    Some(TypeKind::Guid) | Some(TypeKind::Uint128) | Some(TypeKind::Int128) => 16,
    Some(TypeKind::String) | Some(TypeKind::Array) | Some(TypeKind::Map) => 8, // pointer size
    Some(TypeKind::Defined) => 8, // unknown, assume pointer
    _ => 1,
  }
}
```

### Cost

- ~30 lines of new code in `gen_struct.rs` and `type_mapper.rs`
- Maintenance burden of keeping alignment table accurate
- **Zero performance benefit** (compiler already does this for `repr(Rust)`)
- Readability regression

## Cost-Benefit Analysis

### Benefits of Implementing

- None for `repr(Rust)` structs (which is what we generate)

### Costs of Implementing

- ~30 lines of new code
- Ongoing maintenance of alignment table
- Reduced readability of generated code
- Field order in `Debug` output diverges from schema

### Recommendation

**Do not implement.** This is a solved problem in Rust. The compiler's field reordering is more sophisticated than any manual sort we could implement (it considers the full struct layout holistically, not just individual field alignment).

The spec-level field ordering idea is interesting and worth considering as a future Bebop spec feature, but it's outside the scope of the Rust plugin alone.

## Test Plan

No implementation → no tests needed. This plan serves as documentation that the issue was evaluated and found to be unnecessary for Rust, with the spec-level consideration noted for future reference.
