# Bebop Next-Generation Plugins: Comparative Analysis

A thorough comparison of the three code-generation plugins (C, Rust, Swift) for the Bebop "edition 2026" wire format.

---

## 1. High-Level Architecture

| Dimension                   | C (`bebopc-gen-c`)                                       | Rust (`bebopc-gen-rust`)                                        | Swift (`bebopc-gen-swift`)                                                  |
| --------------------------- | -------------------------------------------------------- | --------------------------------------------------------------- | --------------------------------------------------------------------------- |
| **Implementation language** | C                                                        | Rust                                                            | Swift                                                                       |
| **Source organization**     | Single file (`generator.c`, ~4660 lines)                 | Multi-module Cargo workspace (4 crates)                         | Multi-module SPM package (5 modules)                                        |
| **Plugin binary**           | Standalone executable, links `libbebop`                  | Standalone executable via `cargo build`                         | Standalone executable via `swift build`                                     |
| **Runtime library**         | C runtime (`bebop_wire.h`, `bebop.h`) shipped separately | `bebop-runtime` crate (published on crates.io)                  | `SwiftBebop` SPM library                                                    |
| **Build system**            | CMake (3 lines)                                          | CMake → Cargo                                                   | CMake → SwiftPM                                                             |
| **Protocol bootstrapping**  | Uses C descriptor APIs from `libbebop`                   | Self-bootstrapped (generates own `descriptor.rs` / `plugin.rs`) | Self-bootstrapped (generates own `descriptor.bb.swift` / `plugin.bb.swift`) |

All three follow the identical plugin protocol: read a binary `CodeGeneratorRequest` from stdin, generate code, write a binary `CodeGeneratorResponse` to stdout. The compiler invokes each as an external process.

---

## 2. Configuration Options

| Option                 | C                                                          | Rust                                      | Swift                                                    |
| ---------------------- | ---------------------------------------------------------- | ----------------------------------------- | -------------------------------------------------------- |
| **Visibility**         | Not supported                                              | `Visibility`: `public` (default), `crate` | `Visibility`: `public` (default), `package`, `internal`  |
| **Output mode**        | `output_mode`: `split` (default), `unity`, `single_header` | Single `.rs` file per schema (no option)  | Single `.bb.swift` file per schema (no option)           |
| **C standard**         | `c_standard`: `c99`, `c11` (default), `c23`                | N/A                                       | N/A                                                      |
| **Type prefix**        | `prefix`: any string (default `""`)                        | Not supported                             | Not supported                                            |
| **Reflection toggle**  | `no_reflection`: `true`/`false` (default `false`)          | Not supported (no reflection generated)   | Not supported (always generated)                         |
| **Service generation** | N/A (services are reflection-only)                         | N/A (services are TODO stubs)             | `Services`: `none`, `client`, `server`, `both` (default) |

**Key differences:**

- C is the only plugin with output-mode flexibility (split headers/sources, unity builds, or single-header libraries).
- C is the only plugin supporting a name prefix, essential in C where there are no namespaces or modules.
- Swift has the richest visibility control with three levels (`public`/`package`/`internal`) plus per-definition overrides from the schema's `visibility` field.
- Rust supports two levels (`pub`/`pub(crate)`) with per-definition overrides.
- Swift is the only plugin allowing selective service generation (client-only, server-only, both, or none).

---

## 3. Type System Mapping

### 3.1 Scalar Types

| Bebop Type  | C                 | Rust                        | Swift                    |
| ----------- | ----------------- | --------------------------- | ------------------------ |
| `bool`      | `bool`            | `bool`                      | `Bool`                   |
| `byte`      | `uint8_t`         | `u8`                        | `UInt8`                  |
| `int8`      | `int8_t`          | `i8`                        | `Int8`                   |
| `int16`     | `int16_t`         | `i16`                       | `Int16`                  |
| `uint16`    | `uint16_t`        | `u16`                       | `UInt16`                 |
| `int32`     | `int32_t`         | `i32`                       | `Int32`                  |
| `uint32`    | `uint32_t`        | `u32`                       | `UInt32`                 |
| `int64`     | `int64_t`         | `i64`                       | `Int64`                  |
| `uint64`    | `uint64_t`        | `u64`                       | `UInt64`                 |
| `int128`    | `Bebop_Int128`    | `i128`                      | `Int128`                 |
| `uint128`   | `Bebop_UInt128`   | `u128`                      | `UInt128`                |
| `float16`   | `Bebop_Float16`   | `f16` (half crate)          | `Float16` (stdlib)       |
| `float32`   | `float`           | `f32`                       | `Float`                  |
| `float64`   | `double`          | `f64`                       | `Double`                 |
| `bfloat16`  | `Bebop_BFloat16`  | `bf16` (half crate)         | `BFloat16` (custom impl) |
| `string`    | `Bebop_Str`       | `Cow<'buf, str>` / `String` | `String`                 |
| `uuid`      | `Bebop_UUID`      | `bebop_runtime::Uuid`       | `BebopUUID`              |
| `timestamp` | `Bebop_Timestamp` | `BebopTimestamp`            | `BebopTimestamp`         |
| `duration`  | `Bebop_Duration`  | `BebopDuration`             | `Duration`               |

