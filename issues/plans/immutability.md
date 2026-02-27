# Plan: Immutability Support for Struct Fields

**Status:** Draft — No Change Needed
**Depends on:** None
**Blocks:** None

## Problem Statement

The Rust plugin currently emits all struct fields with `pub` visibility, ignoring the `is_mutable` flag on `StructDef`. The Bebop schema distinguishes between mutable and immutable structs (declared with `mut`), and other plugins honor this:

- **C plugin:** Uses `const` qualifier on immutable struct fields
- **Swift plugin:** Uses `let` (immutable) vs `var` (mutable) properties

The Rust plugin treats every struct identically regardless of `is_mutable`.

## Current Code Analysis

### Schema: `StructDef.is_mutable`

```rust
// rust/src/generated/descriptor.rs:1777-1784
pub struct StructDef<'buf> {
  pub fields: ::core::option::Option<alloc::vec::Vec<FieldDescriptor<'buf>>>,
  /// True when declared with `mut`. Mutable structs allow field reassignment.
  pub is_mutable: ::core::option::Option<bool>,
  /// Total wire bytes when all fields are fixed-size.
  pub fixed_size: ::core::option::Option<u32>,
}
```

The `is_mutable` field is available but never read by the generator.

### Generator: `gen_struct.rs`

At line 85, fields are emitted uniformly:

```rust
// gen_struct.rs:85
output.push_str(&format!("  {} {}: {},\n", vis, meta.fname, meta.cow_type));
```

The `vis` variable comes from `visibility_keyword()` (mod.rs:19-24), which maps schema visibility to `pub` or `pub(crate)`. There is no field-level mutability concept.

### Constructor: `gen_struct.rs:137-182`

The `new()` constructor accepts all fields as parameters and constructs the struct directly. For mutable structs, the constructor is fine as-is. For immutable structs, the constructor is the only intended mutation point.

## Design Options

### Option A: Make Immutable Struct Fields Private with Getters

For structs where `is_mutable` is `false` (or `None`, the default), emit fields as private with getter methods:

```rust
// Generated for: struct Point { x: i32; y: i32; }
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Point {
  x: i32,   // no `pub` — field is private
  y: i32,
}

impl Point {
  pub fn new(x: i32, y: i32) -> Self {
    Self { x, y }
  }

  pub fn x(&self) -> i32 { self.x }
  pub fn y(&self) -> i32 { self.y }
}
```

For mutable structs (`mut struct Point { ... }`), continue emitting `pub` fields as today:

```rust
// Generated for: mut struct MutablePoint { x: i32; y: i32; }
pub struct MutablePoint {
  pub x: i32,
  pub y: i32,
}
```

**Getters for borrowed types** would return references:

```rust
pub fn name(&self) -> &str { &self.name }  // Cow<str> auto-derefs
```

For complex types (arrays, maps, defined types), return `&T`.

**Costs of Option A:**

- ~3-5 lines per field for getter methods on immutable structs
- New helper function for getter return types (~30 lines in type_mapper.rs)
- Users must call `point.x()` instead of `point.x` for immutable structs
- Pattern matching (`let Point { x, y } = point`) no longer works for immutable structs
- The `new()` constructor still allows construction of "immutable" structs, so immutability is enforced at the field level, not the type level

### Option B: No-Op — Document as Intentional Divergence (Recommended)

Rust has no `const` field qualifier. Instead, Rust's ownership model handles mutability explicitly at the binding site: a variable must be declared `let mut` to allow any mutation. This is fundamentally different from C's `const` or Swift's `let` vs `var`, which qualify fields/properties at the type level.

In Rust, given:

```rust
let point = Point { x: 1, y: 2 };     // immutable — cannot modify fields
let mut point = Point { x: 1, y: 2 };  // mutable — can modify fields
```

The caller decides mutability, and the compiler enforces it. Making fields private with getters would:

1. **Not add real safety** — Rust already prevents mutation without `let mut`
2. **Reduce ergonomics** — `point.x()` instead of `point.x`, no destructuring
3. **Add complexity** — getter return type logic, Copy vs reference handling, Cow deref
4. **Diverge from Rust conventions** — idiomatic Rust structs use `pub` fields when there are no invariants to protect

The `is_mutable` distinction in the schema is meaningful for languages like C and Swift where mutability is a property of the field declaration. In Rust, mutability is a property of the *binding*, making field-level immutability enforcement redundant.

### Option C: Wrapper Type Pattern

Wrap immutable structs in a newtype that only exposes getters. This adds type system complexity and interferes with pattern matching. Not recommended.

## Recommendation: Option B — No Change Needed

Rust's ownership and borrowing system already provides compile-time mutation control that is more expressive than what field-level `const`/`let` provides in other languages. The `is_mutable` flag can be safely ignored.

### What This Means

- All struct fields continue to be emitted as `pub`
- Zero generator changes required
- Zero runtime changes required
- The `is_mutable` field on `StructDef` is acknowledged and intentionally not acted upon

### Why This Differs from C and Swift

| Language | Mutation controlled by | Schema's `is_mutable` maps to |
|---|---|---|
| C | Field qualifier (`const`) | `const` on struct members |
| Swift | Property declaration (`let`/`var`) | `let` vs `var` properties |
| Rust | Binding declaration (`let`/`let mut`) | N/A — caller already controls this |

## Cost-Benefit Analysis

### Benefits of No-Op

- **Zero implementation cost** — no code changes
- **Idiomatic Rust** — `pub` fields with caller-controlled mutability
- **Full ergonomics** — field access, pattern matching, destructuring all work
- **No maintenance burden** — nothing to keep in sync

### Costs of No-Op

- **Semantic gap** — the schema's intent (immutable fields) is not encoded in the generated Rust code
- **Cross-language inconsistency** — C and Swift plugins respect `is_mutable`, Rust does not

### Why the Costs Are Acceptable

The semantic gap is cosmetic, not functional. A Rust user who wants an immutable struct simply doesn't declare it `let mut`. The compiler enforces this more strictly than private-fields-with-getters would (private fields can still be mutated from within the same module). The cross-language inconsistency is justified because each language's idiomatic approach to immutability is different.

## Test Plan

No implementation → no tests needed. This plan documents that the `is_mutable` flag was evaluated and found unnecessary for the Rust plugin due to Rust's ownership model.
