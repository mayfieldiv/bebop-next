# Bebop Rust Plugin — Detailed Research Document

## 1. Overview

The Rust plugin (`bebopc-gen-rust`) is a **code generator plugin** for the [Bebop](https://github.com/6over3/bebop) serialization framework (next-generation, "edition 2026"). It reads a binary-encoded `CodeGeneratorRequest` from stdin, generates idiomatic Rust source code for each schema, and writes a binary-encoded `CodeGeneratorResponse` to stdout. The compiler (`bebopc`) invokes it as an external process.

The plugin is organized as a Cargo workspace with four crates:

| Crate                     | Path                 | Purpose                                            |
| ------------------------- | -------------------- | -------------------------------------------------- |
| `bebopc-gen-rust`         | `/` (root)           | The code generator binary                          |
| `bebop-runtime`           | `runtime/`           | Runtime library shipped with generated code        |
| `bebop-integration-tests` | `integration-tests/` | End-to-end tests against real `.bop` schemas       |
| `bebop-rust-benchmarks`   | `benchmarks/`        | Criterion benchmarks for encode/decode performance |

---

## 2. Architecture & Data Flow

```
bebopc compiler
    │
    │  stdin: CodeGeneratorRequest (Bebop binary)
    ▼
┌──────────────────────────────────────┐
│         bebopc-gen-rust (main.rs)    │
│                                      │
│  1. Read all bytes from stdin        │
│  2. Decode CodeGeneratorRequest      │
│  3. For each schema in               │
│     files_to_generate:               │
│     a. Run RustGenerator::generate() │
│     b. Emit .rs file content         │
│  4. Encode CodeGeneratorResponse     │
│  5. Write to stdout                  │
└──────────────────────────────────────┘
    │
    │  stdout: CodeGeneratorResponse (Bebop binary)
    ▼
bebopc compiler (writes .rs files to disk)
```

### 2.1 Plugin Protocol

The plugin protocol is defined by two Bebop schemas (`descriptor.bop` and `plugin.bop`) that are themselves generated into Rust types (`src/generated/descriptor.rs` and `src/generated/plugin.rs`). This is a **bootstrapping** arrangement — the plugin generates its own protocol types.

**Key protocol types:**

- `CodeGeneratorRequest`: contains `files_to_generate` (which `.bop` files to generate), `schemas` (full `SchemaDescriptor` array for type resolution), `compiler_version`, `parameter`, and `host_options`.
- `CodeGeneratorResponse`: contains `files` (list of `GeneratedFile` with name+content) and an optional `error` string.
- `SchemaDescriptor`: contains `path`, `edition`, `definitions` (array of `DefinitionDescriptor`), and `imports`.
- `DefinitionDescriptor`: the core type — has `kind` (enum/struct/message/union/const/service/decorator), `name`, `fqn` (fully-qualified name), `visibility`, `documentation`, `decorators`, nested definitions, and kind-specific sub-structs (`enum_def`, `struct_def`, `message_def`, `union_def`, `const_def`, `service_def`).

### 2.2 Build Integration

The plugin is built with CMake (`CMakeLists.txt`) in the larger bebop-next build system. CMake invokes `cargo build` and copies the resulting binary to the output directory. It's also installable via `install(PROGRAMS ...)`.

---

## 3. The Runtime Library (`bebop-runtime`)

### 3.1 Design Philosophy

The runtime is `#![no_std]`-compatible (uses `extern crate alloc`), with optional `std`, `chrono`, and `serde` features. This means generated code can run in embedded or WASM environments. When `std` is enabled, it uses `std::collections::HashMap`; otherwise it falls back to `hashbrown::HashMap`.

### 3.2 Wire Format

All multi-byte integers are **little-endian**. The wire format is:

| Type                                                    | Wire Format                                                   |
| ------------------------------------------------------- | ------------------------------------------------------------- |
| Scalars (bool, u8..u128, i8..i128, f16, bf16, f32, f64) | Raw little-endian bytes (1–16 bytes)                          |
| String                                                  | `u32 byte_count` + UTF-8 bytes + NUL terminator               |
| UUID                                                    | 16 raw bytes                                                  |
| Timestamp                                               | `i64 seconds` + `i32 nanos` (12 bytes)                        |
| Duration                                                | `i64 seconds` + `i32 nanos` (12 bytes)                        |
| Array                                                   | `u32 count` + elements                                        |
| Fixed Array                                             | Elements concatenated (no length prefix)                      |
| Map                                                     | `u32 count` + (key, value) pairs                              |
| Byte Array                                              | `u32 count` + raw bytes                                       |
| Struct                                                  | Fields concatenated in declaration order (no tags, no length) |
| Message                                                 | `u32 body_length` + tagged fields + `0x00` end marker         |
| Union                                                   | `u32 body_length` + `u8 discriminator` + branch payload       |

### 3.3 Key Types and Traits

**`BebopReader<'a>`** — A cursor-based zero-copy reader over a `&'a [u8]`. Provides:

- Scalar read methods (`read_bool`, `read_byte`, `read_i16`, ..., `read_f64`)
- Half-precision reads (`read_f16`, `read_bf16`) via the `half` crate
- String reads: `read_string()` (allocating) and `read_str()` (zero-copy `&'a str`)
- Byte array reads: `read_byte_array()` (allocating) and `read_byte_slice()` (zero-copy `&'a [u8]`)
- Collection reads: `read_array()`, `read_map()`, `read_fixed_array()`
- Message helpers: `read_message_length()`, `read_tag()`, `skip()`

**`BebopWriter`** — An accumulating `Vec<u8>` writer. Provides mirror write methods plus message helpers (`reserve_message_length`, `fill_message_length`, `write_tag`, `write_end_marker`).

**`BebopEncode`** — Trait with `encode(&self, writer)`, `encoded_size(&self) -> usize`, and convenience `to_bytes(&self) -> Vec<u8>`.

**`BebopDecode<'buf>`** — Trait with `decode(reader) -> Result<Self>` and convenience `from_bytes(buf) -> Result<Self>`. The lifetime `'buf` enables zero-copy deserialization.

**`BebopDecodeOwned`** — Marker trait for types decodable from any lifetime (`for<'buf> BebopDecode<'buf>`).

**`BebopFlags`** — Trait for generated `@flags` enum newtypes. Provides bitwise methods (`contains`, `intersects`, `insert`, `remove`, `toggle`, `empty`, `all`, `from_bits`, `from_bits_truncate`, etc.). Has blanket implementations for `BebopEncode` and `BebopDecode`.

**`BebopFlagBits`** — Trait for integer types usable as flag storage (`u8`, `i8`, `u16`, ..., `i64`).

**`FixedScalar`** — Trait for scalar types that can appear in fixed-size arrays. Implemented for all primitive numeric types including `f16` and `bf16`.

**`BebopTimestamp` / `BebopDuration`** — Simple `{seconds: i64, nanos: i32}` structs with conversions to `std::time::SystemTime`/`Duration` (behind `std` feature) and `chrono::DateTime<Utc>` (behind `chrono` feature).

### 3.4 Wire Size Helpers (`wire_size` module)

Constants and functions for computing encoded sizes without actually encoding:

- `WIRE_LEN_PREFIX_SIZE` (4), `WIRE_TAG_SIZE` (1), `WIRE_NUL_TERMINATOR_SIZE` (1)
- `WIRE_MESSAGE_BASE_SIZE` (5 = length prefix + end marker)
- `string_size(len)`, `byte_array_size(len)`, `array_size(items, elem_size)`, `map_size(map, entry_size)`, `tagged_size(payload_size)`

### 3.5 Serde Support

When the `serde` feature is enabled:

- All generated types get `#[derive(serde::Serialize, serde::Deserialize)]`
- Byte array fields use the `BebopBytes<'buf>` newtype which has built-in `Serialize`/`Deserialize` via `serde_bytes`, working in all contexts (direct fields, `Option`, `Vec`, `HashMap`)
- Unions use `#[serde(tag = "type", content = "value")]` for internally-tagged JSON representation
- The `Unknown` variant of unions is `#[serde(skip)]`

### 3.6 Dependencies

- `half` 2.7 — `f16` and `bf16` support
- `hashbrown` 0.15 — `no_std` HashMap
- `uuid` 1 — UUID type
- `serde` 1 + `serde_bytes` 0.11 (optional)
- `chrono` 0.4 (optional)

---

## 4. The Code Generator

### 4.1 Entry Point (`main.rs`)

1. Reads all bytes from stdin
2. Decodes `CodeGeneratorRequest` using `BebopDecode`
3. Converts to owned (`into_owned()`) to detach from the input buffer
4. Logs diagnostics to stderr (compiler version, parameters, schema count)
5. Parses `GeneratorOptions` from `host_options`
6. Builds a `LifetimeAnalysis` across all schemas
7. For each schema whose `path` is in `files_to_generate`:
   - Computes the output filename (e.g., `foo.bop` → `foo.rs`)
   - Determines sibling imports (other schemas being generated concurrently) by matching file stems
   - Calls `RustGenerator::generate(schema, sibling_imports, analysis)`
8. Encodes `CodeGeneratorResponse` and writes to stdout
9. On error, returns a response with the `error` field set

### 4.2 Generator Options

Currently one option: **`Visibility`** (from `host_options`):

- `"public"` (default) → all generated items are `pub`
- `"crate"` → all generated items are `pub(crate)`

Per-definition visibility can override via the schema's `visibility` field:

- `Visibility::Export` → `pub`
- `Visibility::Local` → `pub(crate)`
- `Visibility::Default` / `None` → uses the generator's default

### 4.3 Lifetime Analysis

`LifetimeAnalysis` is a critical pre-pass that determines, for every definition across all schemas:

1. **Whether the type needs a `'buf` lifetime parameter.** Types need lifetimes if they transitively contain:
   - `String` fields (→ `Cow<'buf, str>`)
   - `byte[]` fields (→ `Cow<'buf, [u8]>`)
   - Union types (always need lifetime due to `Unknown(u8, Cow<'buf, [u8]>)` variant)
   - References to other types that need lifetimes

2. **Whether the type can derive `Eq`.** Types with floating-point fields (transitively) cannot.

3. **Whether the type can derive `Hash`.** Types with floating-point OR map fields (transitively) cannot.

The analysis uses **fixpoint iteration** — it loops until no new types are marked as needing lifetimes. This handles forward references correctly (type A references type B which is defined later).

Enum FQNs are tracked separately since enums never need lifetimes.

### 4.4 Naming Conventions (`naming.rs`)

The generator converts Bebop naming conventions to idiomatic Rust:

| Conversion     | Function                                            | Example                           |
| -------------- | --------------------------------------------------- | --------------------------------- |
| Field names    | `field_name()` → `to_snake_case()` + keyword escape | `displayName` → `display_name`    |
| Type names     | `type_name()` → `to_pascal_case()` + keyword escape | `my_struct` → `MyStruct`          |
| Const names    | `const_name()` → `to_screaming_snake_case()`        | `httpStatusOk` → `HTTP_STATUS_OK` |
| Enum variants  | `variant_name()` → PascalCase from SCREAMING_SNAKE  | `SERVER_STREAM` → `ServerStream`  |
| FQN extraction | `fqn_to_type_name()` → last segment, PascalCase     | `bebop.TypeKind` → `TypeKind`     |

Rust keywords are escaped with `r#` prefix (e.g., `type` → `r#type`).

### 4.5 Type Mapping (`type_mapper.rs`)

This is the largest and most complex module. It handles the bidirectional mapping between Bebop types and Rust types.

#### Scalar Type Mapping

| Bebop Type            | Rust Type               | Cow Type                        | Size     |
| --------------------- | ----------------------- | ------------------------------- | -------- |
| `bool`                | `bool`                  | `bool`                          | 1        |
| `byte`                | `u8`                    | `u8`                            | 1        |
| `int8`                | `i8`                    | `i8`                            | 1        |
| `int16` / `uint16`    | `i16` / `u16`           | same                            | 2        |
| `int32` / `uint32`    | `i32` / `u32`           | same                            | 4        |
| `int64` / `uint64`    | `i64` / `u64`           | same                            | 8        |
| `int128` / `uint128`  | `i128` / `u128`         | same                            | 16       |
| `float16`             | `f16`                   | `f16`                           | 2        |
| `bfloat16`            | `bf16`                  | `bf16`                          | 2        |
| `float32` / `float64` | `f32` / `f64`           | same                            | 4/8      |
| `string`              | `alloc::string::String` | `alloc::borrow::Cow<'buf, str>` | variable |
| `guid`                | `::bebop_runtime::Uuid` | same                            | 16       |
| `timestamp`           | `BebopTimestamp`        | same                            | 12       |
| `duration`            | `BebopDuration`         | same                            | 12       |

#### Compound Type Mapping

| Bebop Type   | Rust Type (Cow)                                    | Rust Type (Owned)                 |
| ------------ | -------------------------------------------------- | --------------------------------- |
| `T[]`        | `alloc::vec::Vec<T>`                               | `alloc::vec::Vec<T>`              |
| `byte[]`     | `alloc::borrow::Cow<'buf, [u8]>`                   | `alloc::vec::Vec<u8>`             |
| `T[N]`       | `[T; N]`                                           | `[T; N]`                          |
| `map[K, V]`  | `::bebop_runtime::HashMap<K, V>`                   | same                              |
| Defined type | `TypeName<'buf>` (if needs lifetime) or `TypeName` | `TypeName<'static>` or `TypeName` |

#### Key Functions

- **`rust_type(td, analysis)`** — Maps a `TypeDescriptor` to its Cow-aware Rust type string for struct/message fields.
- **`rust_type_owned(td, analysis)`** — Maps to the owned type for `new()` constructor parameters.
- **`read_expression(td, reader, analysis)`** — Generates a read expression. Strings use `Cow::Borrowed(reader.read_str()?)`, byte arrays use `Cow::Borrowed(reader.read_byte_slice()?)`. Defined types call `TypeName::decode(reader)`.
- **`write_expression(td, value, writer, analysis)`** — Generates a write expression. Cow types deref naturally. Defined types call `.encode(writer)`.
- **`encoded_size_expression(td, value, analysis)`** — Generates a size computation expression using `wire::*` helpers.
- **`into_owned_expression(td, value, analysis)`** — Generates a borrowed → owned conversion. Cow types use `Cow::Owned(v.into_owned())`. Recursively handles nested collections.
- **`into_borrowed_expression(td, value, analysis)`** — Generates an owned → borrowed conversion for constructors. Strings become `Cow::Owned(value)`. Used in `new()` methods.
- **`fixed_encoded_size_expression(td)`** — Returns a compile-time const expression for fixed-size types.
- **`is_cow_field(td)` / `is_byte_array_cow_field(td)`** — Detect fields that need `Cow` wrapping.

### 4.6 Code Generation by Definition Kind

#### 4.6.1 Enums (`gen_enum.rs`)

Generates two forms depending on `@flags`:

**Regular enums:**

```rust
#[repr(u8)]  // or u16, u32, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
  Unknown = 0,
  Red = 1,
  Green = 2,
  Blue = 3,
}
```

Plus:

- `TryFrom<base_type>` impl for decoding (returns `DecodeError::InvalidEnum` for unknown values)
- `From<Name> for base_type` impl
- `FIXED_ENCODED_SIZE` constant
- `BebopEncode` impl (writes the discriminator value)
- `BebopDecode` impl (reads + `TryFrom`)
- Serde derive support
- Signed integer value formatting (e.g., casts through `u32 as i32` for negative values)

**Flags enums (`@flags`):**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Permissions(pub u8);

impl Permissions {
  pub const NONE: Self = Self(0);
  pub const READ: Self = Self(1);
  pub const WRITE: Self = Self(2);
  pub const EXECUTE: Self = Self(4);
  pub const ALL: Self = Self(7);
}
```

Plus:

- `BebopFlags` impl with computed `ALL_BITS`
- All bitwise operator impls (`BitOr`, `BitAnd`, `BitXor`, `Not`, `Sub`, and their `Assign` variants)
- Encode/decode via blanket `BebopFlags` impl

#### 4.6.2 Structs (`gen_struct.rs`)

Structs are fixed-layout, ordered field types. Generated code includes:

```rust
#[derive(Debug, Clone, PartialEq)]  // + Eq, Hash if possible
pub struct Person<'buf> {  // lifetime only if needed
  pub name: Cow<'buf, str>,
  pub age: u32,
}