**Notable divergences:**

- **Strings**: Rust uses `Cow<'buf, str>` for zero-copy borrowed deserialization; C uses a runtime string type (`Bebop_Str`); Swift uses `String` (always copied).
- **128-bit integers**: C uses custom runtime types; Rust and Swift use native language primitives.
- **BFloat16**: C uses a runtime type; Rust delegates to the `half` crate; Swift implements a full `BinaryFloatingPoint` conformance (~10 source files) with a C shim for hardware detection (ARM `__bf16`, x86 SSE2, fallback bit manipulation).
- **UUID**: C uses a runtime type; Rust wraps the `uuid` crate; Swift implements a Foundation-free `BebopUUID` type that is layout-compatible with `Foundation.UUID`.

### 3.2 Container Types

| Bebop Type          | C                                           | Rust                          | Swift                           |
| ------------------- | ------------------------------------------- | ----------------------------- | ------------------------------- |
| `array<T>`          | `T_Array` struct `{data, length, capacity}` | `Vec<T>`                      | `[T]` (Swift Array)             |
| `byte[]`            | Same as `array<byte>`                       | `Cow<'buf, [u8]>`             | `[UInt8]`                       |
| `fixed_array<T, N>` | `T field[N]` (C array)                      | `[T; N]` (Rust array)         | `InlineArray<N, T>` (Swift 6.2) |
| `map<K, V>`         | `Bebop_Map` (opaque runtime hash map)       | `HashMap<K, V>`               | `[K: V]` (Swift Dictionary)     |
| Defined type        | `Prefix_TypeName`                           | `TypeName<'buf>` / `TypeName` | `TypeName`                      |

**Notable divergences:**

- **Maps**: C uses an opaque runtime hash map with void pointers and type-specific hash/equality functions. Rust uses `std::collections::HashMap` or `hashbrown::HashMap` (no_std). Swift uses native Dictionary.
- **Fixed arrays**: Swift uses `InlineArray` (Swift 6.2 value-parameter generics) which lacks native `Equatable`/`Hashable`/`Codable` conformance, requiring the generator to emit custom helpers. C and Rust use native array types.
- **Byte arrays**: Rust uses `Cow<'buf, [u8]>` for zero-copy; C and Swift copy.

---

## 4. Definition Generation

### 4.1 Enums

| Aspect                    | C                                            | Rust                                                            | Swift                                             |
| ------------------------- | -------------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------- |
| **Language construct**    | `typedef enum { ... } Name;`                 | `enum Name { ... }` with `#[repr(base)]`                        | `struct Name: RawRepresentable`                   |
| **Unknown values**        | Implicit (C enums are integers)              | `TryFrom` fails with `InvalidEnum`                              | Round-trips safely (struct stores raw value)      |
| **Flags enums**           | Same as regular enum but with hex values     | Newtype struct with `BebopFlags` trait + bitwise operator impls | `struct Name: OptionSet`                          |
| **Member naming**         | `SCREAMING_SNAKE_CASE` with smart word dedup | `PascalCase` variants                                           | `camelCase` static lets                           |
| **Derive traits**         | N/A (C has no trait system)                  | `Debug, Clone, Copy, PartialEq, Eq, Hash`                       | `Sendable, Hashable, Codable` (via `BebopRecord`) |
| **Forward compatibility** | Safe (C integer cast)                        | Fails on unknown values                                         | Safe (struct stores any raw value)                |

