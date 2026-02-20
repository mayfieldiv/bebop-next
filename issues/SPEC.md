# Spec Compliance Checklist

Exhaustive checklist of every spec item from the formal docs (`WIRE.md`, `GRAMMAR.md`, `RPC.md`, `DESCRIPTOR.md`, `PLUGINS.md`) and whitepaper against the Rust plugin + runtime implementation.

Legend: `[x]` = implemented & tested, `[/]` = partially done, `[-]` = N/A or out of scope, `[ ]` = not yet done

---

## Wire Format — Primitive Types (WIRE.md)

- [x] All multi-byte integers use little-endian byte order
- [x] `bool`: 1 byte, 0x00 = false, non-zero = true
- [x] `byte`: 1 byte, unsigned
- [x] `int8`: 1 byte, signed two's complement
- [x] `int16`: 2 bytes LE
- [x] `uint16`: 2 bytes LE
- [x] `int32`: 4 bytes LE
- [x] `uint32`: 4 bytes LE
- [x] `int64`: 8 bytes LE
- [x] `uint64`: 8 bytes LE
- [x] `int128`: 16 bytes, low 8 bytes first (LE word order)
- [x] `uint128`: 16 bytes, low 8 bytes first (LE word order)
- [x] `float16`: 2 bytes, IEEE 754 binary16 LE
- [x] `float32`: 4 bytes, IEEE 754 binary32 LE
- [x] `float64`: 8 bytes, IEEE 754 binary64 LE
- [x] `bfloat16`: 2 bytes, brain float LE
- [x] `uuid`: 16 bytes, RFC 4122 byte order (canonical, NOT Microsoft swapped)
- [x] `timestamp`: 12 bytes — `int64` seconds (offset 0) + `int32` nanos (offset 8)
- [x] `duration`: 12 bytes — `int64` seconds (offset 0) + `int32` nanos (offset 8)

## Wire Format — Strings (WIRE.md)

- [x] String wire layout: `uint32 length` + `UTF-8 content` + `NUL` (0x00)
- [x] Length prefix = byte count of UTF-8 content only (excludes NUL)
- [x] Total wire size = `4 + length + 1`
- [x] Runtime `write_string` writes NUL terminator
- [x] Runtime `read_string` reads and skips NUL terminator
- [x] Runtime `read_str` zero-copy path also reads/skips NUL
- [x] Empty string encodes as 5 bytes (`00 00 00 00 00`)

## Wire Format — Dynamic Arrays (WIRE.md)

- [x] `uint32` count prefix + elements in sequence, no padding
- [x] Runtime `read_array`/`write_array` with count prefix
- [x] Empty array encodes as 4 bytes (count = 0)
- [ ] Bulk scalar array optimization (per-element loop currently) → [[bulk-array-optimization]]

## Wire Format — Fixed Arrays (WIRE.md)

- [x] No length prefix; count known at compile time
- [x] Generated code handles fixed arrays with const generics `[T; N]`
- [/] Runtime only has `read_fixed_i32_array`/`write_fixed_i32_array` for i32; other types use general loop → [[fixed-array-generics]]

## Wire Format — Maps (WIRE.md)

- [x] `uint32` count prefix + key-value pairs in sequence
- [x] Runtime `read_map`/`write_map` with count prefix
- [x] Empty map encodes as 4 bytes (count = 0)
- [x] Generated code uses `HashMap<K, V>`

## Wire Format — Structs (WIRE.md)

- [x] Fields encoded in definition order
- [x] No tags, no length prefix, no padding
- [x] Empty structs encode as zero bytes
- [x] Nested structs encode inline
- [x] Fixed-size structs get `FIXED_ENCODED_SIZE` constant
- [x] Variable-size structs get computed `encoded_size()`
- [x] `new()` constructor with ergonomic params

## Wire Format — Messages (WIRE.md)