pub type PersonOwned = Person<'static>;  // only if has lifetime

impl<'buf> Person<'buf> {
  // FIXED_ENCODED_SIZE (only for fixed-size structs)
  pub const FIXED_ENCODED_SIZE: usize = ...;

  pub fn new(name: impl Into<Cow<'buf, str>>, age: u32) -> Self { ... }
}
```

Plus:

- `into_owned()` method (only if has lifetime)
- `BebopEncode` impl — fields written in declaration order, no tags
- `BebopDecode` impl — fields read in declaration order
- `encoded_size()` — returns `FIXED_ENCODED_SIZE` for fixed-size, or computes dynamically
- `@@bebop_insertion_point(struct_scope:Name)` marker for user extensions

**Constructor ergonomics:**

- Cow fields (`String`, `byte[]`) accept `impl Into<Cow<'buf, str>>` or `impl Into<BebopBytes<'buf>>`, allowing both `&[u8]` and `Vec<u8>` arguments
- Non-Cow fields that need lifetime conversion use `into_borrowed_expression()` to handle `Vec<String>` → `Vec<Cow<'buf, str>>` etc.

**Byte array fields** use the `BebopBytes<'buf>` newtype which carries its own serde impls:

```rust
pub data: ::bebop_runtime::BebopBytes<'buf>,
```

#### 4.6.3 Messages (`gen_message.rs`)

Messages are extensible, tagged-field types. All fields are `Option<T>`.

```rust
#[derive(Debug, Clone, Default, PartialEq)]  // Default for messages
pub struct DrawCommand<'buf> {
  pub target: Option<Point>,
  pub color: Option<Color>,
  pub label: Option<Cow<'buf, str>>,
  pub thickness: Option<f32>,
}
```

**Self-referential field handling (`FieldWrap`):**

- `FieldWrap::Plain` — normal `Option<T>` for most fields
- `FieldWrap::Boxed` — `Option<Box<T>>` for direct self-references (field type FQN == own FQN)
- `FieldWrap::VecIndirect` — `Option<Vec<T>>` for array-of-self (Vec provides heap indirection)

**Wire encoding:**

```
[u32 body_length]
  [u8 tag1] [field1_data]
  [u8 tag2] [field2_data]
  ...
  [u8 0x00]  // end marker
```

- Only `Some` fields are written (tag + value)
- End marker `0x00` terminates the field list
- Body length is back-patched after encoding all fields

**Wire decoding:**

- Reads body length, then loops reading tags until end marker or position reaches end
- Unknown tags skip remaining bytes (forward compatibility)
- Scalar `Copy` fields use `if let Some(v) = ...` (by value); non-Copy use `ref v` (by reference)

**Deprecated fields note:** A comment documents a spec ambiguity — deprecated message fields are currently encoded/decoded like normal fields, matching the Swift plugin behavior. The C plugin skips them on encode but still decodes them.

#### 4.6.4 Unions (`gen_union.rs`)

Unions are discriminated tagged enums.

```rust
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "value"))]
pub enum Shape<'buf> {
  Point(Point),
  Pixel(Pixel),
  Label(TextLabel<'buf>),
  #[cfg_attr(feature = "serde", serde(skip))]
  Unknown(u8, Cow<'buf, [u8]>),  // forward-compatibility catch-all
}

pub type ShapeOwned = Shape<'static>;
```

**Unions always have a `'buf` lifetime** because of the `Unknown` variant which stores raw bytes as `Cow<'buf, [u8]>`.

**Wire format:**

```
[u32 body_length]
  [u8 discriminator]
  [branch_payload]
```

**Branch resolution:**

- `inline_fqn` — inline (anonymous) branch type
- `type_ref_fqn` — named reference to another type
- `name` — branch name (variant name)

**Encode:** Match on `self`, write discriminator byte, then encode inner type. Unknown variant writes discriminator + raw bytes.

**Decode:** Read discriminator, match known values to decode the appropriate type. Unknown discriminators read remaining bytes as raw `Cow::Borrowed` data.

**`into_owned()`:** Converts each variant's inner type. Branches with lifetimes call `.into_owned()`. Unknown variant converts `Cow::Borrowed` → `Cow::Owned`.

#### 4.6.5 Constants (`gen_const.rs`)

```rust
pub const HTTP_STATUS_OK: u32 = 200u32;
pub const EXAMPLE_CONST_STRING: &str = "hello \"world\"\nwith newlines";
pub const EXAMPLE_CONST_GUID: Uuid = Uuid::from_bytes([0xE2, 0x15, ...]);
```

Supports all literal kinds: `Bool`, `Int`, `Float`, `String`, `Uuid`, `Bytes`, `Timestamp`, `Duration`.

**Special handling:**

- Integer literals get type suffixes (`42i32`, `200u32`)
- Float literals handle NaN, Infinity, -Infinity as `f32::NAN`, `f32::INFINITY`, etc.
- Half-precision floats use `f16::from_f64_const()` / `bf16::from_f64_const()`
- Integer literals for half-precision types also use `from_f64_const()`
- String constants use `&str` (not `Cow`)
- Byte array constants use `&[u8]`
- Strings are escaped for Rust (`escape_default()`)
- Byte arrays are formatted as hex (`0xDE, 0xAD, 0xBE, 0xEF`)

#### 4.6.6 Services (`gen_service.rs`)

**Not yet implemented.** Currently emits a TODO comment listing the needed components:

- Service definition enum with Method sub-enum
- Handler trait (async fns for unary, server-stream, client-stream, duplex-stream)
- Client struct with typed RPC methods
- Batch accessor
- Router registration

The generator logs service method info (name, type, MurmurHash3 ID) to stderr.

### 4.7 Generated File Structure

Every generated `.rs` file has:

1. **Header:** "DO NOT EDIT" comment, source path, compiler version, edition
2. **Inner attributes:** `#![allow(warnings)]`, `#![no_implicit_prelude]`
3. **Extern crates:** `alloc`, `bebop_runtime`, `core`
4. **Use declarations:** Explicitly imported traits (`Into as _`, `IntoIterator as _`, `Iterator as _`), `vec!` macro, runtime types
5. **Cross-module imports:** `use super::{stem}::*;` for sibling schemas
6. **Insertion point:** `// @@bebop_insertion_point(imports)`
7. **Definitions:** All generated types
8. **EOF insertion point:** `// @@bebop_insertion_point(eof)`

### 4.8 `#![no_implicit_prelude]` Safety

Generated files use `#![no_implicit_prelude]` to avoid name collisions with user code. All standard library types are fully qualified:

- `::core::result::Result::Ok(...)` instead of `Ok(...)`
- `::core::option::Option::Some(...)` instead of `Some(...)`
- `::core::convert::TryFrom::try_from(...)` instead of `.try_into()`
- `alloc::borrow::Cow::Borrowed(...)` instead of `Cow::Borrowed(...)`
- `alloc::vec::Vec<T>` instead of `Vec<T>`
- `alloc::boxed::Box<T>` instead of `Box<T>`

The preamble imports only the traits needed as anonymous bindings (`Into as _`, `IntoIterator as _`, `Iterator as _`) to make method calls work.

### 4.9 Insertion Points

Generated code contains `@@bebop_insertion_point(...)` markers at strategic locations for post-generation customization:

- `imports` / `eof` — file level
- `struct_scope:Name` / `message_scope:Name` / `enum_scope:Name` / `union_scope:Name` — inside empty `impl` blocks
- `encode_start:Name` / `encode_end:Name` — around encode logic
- `decode_start:Name` / `decode_end:Name` — around decode logic
- `encode_switch:Name` / `decode_switch:Name` — inside union match arms

---

## 5. Zero-Copy Deserialization

The zero-copy strategy is one of the plugin's most sophisticated features:

1. **Strings** decode as `Cow::Borrowed(&'buf str)` — the decoded `&str` points directly into the input buffer.
2. **Byte arrays** decode as `Cow::Borrowed(&'buf [u8])` — the decoded slice points directly into the input buffer.
3. **Union Unknown variants** store raw bytes as `Cow::Borrowed(&'buf [u8])`.

All other types (integers, floats, UUIDs, timestamps, defined types) are copied during decode since they're small fixed-size values.

**Lifetime propagation:** If a struct/message contains _any_ field that transitively needs borrowed data, the entire type gets a `<'buf>` lifetime parameter. The `LifetimeAnalysis` determines this via fixpoint iteration.

**Owned conversion:** Every type with a lifetime has an `into_owned()` method and a `TypeOwned = Type<'static>` alias, allowing data to outlive the input buffer when needed.

**Constructor ergonomics:** `new()` constructors accept `impl Into<Cow<'buf, str>>` for string fields, so callers can pass `&str`, `String`, or `Cow<str>` without manual conversion.

---

## 6. Derive Trait Analysis

The generator computes which standard traits each type can derive:

| Trait       | Condition to derive                              |
| ----------- | ------------------------------------------------ |
| `Debug`     | Always                                           |
| `Clone`     | Always                                           |
| `Copy`      | Only on enums (non-flags)                        |
| `Default`   | Only on messages                                 |
| `PartialEq` | Always                                           |
| `Eq`        | No floating-point fields transitively            |
| `Hash`      | No floating-point AND no map fields transitively |

The analysis uses fixpoint iteration (same pattern as lifetime analysis) to handle forward references and recursive types.

**Enums** always get `Eq + Hash` since they're integer-backed. **Flags** also always get `Eq + Hash`.

---

## 7. Error Handling

### 7.1 Generator Errors (`error.rs`)

```rust
enum GeneratorError {
  Decode(DecodeError),     // Failed to decode the request
  Io(std::io::Error),     // Failed to read/write
  EmptyInput,             // No data on stdin
  MalformedDefinition(String),  // Schema data is structurally wrong
  MalformedType(String),        // Type descriptor is invalid
  InvalidOption(String),        // Bad host option value
}
```

On error, the generator returns a `CodeGeneratorResponse` with the `error` field set rather than panicking.

### 7.2 Runtime Errors

```rust
enum DecodeError {
  UnexpectedEof { needed: usize, available: usize },
  InvalidUtf8,
  InvalidEnum { type_name: &'static str, value: u64 },
}
```

All decode operations return `Result<T, DecodeError>`. The reader validates bounds before every read.

---

## 8. Cross-Schema Support

When multiple `.bop` files are compiled together, the generator handles cross-file type references:

1. **`LifetimeAnalysis::build_all()`** builds a unified analysis across all schemas, so a type in `a.bop` correctly knows whether a type from `b.bop` needs a lifetime.

2. **Sibling imports:** For each schema being generated, the generator computes which other schemas are also being generated (by matching file stems). It emits `use super::{stem}::*;` imports so cross-module references resolve.

3. **FQN resolution:** Type references use fully-qualified names (FQNs). The `fqn_to_type_name()` function extracts the simple name from a FQN (e.g., `"bebop.TypeKind"` → `"TypeKind"`).

---

## 9. Testing

### 9.1 Unit Tests (in `generator/mod.rs`)

Tests within the generator module construct `SchemaDescriptor` values programmatically and verify:

- File-level insertion points are emitted in correct order
- Type-specific insertion points exist for all definition kinds
- Derive traits are computed correctly (float → no Eq, map → no Hash)
- Recursive union types propagate trait restrictions correctly
- Forward references resolve correctly in lifetime analysis
- Constructor conversions work (e.g., `Vec<String>` → `Vec<Cow<str>>`)
- Visibility options (`pub`, `pub(crate)`) apply correctly
- Default visibility option from host options works

### 9.2 Integration Tests (`integration-tests/`)

End-to-end tests against a real `.bop` schema (`test_types.bop`) covering:

- Enum round-trip, fixed size, TryFrom, Into
- Flags round-trip, bitwise operations, from_bits, truncate
- Fixed-size struct round-trip, FIXED_ENCODED_SIZE
- Composite fixed structs (struct containing struct + enum)
- Variable-size structs (string, byte array fields)
- Zero-copy verification (asserting `Cow::Borrowed`)
- `into_owned()` verification (asserting `Cow::Owned`)
- Message default state, partial/full round-trip
- Messages with complex fields (string arrays, maps, flags)
- Union all branch types, unknown discriminator handling
- Messages with union arrays (Scene)
- Cow covariance (owned usable as borrowed)
- Unicode strings, large byte arrays
- Forward references, deprecated fields
- Integer-key maps, deep nested collections
- Temporal types (timestamp, duration)
- Serde JSON round-trip (behind feature flag)
- Half-precision types (f16, bf16) — scalars, arrays, fixed arrays, messages
- Constants (scalars, floats with special values, strings, UUIDs, half-precision)

### 9.3 `no_std` Integration Tests

A separate test file (`no_std_integration.rs`) verifies that generated code compiles and works without the standard library:

- Uses `#![no_std]` and `#![no_implicit_prelude]`
- Tests Point, Person, BinaryPayload, and UserProfile round-trips
- Validates `hashbrown::HashMap` usage

### 9.4 Test Runner Script (`test.sh`)

Runs the full validation pipeline:

1. `cargo fmt --all -- --check`
2. `cargo check` for all crates (including `--no-default-features` for `no_std`)
3. Unit tests for runtime
4. Integration tests (both `std` and `no_std` configurations)
5. `cargo clippy --workspace --all-targets --all-features -- -D warnings`

---

## 10. Benchmarks

Criterion benchmarks cover:

- **Scalar encode/decode** for every primitive type (bool through duration)
- **Struct/message encode/decode/encoded_size/into_owned** for:
  - `Person` (small string struct)
  - `TextSpan` (fixed enum struct)
  - `EmbeddingBf16` (1536-element bf16 vector — AI embedding use case)
  - `TreeNode` (recursive message, 32 levels deep)
  - `JsonValue` (recursive union — JSON-like tree)
  - `Document` (large message with Alice in Wonderland text, ~170KB)
  - `TensorShard` (large bf16 tensor — ML use case, 40×96×96 = 368,640 elements)
  - `InferenceResponse` (8 embeddings + timing — ML inference use case)

These benchmarks reflect realistic AI/ML workloads, suggesting the runtime is designed for high-performance model serving scenarios.

---

## 11. Notable Design Decisions

1. **Cow everywhere, not just Option fields.** Even struct fields use `Cow<'buf, str>` (not just message `Option<Cow<'buf, str>>`), enabling zero-copy for the entire type hierarchy.

2. **Unknown union variant.** Every union has an `Unknown(u8, Cow<'buf, [u8]>)` catch-all variant for forward compatibility — new union branches can be added without breaking existing decoders.

3. **`#![no_implicit_prelude]` for generated files.** Prevents accidental name collisions when generated code is included in user modules that may shadow standard library types.

4. **Fixpoint iteration for analysis.** Both lifetime analysis and trait derive analysis use iterative fixpoint computation, cleanly handling circular and forward references.

5. **`into_borrowed_expression` for constructors.** Constructors accept owned types and convert to borrowed form, making the API ergonomic while maintaining zero-copy internal representation.

6. **Messages derive `Default`.** All message fields are `Option<T>`, so `Default` gives an empty message. This is the idiomatic way to construct messages (create default, then set fields).

7. **Flags as newtype structs, not enums.** `@flags` types are `struct Permissions(pub u8)` with associated constants, not Rust enums. This allows arbitrary bit combinations that wouldn't be valid enum variants.

8. **Self-referential message fields use `Box`.** When a message field directly references its own type, the generator wraps it in `Box` to break the infinite-size cycle. Array-of-self fields use `Vec` which provides heap indirection naturally.

9. **Wire size computed independently.** `encoded_size()` is computed without encoding, using the `wire_size` module. This enables pre-allocation of output buffers (`BebopWriter::with_capacity`).

10. **Bootstrap generation.** The plugin generates its own protocol types (`descriptor.rs`, `plugin.rs`) from the Bebop schema definitions. The `generate.sh` script builds the plugin, runs it against the protocol schemas, and copies the output back into `src/generated/`.

---

## 12. Current Limitations & TODOs

1. **Services are not implemented.** The `gen_service.rs` file only emits TODO comments. Service generation would need: handler traits, client stubs, method enums, and router integration.

2. **Deprecated field handling is ambiguous.** The spec says deprecated message fields should be skipped during encoding, but the current implementation encodes them normally (matching the Swift plugin, diverging from the C plugin).

3. **No nested module generation.** Nested definitions (e.g., union inline branches) are flattened into the same module scope rather than generating nested Rust modules.

4. **Single output file per schema.** Each `.bop` file produces exactly one `.rs` file. There's no option to split generated code into multiple files.

5. **Fixed array decoding for non-scalars.** Non-scalar fixed arrays use a manual loop with `Default::default()` initialization, which requires `Default` on the element type.