**Critical difference in forward-compatibility strategy:** Rust enums reject unknown discriminator values during decoding (`TryFrom` returns `InvalidEnum`), while C and Swift both round-trip unknown values. Swift achieves this by modeling enums as structs with `RawRepresentable` rather than Swift `enum` cases. This is a deliberate design choice for wire-format forward compatibility.

### 4.2 Structs

| Aspect                      | C                                                                            | Rust                                                | Swift                                                 |
| --------------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------- | ----------------------------------------------------- |
| **Language construct**      | `struct Name { fields... };`                                                 | `struct Name<'buf> { fields... }`                   | `struct Name: BebopRecord { fields... }`              |
| **Field ordering**          | Reordered by alignment (descending) for padding minimization                 | Declaration order                                   | Declaration order                                     |
| **Mutability**              | `const` fields if schema says immutable; `BEBOP_WIRE_MUTPTR` cast for decode | All fields `pub` (no immutability concept)          | `let` (immutable) or `var` (if `isMutable` in schema) |
| **Empty structs**           | `BEBOP_WIRE_EMPTY_STRUCT` macro                                              | Empty struct (Rust allows zero-sized types)         | Empty struct                                          |
| **Fixed-size optimization** | `TYPE_FIXED_SIZE` macro, returned immediately                                | `FIXED_ENCODED_SIZE` const                          | Constant `encodedSize` if all fields are fixed        |
| **Constructor**             | N/A (C has no constructors)                                                  | `fn new(...)` with `impl Into<Cow<...>>` ergonomics | Memberwise `init(...)`                                |
| **Owned type alias**        | N/A                                                                          | `type NameOwned = Name<'static>` (if has lifetime)  | N/A (no lifetime parameter)                           |

**Key difference — field reordering:** Only the C plugin reorders struct fields by alignment to minimize padding. This is a C-specific optimization since Rust and Swift compilers can reorder fields internally. The C plugin uses `qsort` with alignment-descending, size-descending comparison.

### 4.3 Messages

| Aspect                        | C                                               | Rust                                     | Swift                                                        |
| ----------------------------- | ----------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------ |
| **Language construct**        | `struct` with `BEBOP_WIRE_OPT(T)` fields        | `struct` with `Option<T>` fields         | `final class` with `Optional` (`T?`) fields                  |
| **Value vs Reference**        | Value type (struct)                             | Value type (struct)                      | Reference type (class)                                       |
| **Default construction**      | Manual `BEBOP_WIRE_SET_NONE` per field          | `#[derive(Default)]`                     | `init(...)` with all `= nil`                                 |
| **Self-referential fields**   | Pointer (`Type *`) via `BEBOP_WIRE_OPT(Type *)` | `Option<Box<T>>`                         | Direct reference (class semantics give indirection for free) |
| **Array-of-self**             | Array struct (inherent pointer indirection)     | `Option<Vec<T>>` (Vec gives indirection) | Direct `[T]?` (class semantics)                              |
| **Thread safety**             | N/A (C has no concurrency model)                | `Send + Sync` if all fields are          | `@unchecked Sendable`                                        |
| **Deprecated field encoding** | **SKIPS** deprecated fields                     | **ENCODES** deprecated fields            | **ENCODES** deprecated fields                                |
| **Deprecated field decoding** | Decodes (still present in switch)               | Decodes normally                         | Decodes normally                                             |
| **Deprecated field sizing**   | **SKIPS** in size calculation                   | Includes in size                         | Includes in size                                             |

**Critical discrepancy — deprecated field handling:** The C plugin skips deprecated message fields during encoding and size calculation but still decodes them. The Rust and Swift plugins encode, decode, and size deprecated fields identically to normal fields. The Rust plugin includes an explicit comment documenting this divergence:

> _"The GRAMMAR.md spec says deprecated message fields should be skipped during encoding and decoding. The C plugin skips them on encode/size but still decodes them. The Swift plugin encodes them normally. This behavior should be revisited once the spec intent is clarified."_

This is a **wire-compatibility issue**: a C-encoded message missing deprecated fields could be correctly decoded by any plugin, but a Rust/Swift-encoded message that includes deprecated fields would also decode correctly in all three — the only difference is message size.

### 4.4 Unions