- [x] `uint32` length prefix + tagged fields + `0x00` end marker
- [x] Tags are 1 byte, range 1–255
- [x] Absent fields are not encoded
- [x] Unknown tags skipped during decode (forward compat)
- [x] All fields are `Option<T>` in generated code
- [x] Empty message = 5 bytes (`01 00 00 00 00`)
- [x] Self-referential fields wrapped in `Option<Box<T>>`
- [x] `#[derive(Default)]` for empty construction

## Wire Format — Unions (WIRE.md)

- [x] `uint32` length prefix + `uint8` discriminator + branch content
- [x] Discriminators range 0–255
- [x] Generated as Rust enum with one variant per branch
- [x] `Unknown(u8, Cow<'buf, [u8]>)` variant for forward compat
- [x] Inline struct branches encode as struct (positional, no tags)
- [x] Inline message branches encode as message (with own length prefix + tags)
- [x] Type-reference branches encode according to referenced type

## Wire Format — Nested / Recursive Types (WIRE.md)

- [x] Array of arrays: each inner has own count prefix
- [x] Array of strings: each string has own length prefix + NUL
- [x] Fixed array of dynamic arrays: no outer prefix, inner arrays have count prefixes
- [x] `TypeDescriptor` recursive self-reference (Box wrapping)

## Wire Format — Empty vs Null (WIRE.md)

- [x] Null values do not exist — all values encode normally
- [x] Empty string = 5 bytes
- [x] Empty byte array = 4 bytes
- [x] Empty struct = 0 bytes
- [x] Empty message = 5 bytes

## Wire Format — Size Limits (WIRE.md)

- [x] Fixed array max 65535 elements (compile-time check by bebopc)
- [x] String/array/map/message/union length = uint32 (max ~4GB)

## Wire Format — Size Calculations (WIRE.md)

- [x] `wire_size::string_size()` — accounts for 4 + len + 1
- [x] `wire_size::byte_array_size()` — 4 + len
- [x] `wire_size::array_size()` — 4 + sum of elem sizes
- [x] `wire_size::map_size()` — 4 + sum of entry sizes
- [x] `wire_size::tagged_size()` — 1 (tag) + elem size
- [x] `wire_size::WIRE_MESSAGE_BASE_SIZE` — 4 (length) + 1 (end marker) = 5
- [x] `FIXED_ENCODED_SIZE` on fixed structs and enums

## Wire Format — Design Properties (WIRE.md)

- [x] Fixed-width integers, no varints
- [x] Single-pass encoding/decoding
- [x] Zero-copy decoding via `Cow<'buf, str>` and `Cow<'buf, [u8]>`
- [x] Linear time complexity

---

## Schema Language — Enums (GRAMMAR.md)

- [x] Basic enum with named integer members
- [x] Custom base type (`enum Name : uint8`)
- [x] Default base type is `uint32` when unspecified
- [x] Valid base types: byte, int8, int16, uint16, int32, uint32, int64, uint64
- [x] Member values stored as `uint64`; sign-extended for signed base types
- [x] `@flags` decorator switches to bitflags generation
- [x] `TryFrom<base>` for decode validation
- [x] `From<Enum> for base` for encode
- [x] Doc comments on enums and members
- [x] `@deprecated` on enum members
- [-] Enum value expressions (bitwise ops, member references) — resolved by compiler, plugin sees final values

## Schema Language — Structs (GRAMMAR.md)

- [x] Immutable struct (default)
- [/] Mutable struct (`mut`) — generated code doesn't distinguish `mut` vs immutable in Rust (both are `pub` fields)
- [x] Positional field encoding (no tags)
- [x] Nested structs
- [x] Empty structs encode as zero bytes
- [x] `new()` constructor
- [x] `into_owned()` for borrowed types
- [x] `TypeOwned = Type<'static>` alias
- [x] `FIXED_ENCODED_SIZE` for fixed-size structs

## Schema Language — Messages (GRAMMAR.md)

- [x] Tagged fields with `uint8` index (1–255)
- [x] All fields optional (`Option<T>`)
- [x] Self-referential messages (`TreeNode { children: TreeNode[] }`)
- [x] Schema evolution: add/remove fields safely
- [x] Doc comments on messages and fields
- [x] `@deprecated` on messages and fields
- [/] Deprecated fields still encoded (C skips them) → [[deprecated-field-encoding]]