| Aspect                             | C                                                  | Rust                                                                     | Swift                                                       |
| ---------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------- |
| **Language construct**             | `struct` with discriminator + anonymous `union`    | `enum` with associated values                                            | `enum` with associated values                               |
| **Unknown discriminator (decode)** | Seeks to end, no error; discriminator stored as-is | Captures as `Unknown(u8, Cow<'buf, [u8]>)`                               | Captures as `unknown(discriminator: UInt8, data: [UInt8])`  |
| **Unknown discriminator (encode)** | Returns `BEBOP_WIRE_ERR_INVALID`                   | Re-encodes discriminator + raw bytes                                     | Re-encodes discriminator + raw bytes                        |
| **Forward compatibility**          | Partial (decode OK, encode rejected)               | Full (decode + re-encode preserves data)                                 | Full (decode + re-encode preserves data)                    |
| **Lifetime**                       | N/A                                                | Always `<'buf>` (Unknown stores `Cow<'buf, [u8]>`)                       | N/A (no lifetime concept in Swift)                          |
| **Serde support**                  | N/A                                                | `#[serde(tag = "type", content = "value")]`; Unknown is `#[serde(skip)]` | Custom `Codable` with `discriminator` + `value` coding keys |

**Key difference — unknown variant round-tripping:** Rust and Swift both preserve unknown union branches as raw bytes for full round-trip fidelity. C preserves the discriminator value but has no mechanism to store the raw payload data; decoding seeks past it, and encoding rejects unknown discriminators. This means C cannot act as a transparent proxy for unions with unknown branches.

### 4.5 Constants

| Aspect             | C                                   | Rust                                               | Swift                                      |
| ------------------ | ----------------------------------- | -------------------------------------------------- | ------------------------------------------ |
| **Integer**        | `const int64_t Name = 42LL;`        | `const NAME: i32 = 42i32;`                         | `let name: Int32 = 42`                     |
| **Float**          | `const double Name = 3.14;`         | `const NAME: f64 = 3.14f64;`                       | `let name: Double = 3.14`                  |
| **Bool**           | `const bool Name = true;`           | `const NAME: bool = true;`                         | `let name: Bool = true`                    |
| **String**         | `const char Name[] = "hello";`      | `const NAME: &str = "hello";`                      | `let name: String = "hello"`               |
| **UUID**           | `const Bebop_UUID Name = {{...}};`  | `const NAME: Uuid = Uuid::from_bytes([...]);`      | `let name = BebopUUID(uuidString: "...")!` |
| **Bytes**          | Static array + `Bebop_Bytes` struct | `const NAME: &[u8] = &[0x01, ...];`                | N/A (not observed in generator)            |
| **Timestamp**      | `{.seconds = X, .nanos = Y}`        | `BebopTimestamp { seconds: X, nanos: Y }`          | Supported                                  |
| **Duration**       | `{.seconds = X, .nanos = Y}`        | `BebopDuration { seconds: X, nanos: Y }`           | Supported                                  |
| **Float specials** | Direct literal                      | `f32::NAN`, `f32::INFINITY`, `f32::NEG_INFINITY`   | `.nan`, `.infinity`                        |
| **Half-precision** | Runtime type literal                | `f16::from_f64_const()` / `bf16::from_f64_const()` | Supported                                  |
| **Naming**         | Original case with prefix           | `SCREAMING_SNAKE_CASE`                             | `camelCase`                                |

### 4.6 Services

| Aspect                 | C                                           | Rust                                   | Swift                                                               |
| ---------------------- | ------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------- |
| **Status**             | Reflection-only (metadata emitted, no code) | Not implemented (TODO stubs)           | **Fully implemented**                                               |
| **Service definition** | Reflection descriptor only                  | TODO comment listing needed components | Full service enum with Method sub-enum, MurmurHash3 IDs             |
| **Server handler**     | None                                        | None                                   | `protocol {Name}Handler: BebopHandler` with typed method signatures |
| **Client stub**        | None                                        | None                                   | `struct {Name}Client<C: BebopChannel>` with typed RPC methods       |
| **Batch support**      | None                                        | None                                   | Full batch accessor with dependency DAG, forwarding                 |
| **Streaming**          | None                                        | None                                   | All 4 patterns: unary, server-stream, client-stream, duplex         |
| **Router**             | None                                        | None                                   | `BebopRouter` + `BebopRouterBuilder` with interceptor chain         |
| **Discovery**          | None                                        | None                                   | Reserved method ID 0 for service reflection                         |
| **RPC framing**        | None                                        | None                                   | 9-byte frame header (length + flags + streamId)                     |
| **Deadline support**   | None                                        | None                                   | `BebopTimestamp+Deadline` with `clock_gettime(CLOCK_REALTIME)`      |

**Swift is the only plugin with a complete RPC framework.** It includes client stubs, server handlers, a router with interceptor chains, batch execution with dependency resolution, streaming support for all four RPC patterns, framing, discovery, deadlines, and cancellation.

---

## 5. Zero-Copy and Performance Strategies

### 5.1 Zero-Copy Deserialization

| Strategy               | C                                                                     | Rust                                            | Swift                                        |
| ---------------------- | --------------------------------------------------------------------- | ----------------------------------------------- | -------------------------------------------- |
| **Primitive arrays**   | Direct pointer into read buffer (`capacity=0` sentinel marks as view) | Copied (no zero-copy for primitive arrays)      | memcpy bulk read (`readLengthPrefixedArray`) |
| **Strings**            | Runtime string type (unclear if zero-copy)                            | `Cow::Borrowed(&'buf str)` — zero-copy          | Copied (`String(decoding:as: UTF8.self)`)    |
| **Byte arrays**        | Same as primitive arrays                                              | `Cow::Borrowed(&'buf [u8])` — zero-copy         | Copied                                       |
| **Union unknown data** | Seeked past (not preserved)                                           | `Cow::Borrowed(&'buf [u8])` — zero-copy         | Copied into `[UInt8]`                        |
| **Lifetime tracking**  | N/A (C has no lifetime system)                                        | Full `LifetimeAnalysis` with fixpoint iteration | N/A (Swift has ARC, no lifetimes)            |

**Key insight:** Each plugin optimizes for its language's strengths:

- **C** achieves zero-copy for primitive arrays by pointing directly into the read buffer, using a `capacity=0` sentinel to distinguish views from owned data.
- **Rust** achieves zero-copy for strings and byte arrays through `Cow<'buf, T>` and lifetime-parameterized types, with a sophisticated fixpoint analysis to propagate lifetime requirements.
- **Swift** relies on bulk memcpy for scalar arrays but does not achieve true zero-copy for any type; it compensates with aggressive inlining and compiler optimizations.

### 5.2 Performance Annotations

| Technique                   | C                                                                     | Rust                         | Swift                                                                                   |
| --------------------------- | --------------------------------------------------------------------- | ---------------------------- | --------------------------------------------------------------------------------------- |
| **Branch prediction hints** | `BEBOP_WIRE_UNLIKELY()` on error checks                               | Not used                     | `_fastPath` / `_slowPath`                                                               |
| **Function attributes**     | `BEBOP_WIRE_HOT` (encode/decode), `BEBOP_WIRE_PURE` (size)            | Not used (relies on LTO)     | `@inlinable @inline(__always)` on all reader/writer methods                             |
| **Cache prefetch**          | `BEBOP_WIRE_PREFETCH_W/R` with calculated distance (~128 bytes ahead) | Not used                     | Not used                                                                                |
| **Bulk operations**         | `SetXxxArray` / `GetFixedXxxArray` (single memcpy)                    | Not used (per-element loops) | `readLengthPrefixedArray` / `writeLengthPrefixedArray` (memcpy for `BebopScalar` types) |
| **Non-copyable writer**     | N/A                                                                   | N/A (Vec-based, copyable)    | `~Copyable` writer prevents accidental copies                                           |
| **Wrapping arithmetic**     | N/A                                                                   | Not used                     | `&+`, `&*=` for offset/capacity arithmetic                                              |

**C is the most aggressively optimized at the generated-code level**, with cache prefetch hints, branch prediction macros, function hot/pure attributes, and bulk array operations. Swift compensates at the runtime level with aggressive inlining and memcpy-based bulk reads for scalar arrays. Rust relies primarily on LLVM optimization passes and LTO.

### 5.3 Struct Layout Optimization

Only the C plugin reorders struct fields by alignment (8-byte → 4-byte → 2-byte → 1-byte) to minimize padding. Rust and Swift emit fields in declaration order, relying on their respective compilers' internal layout optimization (Rust's default layout is unspecified and may reorder; Swift structs may not be reordered).

---

## 6. Memory Management