## Schema Language — Unions (GRAMMAR.md)

- [x] Discriminator byte (0–255)
- [x] Inline struct branches (positional)
- [x] Inline message branches (tagged)
- [x] Type-reference branches
- [x] `Unknown` variant for forward compat
- [x] Empty branches (zero-byte content)
- [x] `into_owned()` on all branches
- [x] Doc comments and `@deprecated` on branches

## Schema Language — Services (GRAMMAR.md, RPC.md)

- [ ] Service definition generation → [[services-rpc]]
- [ ] Unary method: `Method(Req): Res`
- [ ] Server stream: `Method(Req): stream Res`
- [ ] Client stream: `Method(stream Req): Res`
- [ ] Duplex stream: `Method(stream Req): stream Res`
- [ ] Service composition via `with`
- [ ] Method ID = MurmurHash3 of `/ServiceName/MethodName`
- [ ] Handler trait with async methods
- [ ] Client stub with typed RPC calls
- [ ] Router registration
- [ ] Batch accessor for batched calls

## Schema Language — Constants (GRAMMAR.md)

- [x] `bool` constants (`true`, `false`)
- [x] Integer constants with type suffix
- [x] Float constants including special values (`NaN`, `Inf`, `-Inf`)
- [x] `float16` / `bfloat16` constants via `from_f64_const()`
- [x] String constants (escaped literals)
- [x] UUID constants (byte array literal)
- [x] Byte array constants (`b"..."` prefix)
- [x] Timestamp constants (`(i64, i32)` tuple)
- [x] Duration constants (`(i64, i32)` tuple)

## Schema Language — Decorators (GRAMMAR.md, whitepaper)

- [x] `@deprecated` with optional message → `#[deprecated(note = "...")]`
- [x] `@flags` → bitflags newtype struct generation
- [-] Custom decorator definitions — compiler resolves; plugin gets `export_data` via descriptor
- [/] Decorator `export_data` — available in descriptor but not consumed by Rust generator (no known use case yet)

## Schema Language — Visibility (GRAMMAR.md, whitepaper)

- [ ] Top-level definitions default to exported → `pub` ✓ (but always pub)
- [ ] `local` definitions → should be `pub(crate)` → [[visibility-support]]
- [ ] Nested definitions default to local → not distinguished
- [ ] `export` on nested → not distinguished

## Schema Language — Type Aliases (GRAMMAR.md)

- [-] `uint8` = `byte` — resolved by compiler
- [-] `guid` = `uuid` — resolved by compiler
- [-] `half` = `float16` — resolved by compiler
- [-] `bf16` = `bfloat16` — resolved by compiler

## Schema Language — Nested Types (GRAMMAR.md)

- [x] Nested type definitions within structs/messages/unions
- [x] Processed from descriptor's `nested` field
- [x] Cross-module references via `use super::{stem}::*`
- [x] Fully-qualified name resolution via `defined_fqn`

## Schema Language — Identifiers & Naming (GRAMMAR.md)

- [x] PascalCase for type names (`to_pascal_case`)
- [x] snake_case for field names (`to_snake_case`)
- [x] SCREAMING_SNAKE_CASE for enum members and constants (`to_screaming_snake_case`)
- [x] PascalCase for enum variants from SCREAMING_CASE (`variant_name`)
- [x] Keyword escaping with `r#` prefix

---

## Plugin Protocol (PLUGINS.md)

- [x] Plugin executable named `bebopc-gen-rust`
- [x] Reads `CodeGeneratorRequest` from stdin (Bebop wire format)
- [x] Writes `CodeGeneratorResponse` to stdout (Bebop wire format)
- [x] Processes `schemas` from request
- [x] Generates one output file per schema
- [x] Schema errors → `error` field in response, exit 0
- [x] Plugin bugs → stderr, exit non-zero
- [x] Compiler version available via request
- [x] Plugin-specific options via `parameter` field
- [/] `files_to_generate` filtering — need to verify exact behavior
- [ ] Insertion point markers (`// @@bebop_insertion_point(...)`) → [[insertion-points]]
- [-] Windows binary mode for stdin/stdout — platform-specific, N/A on macOS/Linux