| Aspect                | C                                                                                        | Rust                                         | Swift                                                  |
| --------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------------ |
| **Allocation model**  | `Bebop_WireCtx_Alloc` — caller-provided context manages all decode allocations in a pool | Standard allocator (`Vec`, `Box`, `HashMap`) | ARC + `malloc`/`realloc` for `BebopWriter`             |
| **Writer**            | Runtime-provided `Bebop_Writer`                                                          | `BebopWriter` (wraps `Vec<u8>`)              | `BebopWriter` (`~Copyable`, manual `malloc`/`realloc`) |
| **Growth strategy**   | Runtime-managed                                                                          | `Vec` default (amortized doubling)           | Explicit doubling (`capacity &*= 2`)                   |
| **View vs owned**     | `capacity=0` sentinel                                                                    | `Cow<'buf, T>` (Borrowed vs Owned)           | Always owned                                           |
| **Cleanup**           | Caller frees `WireCtx`                                                                   | RAII (Drop)                                  | ARC + `deinit` for `~Copyable` writer                  |
| **Custom allocators** | Possible via `bebop_host_allocator_t`                                                    | Not supported (uses global allocator)        | Not supported                                          |

---

## 7. Reflection and Metadata

| Aspect                    | C                                                                                    | Rust | Swift                                                       |
| ------------------------- | ------------------------------------------------------------------------------------ | ---- | ----------------------------------------------------------- |
| **Runtime reflection**    | Full — `BebopReflection_DefinitionDescriptor` with field offsets, sizeof, type trees | None | `BebopReflectable` protocol with `BebopTypeReflection`      |
| **Type info dispatch**    | `Bebop_TypeInfo` — function pointers for `size_fn`, `encode_fn`, `decode_fn`         | None | N/A (protocol-based dispatch)                               |
| **Toggle**                | Optional via `no_reflection`                                                         | N/A  | Always generated                                            |
| **Field offsets**         | Yes (`offsetof(Type, field)`)                                                        | N/A  | No                                                          |
| **sizeof**                | Yes (struct/message/union sizeof)                                                    | N/A  | No                                                          |
| **Service reflection**    | Method descriptors with types and streaming kind                                     | N/A  | Full service/method reflection via `BebopServiceDefinition` |
| **Type-erased container** | N/A                                                                                  | N/A  | `BebopAny` — pack/unpack with type URL checking             |

C provides the deepest reflection with C-level `offsetof` and `sizeof` metadata plus dynamic dispatch function pointers. Swift provides protocol-based reflection with type name/FQN/kind/field metadata. Rust generates no reflection metadata.

---

## 8. Naming Conventions

| Aspect                        | C                                            | Rust                                              | Swift                                 |
| ----------------------------- | -------------------------------------------- | ------------------------------------------------- | ------------------------------------- |
| **Type names**                | `Prefix_PascalCase` (FQN dots → underscores) | `PascalCase` (last FQN segment)                   | `PascalCase` (last FQN segment)       |
| **Field names**               | Preserved as-is (keyword `_` suffix)         | `snake_case` (keyword `r#` prefix)                | `camelCase` (keyword backtick escape) |
| **Enum members**              | `SCREAMING_SNAKE_CASE` with word dedup       | `PascalCase` variants                             | `camelCase` static lets               |
| **Constants**                 | Original case with prefix                    | `SCREAMING_SNAKE_CASE`                            | `camelCase`                           |
| **Keyword safety**            | 56 C/C11/C23 keywords; appends `_`           | Rust keywords; prepends `r#`                      | 50 Swift keywords; wraps in backticks |
| **Include guard**             | `SCREAMING_SNAKE_CASE_H_`                    | N/A (Rust modules)                                | N/A (Swift modules)                   |
| **Name collision prevention** | `#ifndef` guards, type set dedup             | `#![no_implicit_prelude]` + fully qualified paths | `import SwiftBebop`                   |

**Rust's `#![no_implicit_prelude]`** is a unique safety measure: generated files explicitly opt out of the standard prelude and fully qualify every stdlib type (`::core::result::Result::Ok(...)`, `alloc::vec::Vec<T>`, etc.). This prevents name collisions when generated code is included in user modules that shadow standard types.

---

## 9. Error Handling

| Aspect               | C                                                        | Rust                                                               | Swift                                             |
| -------------------- | -------------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------- |
| **Error type**       | `Bebop_WireResult` enum (`OK`, `ERR_OOM`, `ERR_INVALID`) | `DecodeError` enum (`UnexpectedEof`, `InvalidUtf8`, `InvalidEnum`) | `BebopDecodingError` enum (`unexpectedEndOfData`) |
| **Propagation**      | `if (UNLIKELY(r != OK)) return r;`                       | `?` operator (idiomatic Rust)                                      | `try` / `throws` (idiomatic Swift)                |
| **Generator errors** | Sticky `ctx->error` string                               | `GeneratorError` enum returned in response                         | Error response via `ResponseBuilder`              |
| **OOM handling**     | Explicit `ERR_OOM` returns                               | Panic (Rust default)                                               | N/A (Swift handles via ARC)                       |

---

## 10. Testing

| Aspect                | C                                                      | Rust                                                                                  | Swift                                                                                 |
| --------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| **Unit tests**        | None in generator (likely tested at integration level) | In-module tests for lifetime analysis, trait derivation, visibility, insertion points | ~60 snapshot tests for generated code                                                 |
| **Integration tests** | Not in plugin directory                                | Full round-trip tests (`integration-tests/` crate) with 50+ test functions            | ~45 runtime tests + 35 plugin tests                                                   |
| **RPC tests**         | N/A                                                    | N/A                                                                                   | ~20 test files covering router, interceptors, batch, streaming, discovery, end-to-end |
| **no_std tests**      | N/A                                                    | Dedicated `no_std_integration.rs`                                                     | N/A                                                                                   |
| **Benchmarks**        | N/A                                                    | Criterion benchmarks (AI/ML workloads: embeddings, tensors, inference)                | N/A                                                                                   |
| **Lint/format**       | N/A                                                    | `cargo fmt --check` + `cargo clippy -D warnings`                                      | N/A                                                                                   |
| **Test runner**       | N/A                                                    | `test.sh` (format → check → test → clippy)                                            | `swift test`                                                                          |

Rust has the most structured test pipeline with explicit format checking, multi-configuration testing (std + no_std), clippy linting, and performance benchmarks. Swift has the broadest test coverage due to the RPC framework. C has no in-tree tests.

---

## 11. Platform and Ecosystem Support

| Aspect                | C                                                                         | Rust                                                            | Swift                                                                      |
| --------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------- |
| **no_std / embedded** | Possible (minimal deps, context-based alloc) but not explicitly supported | Full `#![no_std]` support with `hashbrown` fallback, tested     | Not supported                                                              |
| **Platform targets**  | Windows (`_setmode`), Linux, macOS                                        | All Rust tier-1 targets + WASM                                  | macOS 26+, iOS 26+, etc. (Apple platforms)                                 |
| **C++ interop**       | `extern "C"` guards on all generated headers                              | N/A                                                             | N/A                                                                        |
| **Serde support**     | N/A                                                                       | Optional `serde` feature with `Serialize`/`Deserialize` derives | Built-in `Codable` conformance on all types                                |
| **Foundation bridge** | N/A                                                                       | N/A                                                             | Separate `SwiftBebopFoundation` module (`Date`, `UUID`, `CGFloat`, `Data`) |
| **IDE integration**   | N/A                                                                       | N/A                                                             | SPM Build Tool Plugin + Xcode Command Plugin (format-bop)                  |
| **Chrono support**    | N/A                                                                       | Optional `chrono` feature for `DateTime<Utc>`                   | Foundation `Date` via bridge module                                        |

---

## 12. Generated Code Extensibility

All three plugins emit insertion points — specially formatted comments that downstream tools or users can search for to inject custom code.

| Insertion Point                | C                          | Rust                     | Swift |
| ------------------------------ | -------------------------- | ------------------------ | ----- |
| `imports` / `includes`         | Yes                        | Yes                      | Yes   |
| `forward_declarations`         | Yes                        | No                       | No    |
| `declarations` / `definitions` | Yes                        | No                       | No    |
| `eof`                          | Yes                        | Yes                      | Yes   |
| `struct_scope:Name`            | Yes (inside struct body)   | Yes (empty `impl` block) | Yes   |
| `message_scope:Name`           | No (structs cover this)    | Yes                      | Yes   |
| `enum_scope:Name`              | No                         | Yes                      | Yes   |
| `union_scope:Name`             | No                         | Yes                      | Yes   |
| `encode_start/end:Name`        | Yes                        | Yes                      | Yes   |
| `decode_start/end:Name`        | Yes                        | Yes                      | Yes   |
| `encode_switch:Name`           | Yes (union encode)         | Yes                      | Yes   |
| `decode_switch:Name`           | Yes (union/message decode) | Yes                      | Yes   |