## Descriptor (DESCRIPTOR.md)

- [x] `DescriptorSet` with `schemas` array
- [x] Schemas in topological order (imports before importers)
- [x] Definitions in topological order within each schema
- [x] `DefinitionDescriptor` with kind, name, fqn, doc, visibility, decorators, nested
- [x] All `DefinitionKind` values handled: ENUM, STRUCT, MESSAGE, UNION, SERVICE, CONST
- [-] DECORATOR kind — not a generated type
- [x] `TypeDescriptor` recursive structure for all type kinds
- [x] All 19 scalar `TypeKind` values + 4 compound kinds (ARRAY, FIXED_ARRAY, MAP, DEFINED)
- [x] `EnumDef` with base_type, members, is_flags
- [x] `StructDef` with fields, is_mutable, fixed_size
- [x] `MessageDef` with fields
- [x] `FieldDescriptor` with name, type, index, decorators
- [x] `UnionDef` with branches
- [x] `UnionBranchDescriptor` with discriminator, inline_fqn / type_ref_fqn
- [x] `ServiceDef` with methods (parsed but generation is stub)
- [x] `MethodDescriptor` with name, request_type, response_type, method_type, id
- [x] `ConstDef` with type and value
- [x] `LiteralValue` all kinds: BOOL, INT, FLOAT, STRING, UUID
- [x] `DecoratorUsage` with fqn, args, export_data
- [x] `Visibility` enum (DEFAULT, EXPORT, LOCAL) — parsed but not acted on

---

## RPC Wire Protocol (RPC.md)

These are relevant when services are implemented → [[services-rpc]]

### Frame Format
- [ ] `FrameHeader`: 9 bytes — `uint32 length` + `byte flags` + `uint32 stream_id`
- [ ] `FrameFlags`: END_STREAM (0x01), ERROR (0x02), COMPRESSED (0x04), TRAILER (0x08)
- [ ] ERROR must combine with END_STREAM
- [ ] TRAILER must combine with END_STREAM
- [ ] Reserved flag bits: senders set to 0, receivers ignore

### Call Header
- [ ] `CallHeader` message: `method_id(1): uint32`, `deadline(2): timestamp`, `metadata(3): map[string,string]`
- [ ] Binary transports: CallHeader is first bytes on connection
- [ ] HTTP transports: CallHeader mapped to URL path + headers, not sent as bytes

### Status Codes
- [ ] `StatusCode` enum: OK(0) through UNAUTHENTICATED(16)
- [ ] Codes 0–16 align with gRPC
- [ ] Codes 17–255 application-defined

### Call Lifecycle — Unary
- [ ] Client: CallHeader → Frame[END_STREAM] with request
- [ ] Server: Frame with response → Frame[END_STREAM|TRAILER] (optional trailer)
- [ ] Error: Frame[END_STREAM|ERROR] with RpcError payload

### Call Lifecycle — Server Streaming
- [ ] Client: CallHeader → request[END_STREAM]
- [ ] Server: zero or more response frames → END_STREAM on last

### Call Lifecycle — Client Streaming
- [ ] Client: CallHeader → zero or more requests → END_STREAM on last
- [ ] Server: waits, then response + optional trailer
- [ ] Server MAY respond before client finishes (early termination)

### Call Lifecycle — Duplex Streaming
- [ ] Both sides send frames independently
- [ ] Either side signals END_STREAM independently

### Cancellation
- [ ] Transport-specific cancellation (close connection, RST_STREAM, etc.)
- [ ] Server detects cancellation, propagates to handlers
- [ ] Handlers should check `isCancelled` periodically
- [ ] No error frame sent after cancellation (client already gone)

### Metadata
- [ ] String key-value pairs
- [ ] Keys: ASCII lowercase, digits, hyphens, underscores
- [ ] `bebop-` prefix reserved for protocol use
- [ ] Request metadata in CallHeader.metadata
- [ ] Response metadata in TrailingMetadata or RpcError.metadata

### Deadlines
- [ ] Absolute timestamps, not relative durations
- [ ] Binary: `CallHeader.deadline` (Bebop timestamp, 12 bytes)
- [ ] HTTP: `bebop-deadline` header (decimal millisecond Unix timestamp)
- [ ] Already-passed deadline → DEADLINE_EXCEEDED without invoking handler
- [ ] Propagate to downstream: use earlier of propagated and local timeout

### Batching
- [ ] Method ID 1 reserved for batch calls
- [ ] `BatchRequest` with calls array
- [ ] `BatchCall` with call_id, method_id, payload, input_from
- [ ] `input_from >= 0`: pipe result from referenced call
- [ ] `input_from == -1`: use own payload
- [ ] Server validates all before executing
- [ ] Dependency graph → execute layers in parallel
- [ ] Failed call → dependents fail with INVALID_ARGUMENT
- [ ] Client-stream and duplex excluded from batching

### Service Discovery
- [ ] Method ID 0 reserved for discovery
- [ ] Request: `bebop.Empty`, Response: `DiscoveryResponse`
- [ ] Optional — UNIMPLEMENTED if not supported

### HTTP Transport Mapping
- [ ] `POST /{ServiceName}/{MethodName}`
- [ ] Content-Type: `application/bebop` (unary), `application/bebop+stream` (streaming)
- [ ] Unary: bare Bebop payloads, no framing
- [ ] Streaming: frames in body
- [ ] Status code mapping (Bebop → HTTP)
- [ ] Batch endpoint: `POST /_bebop/batch`

### Binary Transport
- [ ] CallHeader as first bytes on connection
- [ ] All data after header is frames
- [ ] WebSocket: one protocol element per binary message
- [ ] TCP/Unix: stream of concatenated frames
- [ ] Multiplexing: odd IDs = client-initiated, even = server-initiated

### Call Context
- [ ] Request metadata (read-only)
- [ ] Response metadata (mutable)
- [ ] Deadline
- [ ] Cancellation flag
- [ ] Transport-specific downcast

### Interceptors
- [ ] Wrap handler dispatch
- [ ] Registration order = execution order (first = outermost)
- [ ] Can short-circuit by throwing error

---

## Reflection (C plugin, Swift plugin — no formal spec)

- [ ] Per-type reflection metadata → [[reflection]]
- [ ] Type name, FQN, kind
- [ ] Field/member/branch descriptors
- [ ] Optional `no_reflection` flag

---

## Code Quality & Ergonomics (cross-cutting)

- [x] `#[allow(warnings)]` on generated code
- [x] `extern crate alloc` for no_std compatibility
- [x] `Cow<'buf, str>` for zero-copy strings
- [x] `Cow<'buf, [u8]>` for zero-copy byte arrays
- [x] Lifetime analysis pre-pass
- [x] `into_owned()` for borrowed → owned conversion
- [x] `TypeOwned = Type<'static>` aliases
- [x] `BebopDecodeOwned` marker trait
- [x] `no_std` + `alloc` feature runtime
- [x] `std` feature swaps hashbrown → std HashMap
- [x] `BebopFlags` trait with blanket encode/decode
- [x] Doc comments from schema `///`
- [x] `#[deprecated]` from `@deprecated`
- [x] Cross-module `use super::{stem}::*`
- [x] Self-hosting: descriptor/plugin types generated by the plugin itself
- [ ] `PartialEq`/`Eq`/`Hash` derives on non-float types → [[derive-traits]]
- [ ] Serde integration (feature-gated) → [[serde-integration]]
- [ ] Timestamp/Duration newtypes → [[temporal-newtypes]]
- [ ] UUID crate integration (feature-gated) → [[uuid-integration]]
- [x] Dead code in type_mapper.rs → [[dead-code-cleanup]]
- [ ] Rename `_cow` suffixed functions → [[rename-cow-functions]]