C has the most header-specific insertion points (`forward_declarations`, `declarations`, `definitions`) because C's compilation model requires careful ordering. Rust and Swift have more type-scoped insertion points because their module systems handle ordering automatically.

---

## 13. Wire Format Compatibility

All three plugins target the same "edition 2026" wire format with identical binary encoding:

| Format Element  | All Three Plugins                                                        |
| --------------- | ------------------------------------------------------------------------ |
| Byte order      | Little-endian                                                            |
| String encoding | `u32 length` + UTF-8 bytes + NUL terminator                              |
| Struct          | Fields concatenated in declaration order, no tags                        |
| Message         | `u32 body_length` + tagged fields (`u8 tag` + value) + `0x00` end marker |
| Union           | `u32 body_length` + `u8 discriminator` + branch payload                  |
| Array           | `u32 count` + elements                                                   |
| Fixed Array     | Elements concatenated, no length prefix                                  |
| Map             | `u32 count` + key-value pairs                                            |

**Wire interoperability is guaranteed** for all shared types across the three plugins, with the exception of deprecated message fields (see Section 4.3). A message encoded by the C plugin (which skips deprecated fields) will decode correctly in all three plugins. A message encoded by Rust or Swift (which includes deprecated fields) will also decode correctly in all three, but the wire representation will be larger.

---

## 14. Maturity and Completeness Summary

| Feature       | C                   | Rust                  | Swift                                                          |
| ------------- | ------------------- | --------------------- | -------------------------------------------------------------- |
| Structs       | Complete            | Complete              | Complete                                                       |
| Messages      | Complete            | Complete              | Complete                                                       |
| Unions        | Complete            | Complete              | Complete                                                       |
| Enums         | Complete            | Complete              | Complete                                                       |
| Flags         | Complete            | Complete              | Complete                                                       |
| Constants     | Complete            | Complete              | Complete                                                       |
| Services      | Reflection only     | TODO stubs            | **Complete** (client + server + batch + streaming + discovery) |
| Reflection    | Full (toggle-able)  | None                  | Moderate (protocol-based)                                      |
| Zero-copy     | Primitive arrays    | Strings + byte arrays | None (bulk memcpy for scalars)                                 |
| no_std        | Possible (untested) | Full support (tested) | Not supported                                                  |
| RPC framework | None                | None                  | Complete                                                       |
| Serde/Codable | N/A                 | Optional feature      | Built-in                                                       |
| IDE plugins   | None                | None                  | SPM build + command plugins                                    |
| Benchmarks    | None                | Criterion suite       | None                                                           |
| In-tree tests | None                | Comprehensive         | Comprehensive                                                  |

---

## 15. Key Discrepancies and Open Questions

### 15.1 Deprecated Field Handling (Wire Incompatibility Risk)

The spec says deprecated message fields should be skipped during encoding. The C plugin follows this; Rust and Swift do not. This creates a subtle wire-level difference: messages encoded by different plugins may differ in size and content for schemas with deprecated fields. While all three can decode either form, this inconsistency should be resolved.

### 15.2 Unknown Union Encoding (Behavioral Asymmetry)

The C plugin rejects encoding unknown union discriminators (`BEBOP_WIRE_ERR_INVALID`), while Rust and Swift re-encode unknown variants with their preserved raw bytes. This means only Rust and Swift can act as transparent proxies that round-trip unknown union branches.

### 15.3 Enum Forward Compatibility

Rust enums reject unknown discriminator values during decoding (`TryFrom` fails). C and Swift both accept unknown values. This means Rust decoders will fail when encountering enum values added in newer schema versions, while C and Swift will round-trip them.

### 15.4 Service Generation Gap

Only Swift has a complete service/RPC implementation. This is the largest feature gap between the plugins. Rust has documented the needed components but has not implemented them. C only generates reflection metadata for services.

### 15.5 Reflection Parity

C has the deepest reflection (field offsets, sizeof, dynamic dispatch function pointers). Swift has moderate protocol-based reflection. Rust has none. This matters for frameworks that need runtime type introspection.
