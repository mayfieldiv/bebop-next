# Bebop Swift Plugin - Comprehensive Research Document

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Package Structure](#package-structure)
4. [Code Generator (`bebopc-gen-swift`)](#code-generator-bebopc-gen-swift)
5. [Runtime Library (`SwiftBebop`)](#runtime-library-swiftbebop)
6. [Wire Format](#wire-format)
7. [Type Mapping](#type-mapping)
8. [Code Generation Per Definition Kind](#code-generation-per-definition-kind)
9. [RPC Framework](#rpc-framework)
10. [BFloat16 Implementation](#bfloat16-implementation)
11. [Foundation Interoperability (`SwiftBebopFoundation`)](#foundation-interoperability-swiftbebopfoundation)
12. [Plugin Protocol (`BebopPlugin`)](#plugin-protocol-bebopplugin)
13. [SPM Build & Command Plugins](#spm-build--command-plugins)
14. [Build System Integration](#build-system-integration)
15. [Testing Strategy](#testing-strategy)
16. [Design Decisions & Notable Patterns](#design-decisions--notable-patterns)

---

## Overview

The Swift plugin for bebop-next is a complete code generation and runtime system that translates `.bop` schema files into type-safe Swift code and provides a high-performance binary serialization runtime. It is designed for the "edition 2026" of the Bebop wire format and requires **Swift 6.2+** (the package targets macOS 26, iOS 26, etc.).

The plugin consists of five modules:

| Module                                        | Kind        | Purpose                                                            |
| --------------------------------------------- | ----------- | ------------------------------------------------------------------ |
| `SwiftBebop`                                  | Library     | Runtime: reader, writer, protocols, RPC framework, BFloat16        |
| `SwiftBebopFoundation`                        | Library     | Bridges to Foundation (`Date`, `UUID`, `CGFloat`, `Data`)          |
| `BebopPlugin`                                 | Library     | Descriptor types + compatibility layer for compiler communication  |
| `bebopc-gen-swift`                            | Executable  | Code generator invoked by `bebopc` as a plugin                     |
| `BebopBuildToolPlugin` / `BebopCommandPlugin` | SPM Plugins | Xcode/SPM integration for automatic code generation and formatting |

---

## Architecture

```
bebopc (compiler)
  │
  ├── stdin: CodeGeneratorRequest (Bebop wire format)
  │
  └── bebopc-gen-swift (plugin executable)
        │
        ├── Decodes PluginRequest via BebopPlugin
        ├── Iterates schemas, generating Swift code per definition
        ├── Encodes CodeGeneratorResponse via ResponseBuilder
        │
        └── stdout: CodeGeneratorResponse (Bebop wire format)
```

The generated `.bb.swift` files depend on the `SwiftBebop` runtime library at compile time and runtime. The generated code conforms to `BebopRecord`, which provides `decode(from:)`, `encode(to:)`, and `encodedSize` for zero-allocation wire-format serialization.

---

## Package Structure

Defined in the root `Package.swift` (swift-tools-version: 6.2):

```
plugins/swift/
├── Sources/
│   ├── CBFloat16/               # C shim for bf16 ↔ float conversion
│   │   ├── include/cbfloat16.h  # Inline conversion functions + native __bf16 detection
│   │   └── CBFloat16.c          # Stub (all symbols in header)
│   ├── SwiftBebop/              # Core runtime library
│   │   ├── BebopReader.swift    # Wire-format deserialization
│   │   ├── BebopWriter.swift    # Wire-format serialization (~Copyable, malloc-backed)
│   │   ├── BebopRecord.swift    # Central protocol for generated types
│   │   ├── BebopScalar.swift    # Marker protocol for memcpy-safe types
│   │   ├── BebopError.swift     # Decoding error enum
│   │   ├── BebopTimestamp.swift  # 12-byte timestamp (sec + nsec)
│   │   ├── BebopUUID.swift      # Foundation-free 16-byte UUID
│   │   ├── BebopAny.swift       # Type-erased record (type URL + raw bytes)
│   │   ├── BebopEmpty.swift     # Zero-field placeholder
│   │   ├── Reflection.swift     # Compile-time type metadata
│   │   ├── BFloat16/            # Full BinaryFloatingPoint impl (10 files)
│   │   └── RPC/                 # Complete RPC framework
│   │       ├── Client/          # Channel, Batch, CallRef, DiscoveryClient
│   │       ├── Server/          # Handler, Interceptor, ServiceDefinition, CallContext
│   │       ├── Router/          # BebopRouter, Builder, Batch dispatch, Interceptor chain
│   │       ├── Framing/         # Frame, FrameReader, FrameWriter (9-byte header)
│   │       ├── rpc.bb.swift     # Generated from bebop/rpc.bop
│   │       └── *.swift          # Deadline, StatusCode, AsyncStream extensions
│   ├── SwiftBebopFoundation/    # Foundation bridges
│   ├── BebopPlugin/             # Plugin descriptor types
│   │   ├── DescriptorCompat.swift
│   │   └── Generated/           # descriptor.bb.swift, plugin.bb.swift
│   ├── bebopc-gen-swift/        # Code generator
│   │   ├── main.swift           # Entry point: stdin → codegen → stdout
│   │   ├── SwiftGenerator.swift # Top-level generation orchestrator
│   │   ├── TypeMapper.swift     # Type mapping + read/write/size expressions
│   │   ├── NamingPolicy.swift   # FQN → Swift name, camelCase, keyword escaping
│   │   ├── Generate*.swift      # Per-kind generators (6 files)
│   │   └── ...
│   ├── BebopBuildToolPlugin/    # SPM build tool plugin
│   └── BebopCommandPlugin/      # SPM command plugin (format-bop)
├── Tests/
│   ├── SwiftBebopTests/         # Runtime + RPC tests (~30 files)
│   ├── BebopPluginTests/        # Descriptor round-trip tests
│   └── CodegenTests/            # Generator output tests
├── CMakeLists.txt               # CMake integration for top-level build
├── generate.sh                  # Bootstrap: regenerates Generated/ files
└── README.md
```

---

## Code Generator (`bebopc-gen-swift`)

### Entry Point (`main.swift`)

1. **Read stdin** into a `[UInt8]` buffer (64KB chunks via `FileHandle`).
2. **Decode** a `PluginRequest` (alias for `CodeGeneratorRequest`) from wire format.
3. **Parse options** from `request.hostOptions`:
   - `Visibility`: `public` (default), `package`, or `internal`
   - `Services`: `none`, `client`, `server`, or `both` (default)
4. **Register schemas** with `NamingPolicy` (builds FQN → Swift type name map).
5. **Build definition map** for service parameter deconstruction.
6. **Iterate schemas**, filtering to `filesToGenerate`. For each, call `generator.generate(schema:)`.
7. **Encode response** via `ResponseBuilder` and write to stdout.
8. On error, encode an error response rather than crashing.

### SwiftGenerator

Orchestrates generation per schema. For each schema:

- Emits a `generatedNotice` header (source path, bebopc version, edition, license).
- Appends `import SwiftBebop` + insertion point for imports.
- Iterates `definitions`, dispatching to per-kind generators.
- If any definition uses `fixedArray`, inserts `InlineArray` equality/hashing helpers.
- Appends `@@bebop_insertion_point(eof)` at end.

Well-known FQNs (`bebop.Any`, `bebop.Empty`, `bebop.TYPE_URL_PREFIX`) are skipped since they're built into the runtime.

### TypeMapper

The core type-mapping engine. Provides four families of functions:

| Function                | Purpose                                 |
| ----------------------- | --------------------------------------- |
| `swiftType(for:)`       | Maps `TypeDescriptor` → Swift type name |
| `readExpression(for:)`  | Generates reader call expression        |
| `writeExpression(for:)` | Generates writer call expression        |
| `sizeExpression(for:)`  | Generates encoded-size expression       |

**Type mappings:**

| Bebop Type           | Swift Type                                | Wire Size         |
| -------------------- | ----------------------------------------- | ----------------- |
| `bool`               | `Bool`                                    | 1                 |
| `byte`               | `UInt8`                                   | 1                 |
| `int8`               | `Int8`                                    | 1                 |
| `int16` / `uint16`   | `Int16` / `UInt16`                        | 2                 |
| `int32` / `uint32`   | `Int32` / `UInt32`                        | 4                 |
| `int64` / `uint64`   | `Int64` / `UInt64`                        | 8                 |
| `int128` / `uint128` | `Int128` / `UInt128`                      | 16                |
| `float16`            | `Float16`                                 | 2                 |
| `float32`            | `Float`                                   | 4                 |
| `float64`            | `Double`                                  | 8                 |
| `bfloat16`           | `BFloat16`                                | 2                 |
| `string`             | `String`                                  | 4 + len + 1 (NUL) |
| `uuid`               | `BebopUUID`                               | 16                |
| `timestamp`          | `BebopTimestamp`                          | 12                |
| `duration`           | `Duration`                                | 12                |
| `array<T>`           | `[T]`                                     | 4 + elements      |
| `fixed_array<T, N>`  | `InlineArray<N, T>`                       | N \* sizeof(T)    |
| `map<K, V>`          | `[K: V]`                                  | 4 + entries       |
| `defined`            | Resolved via `NamingPolicy.fqnToTypeName` | variable          |

**Bulk scalar optimization:** For arrays/fixed arrays of scalars whose in-memory layout matches the wire format, the generator emits `readLengthPrefixedArray(of:)` / `writeLengthPrefixedArray(_:)` which use memcpy instead of per-element loops.

### NamingPolicy

- **`typeName`**: Escapes Swift hard keywords with backticks.
- **`fieldName`**: Converts to camelCase, then escapes keywords.
- **`enumCaseName`** / **`unionCaseName`**: Same as `fieldName`.
- **`registerSchemas`**: Builds an FQN → Swift name mapping. Strips the package prefix from FQNs. Special entries: `bebop.Any` → `BebopAny`, `bebop.Empty` → `BebopEmpty`.
- **`fqnToTypeName`**: Looks up cached mapping, or falls back to dropping the first (package) segment and joining with `.` for nested types.
- **camelCase conversion**: Handles `snake_case` (split on `_`) and `PascalCase` (lowercases leading uppercase run).
- **Hard keywords**: Comprehensive list of 50 Swift reserved words.

---

## Code Generation Per Definition Kind

### Enum (`GenerateEnum`)

Generates a `struct` conforming to `RawRepresentable, Sendable, Hashable, BebopRecord, BebopReflectable`.

**Regular enums** use `RawRepresentable` with `static let` members (not Swift `enum` cases), which allows unknown values to round-trip without data loss.

**Flags enums** use `OptionSet` instead, with Codable support encoding/decoding as raw integer values. Zero-value members are excluded from the `static let` declarations.

Both emit:

- `rawValue` stored property + `init(rawValue:)`
- `static let` per member with computed literal
- `decode(from:)` / `encode(to:)` using the enum's base type
- `encodedSize` (constant based on base type)
- `bebopReflection` with `EnumReflection`
- `@@bebop_insertion_point(enum_scope:Name)`

### Struct (`GenerateStruct`)

Generates a `struct` conforming to `BebopRecord, BebopReflectable`.

- Fields as `let` (or `var` if `isMutable` is true in the schema).
- `CodingKeys` enum for JSON serialization.
- Memberwise `init`.
- `decode(from:)`: Sequential reads of all fields, constructs instance.
- `encode(to:)`: Sequential writes of all fields.
- `encodedSize`: If all fields are fixed-size scalars, emits a constant. Otherwise, computes dynamically.
- `bebopReflection` with `StructReflection`.
- Nested definitions are placed inside the struct body.

**FixedArray handling:** When any field uses `InlineArray`, generates custom `==`, `hash(into:)`, `encode(to encoder:)`, and `init(from decoder:)` since `InlineArray` doesn't natively conform to `Equatable`/`Hashable`/`Codable`.

### Message (`GenerateMessage`)

Generates a `final class` conforming to `BebopRecord, BebopReflectable, @unchecked Sendable`.

Key differences from struct:

- All fields are `Optional` (`var fieldName: Type?`).
- `init` parameters have `= nil` defaults.
- Custom `==` and `hash(into:)` (since class doesn't auto-synthesize).
- **Tagged wire format:** `decode` reads a length prefix, then loops reading 1-byte tags until end-of-message marker (tag 0). Unknown tags skip remaining bytes.
- `encode` writes a length prefix placeholder, then for each non-nil field writes a tag byte + value, then an end marker, then backfills the length.
- `encodedSize` starts at 5 (4 bytes length + 1 byte end marker), adds `1 + fieldSize` for each non-nil field.

The class semantics are chosen because messages have optional fields and reference identity. `@unchecked Sendable` is used since all fields are mutable but the generated code doesn't expose thread-unsafe mutation.

### Union (`GenerateUnion`)

Generates a Swift `enum` conforming to `BebopRecord, BebopReflectable`.

- One `case` per branch with an associated value of the branch's type.
- An `unknown(discriminator: UInt8, data: [UInt8])` case for forward compatibility.
- `decode`: Reads length prefix + discriminator byte, switches to decode the appropriate branch.
- `encode`: Writes length prefix placeholder + discriminator byte + branch value, then backfills length.
- `encodedSize`: Switch on cases, each is `4 + 1 + branchValue.encodedSize`.
- Full `Codable` support using `discriminator` + `value` coding keys.
- `bebopReflection` with `UnionReflection`.

### Const (`GenerateConst`)

Generates a top-level `let` binding with the appropriate Swift literal:

- `bool` → `true` / `false`
- `int` → integer literal
- `float` → double literal (handles NaN and infinity)
- `string` → escaped string literal
- `uuid` → `BebopUUID(uuidString: "...")!`

### Service (`GenerateService`)

The most complex generator. Produces up to 5 code artifacts per service:

1. **Service enum** (always): Conforms to `BebopServiceDefinition`. Contains a `Method` enum with MurmurHash3 method IDs as raw values. Each method exposes `name`, `methodType`, `requestTypeUrl`, `responseTypeUrl`. Also has `serviceName`, `serviceInfo`, and `method(for:)`.

2. **Handler protocol** (server mode): `protocol {ServiceName}Handler: BebopHandler` with one function per method. Method signatures vary by type:
   - Unary: `func name(_ request: Req, context: some CallContext) async throws -> Res`
   - Server stream: Returns `AsyncThrowingStream<Res, Error>`
   - Client stream: Takes `AsyncThrowingStream<Req, Error>`
   - Duplex: Takes stream, returns stream

3. **Handler registration** (server mode): `extension BebopRouterBuilder` that registers all methods as closures dispatching to the handler protocol. Handles deserialization/serialization, stream bridging, deadline checking, and cancellation.

4. **Client stub** (client mode): `struct {ServiceName}Client<C: BebopChannel>` with typed methods. Handles serialization/deserialization, passes through `CallOptions`.

5. **Batch accessor** (client mode): `struct {ServiceName}_Batch<C>` + `extension Batch` providing typed batch methods for unary and server-stream methods. Supports `forwarding` from other call results.

**Deconstructed parameters:** When a request type is a struct or message with 1-4 fields, the generator emits convenience overloads that accept the fields directly rather than requiring manual construction of the request type. E.g., `client.getWidget(value: "hi")` instead of `client.getWidget(EchoRequest(value: "hi"))`.

---

## Runtime Library (`SwiftBebop`)

### BebopReader

A value-type reader over an `UnsafeRawBufferPointer`. Tracks a current offset and advances it after each read. All reads are bounds-checked via `ensureBytes(_:)`, throwing `BebopDecodingError.unexpectedEndOfData` on overflow.

Key performance characteristics:

- All primitive reads are `@inlinable @inline(__always)` for aggressive inlining.
- Multi-byte integers use `loadUnaligned(fromByteOffset:as:)` + `littleEndian` conversion.
- UInt128/Int128 are read as two 64-bit halves (low first).
- Strings are length-prefixed (UInt32) + NUL-terminated, decoded via `String(decoding:as: UTF8.self)`.
- Overflow checks use `_fastPath` / `_slowPath` for branch prediction hints.
- `&+` (wrapping add) used for offset arithmetic.

Bulk operations:

- `readArray<T: BebopScalar>(_:of:)`: memcpy for scalar arrays.
- `readInlineArray<N, T: BebopScalar>(of:)`: `loadUnaligned(as: InlineArray<N, T>.self)`.
- `readFixedInlineArray`: Per-element closure for non-trivial types.
- `readDynamicArray` / `readDynamicMap`: Length-prefixed, closure-based.

### BebopWriter

A non-copyable (`~Copyable`) value type with a manually managed `malloc`/`realloc` buffer. Freed on `deinit`.

Growth strategy: Doubles capacity when needed (`capacity &*= 2`).

Key characteristics:

- All primitive writes are `@inlinable @inline(__always)`.
- Uses `storeBytes(of:toByteOffset:as:)` for aligned/unaligned writes.
- Strings: writes UInt32 length prefix + UTF8 bytes + NUL terminator.
- `toBytes()`: Copies to a new `[UInt8]` via `unsafeUninitializedCapacity`.
- `withUnsafeBytes`: Zero-copy access for immediate reading.
- Message length backfill: `reserveMessageLength()` returns offset, `fillMessageLength(at:)` writes `UInt32(count - position - 4)`.

### BebopRecord Protocol

```swift
public protocol BebopRecord: Sendable, Hashable, Equatable, Codable {
  static func decode(from reader: inout BebopReader) throws -> Self
  func encode(to writer: inout BebopWriter)
  var encodedSize: Int { get }
}
```

Extension methods:

- `decode(from bytes: [UInt8])`: Wraps bytes in reader.
- `serializedData() -> [UInt8]`: Creates writer, encodes, returns bytes.

### BebopScalar Protocol

Marker protocol for types safe for bulk memcpy: `BitwiseCopyable` with `stride == size`. Conformers: all fixed-width integers, Float16, Float, Double, BFloat16, BebopUUID.

### BebopReflectable Protocol

Provides compile-time metadata via `static var bebopReflection: BebopTypeReflection`. Used by `BebopAny` for pack/unpack type checking.

`BebopTypeReflection` contains: `name`, `fqn` (fully-qualified name), `kind` (struct/message/enum/union), and `detail` (struct-specific, message-specific, etc. reflection data).

### BebopTimestamp

12 bytes: `Int64 seconds` + `Int32 nanoseconds` since Unix epoch. `BitwiseCopyable`. Foundation `Date` conversion available in `SwiftBebopFoundation`.

### BebopUUID

Foundation-free UUID as a 16-byte tuple. Layout-compatible with `Foundation.UUID` for zero-cost bridging. Custom `Equatable` (XOR-based), `Hashable`, `CustomStringConvertible` (formatted with dashes), `LosslessStringConvertible`, and `Codable` (as string).

### BebopAny

Type-erased container: `typeURL: String` + `value: [UInt8]`. Provides:

- `pack(_:)`: Serializes a `BebopRecord & BebopReflectable` with `"type.bebop.sh/<fqn>"` URL.
- `unpack(as:)`: Checks type URL match, then deserializes.
- `is(_:)`: Checks type URL without deserializing.

### BebopEmpty

Zero-field placeholder. `encodedSize` is 0. Decode always succeeds.

---

## Wire Format

### Scalars

All scalars are little-endian fixed-size. Strings are `uint32_length + utf8_bytes + NUL`.

### Messages (tagged)

```
[uint32 body_length]
  [uint8 field_tag] [field_value]
  [uint8 field_tag] [field_value]
  ...
  [uint8 0x00]  // end marker
```

Unknown tags cause skip to end of message body. This enables forward compatibility.

### Unions (discriminated)

```
[uint32 body_length]
[uint8 discriminator]
[branch_value]
```

Unknown discriminators are preserved as raw bytes in the `unknown` case.

### Structs (untagged)

Fields are written sequentially with no tags or length prefix. The schema defines the order.

### Arrays

```
[uint32 count]
[element_0]
[element_1]
...
```

For scalar arrays, elements are packed contiguously (memcpy-able).

### Maps

```
[uint32 count]
[key_0][value_0]
[key_1][value_1]
...
```

### Fixed Arrays (InlineArray)

No length prefix; the count is known at compile time. Scalar elements are packed contiguously.

---

## RPC Framework

The RPC layer is a complete client/server framework built on top of the wire format.

### Framing

Each RPC payload is wrapped in a 9-byte **Frame** header:

| Field      | Type         | Size | Description                                                      |
| ---------- | ------------ | ---- | ---------------------------------------------------------------- |
| `length`   | `UInt32`     | 4    | Payload byte count                                               |
| `flags`    | `FrameFlags` | 1    | Bitfield: END_STREAM (1), ERROR (2), COMPRESSED (4), TRAILER (8) |
| `streamId` | `UInt32`     | 4    | 0 when transport isolates calls                                  |

`FrameReader`: Async, takes a `ReadBytes` closure. Returns `Frame?` (nil on clean EOF). Validates payload size against `maxPayloadSize`.

`FrameWriter`: Static factory methods for common frame types: `data`, `endStream`, `error`, `trailer`.

### RPC Types (from `rpc.bop`)

- `StatusCode`: gRPC-aligned codes 0-16, 17-255 for application use.
- `FrameFlags`: OptionSet for frame metadata.
- `MethodType`: unary (0), serverStream (1), clientStream (2), duplexStream (3).
- `FrameHeader`: The 9-byte frame header struct.
- `CallHeader`: Message with `methodId`, `deadline`, `metadata`.
- `RpcError`: Message with `code`, `detail`, `metadata`.
- `TrailingMetadata`: Message with `metadata` map.
- `MethodInfo` / `ServiceInfo` / `DiscoveryResponse`: Service reflection types.
- `BatchCall` / `BatchRequest` / `BatchSuccess` / `BatchOutcome` / `BatchResult` / `BatchResponse`: Batch RPC types.

### Server Side

**`CallContext`** (protocol): Provides `methodId`, `requestMetadata`, `deadline`, `isCancelled`, `setResponseMetadata`.

**`BebopHandler`** (protocol): Empty base for generated handler protocols.

**`BebopInterceptor`** (protocol): Middleware with `intercept(methodId:ctx:proceed:)`. Called in onion order (first-added is outermost).

**`BebopServiceDefinition`** (protocol): Generated service enum with `Method` associated type, `serviceName`, `serviceInfo`, `method(for:)`.

**`BebopServiceMethod`** (protocol): Generated method enum with `rawValue` (MurmurHash3 ID), `name`, `methodType`, request/response type URLs.

**`BebopRouterBuilder`**: Mutable builder. Registers services via generated `register(serviceName:)` extension. Configures interceptors, discovery, batch limits. Produces immutable `BebopRouter`.

**`BebopRouter`**: Immutable, `Sendable` dispatcher. Methods:

- `unary(methodId:payload:ctx:)`: Handles reserved method 0 (discovery) and 1 (batch), then dispatches to registered handler.
- `serverStream(methodId:payload:ctx:)`: Dispatches to handler, returns `AsyncThrowingStream<[UInt8], Error>`.
- `clientStream(methodId:ctx:)`: Returns `(send, finish)` tuple.
- `duplexStream(methodId:ctx:)`: Returns `(send, finish, responses)` tuple.
- `methodType(for:)`: Lookup.

**Interceptor chain** (`BebopRouter+Interceptors`): Builds a nested closure chain. Before dispatching, checks `Task.isCancelled`, `ctx.isCancelled`, and `deadline.isPast`.

**Batch handling** (`BebopRouter+Batch`):

1. Validates call IDs (unique, non-negative), dependency references.
2. Builds execution layers via dependency depth analysis.
3. Executes each layer with `withTaskGroup` for parallelism.
4. Supports `inputFrom` forwarding: pipes first payload of a dependency's result as input.
5. Only unary and server-stream methods are supported in batches.
6. Respects `maxBatchSize` and `maxBatchStreamElements` limits.

### Client Side

**`BebopChannel`** (protocol): Transport abstraction with four methods (`unary`, `serverStream`, `clientStream`, `duplexStream`). All async throws.

**`CallOptions`**: Carries `metadata: [String: String]` and optional `deadline: BebopTimestamp`. Convenience init with `Duration` timeout.

**Generated client stubs**: `{Service}Client<C: BebopChannel>`. Each method:

- Serializes request via `serializedData()`.
- Calls appropriate channel method.
- Deserializes response via `T.decode(from:)`.
- For streams, uses `AsyncThrowingStream.decode(_:)` extension.

**`Batch<Channel>`**: Accumulates `BatchCall` entries. `addUnary` / `addServerStream` return `CallRef<R>` / `StreamRef<R>` handles. `addUnary(forwardingFrom:)` creates dependency chains. `execute()` sends a single `BatchRequest` via reserved method ID 1, returns `BatchResults`.

**`BatchResults`**: Subscript access via `CallRef<R>` or `StreamRef<R>`. Decodes payloads or throws `BebopRpcError`.

**`DiscoveryClient`**: Queries reserved method ID 0. Returns `DiscoveryResponse` with `[ServiceInfo]`.

### Deadlines

`BebopTimestamp+Deadline`: Uses `clock_gettime(CLOCK_REALTIME)` for `now`. Provides `isPast`, `timeRemaining` (returns `Duration?`), and `init(fromNow:)`.

`withDeadline(_:operation:)`: Races the operation against a sleep. On timeout, cancels the operation and throws `deadlineExceeded`.

### Error Handling

`BebopRpcError`: Bridges to/from wire-format `RpcError` message. Contains `code: StatusCode`, optional `detail` and `metadata`. `StatusCode` has human-readable `.name` (`OK`, `CANCELLED`, `UNKNOWN`, etc.).

---

## BFloat16 Implementation

A complete `BinaryFloatingPoint` implementation of the Brain Float16 format (8-bit exponent, 7-bit significand).

### Storage

`UInt16` backing storage (`_value`). `BitwiseCopyable`, `Sendable`.

### C Shim (`CBFloat16`)

`cbfloat16.h` provides `cbfloat16_from_float` and `cbfloat16_from_double`:

- **Native path**: If `__bf16` type is available (ARM, RISC-V, x86 with SSE2), uses hardware cast.
- **Fallback path**: Manual round-to-nearest-even via bit manipulation. NaN payloads are quieted.

### Protocol Conformances

| Protocol                               | Implementation                                                                         |
| -------------------------------------- | -------------------------------------------------------------------------------------- |
| `FloatingPoint`                        | Full: sign, exponent, significand, classification, arithmetic ops delegated to `Float` |
| `BinaryFloatingPoint`                  | `exponentBitCount = 8`, `significandBitCount = 7`, `init(Float)`, `init(Double)`       |
| `Numeric` / `SignedNumeric`            | Via `Float` promotion                                                                  |
| `Equatable`                            | Default (bitwise)                                                                      |
| `Hashable`                             | Normalizes +-0 to same hash                                                            |
| `Codable`                              | Encodes/decodes as `Float`                                                             |
| `LosslessStringConvertible`            | Via `Float`                                                                            |
| `Strideable`                           | `Stride = BFloat16`                                                                    |
| `ExpressibleByFloatLiteral`            | Via `Float` literal                                                                    |
| `ExpressibleByIntegerLiteral`          | Via `Float` literal                                                                    |
| `SIMDScalar`                           | Full SIMD2/4/8/16/32/64 storage types backed by `UInt16.SIMDNStorage`                  |
| `AtomicRepresentable`                  | Via `UInt16.AtomicRepresentation` (macOS 15+)                                          |
| `BNNSScalar`                           | Accelerate framework integration (macOS 12+)                                           |
| `BNNSGraph.Builder.OperationParameter` | BNNS graph builder (macOS 26+)                                                         |
| `BebopScalar`                          | Enables bulk memcpy in BebopReader/Writer                                              |

### Notable Constants

- `zero` = 0x0000, `negativeZero` = 0x8000
- `one` = 0x3F80, `negativeOne` = 0xBF80
- `infinity` = 0x7F80, `nan` = 0x7FC0, `signalingNaN` = 0xFF81
- `greatestFiniteMagnitude` = 0x7F7F
- `pi` = 0x4049
- `ulpOfOne` = 0x1.0p-7
- `leastNormalMagnitude` = 0x1.0p-126

---

## Foundation Interoperability (`SwiftBebopFoundation`)

Four small extension files:

1. **`BebopUUID+Foundation`**: Zero-cost bridging `BebopUUID ↔ Foundation.UUID` (same 16-byte tuple layout).
2. **`BebopTimestamp+Foundation`**: `BebopTimestamp(date:)` and `.date` property via `timeIntervalSince1970`.
3. **`BebopReaderWriter+Foundation`**: `readFoundationUUID()`, `readData(_:)`, `writeUUID(_ value: UUID)`, `writeData(_ data: Data)`.
4. **`BFloat16+CGFloat`**: `CGFloat(BFloat16)` and `BFloat16(exactly: CGFloat)`.

---

## Plugin Protocol (`BebopPlugin`)

### Generated Types

Two generated files from `bebop/descriptor.bop` and `bebop/plugin.bop`:

- **`descriptor.bb.swift`**: All schema descriptor types (`TypeKind`, `DefinitionKind`, `TypeDescriptor`, `FieldDescriptor`, `EnumMemberDescriptor`, `StructDef`, `MessageDef`, `UnionDef`, `ServiceDef`, `MethodDescriptor`, `SchemaDescriptor`, `LiteralValue`, `DecoratorUsage`, etc.).
- **`plugin.bb.swift`**: `Version`, `CodeGeneratorRequest`, `CodeGeneratorResponse`, `GeneratedFile`, `Diagnostic`, etc.

### DescriptorCompat

Provides convenience aliases and helpers:

- `PluginRequest` = `CodeGeneratorRequest`
- `CompilerVersion` = `Version`
- `ResponseBuilder`: Accumulates generated files, encodes response.
- `FieldDescriptor.isDeprecated`: Checks for `@deprecated` decorator.
- `TypeKind.isScalar`: rawValue in 1...19.

---

## SPM Build & Command Plugins

### BebopBuildToolPlugin

Implements both `BuildToolPlugin` (SPM) and `XcodeBuildToolPlugin` (Xcode):

- Finds `.bop` files in the target's source files.
- Locates `bebopc` in PATH or fallback locations (`~/.local/bin`, `/opt/homebrew/bin`, etc.).
- Emits a build command: `bebopc build <inputs> --swift_out=<outputDir>`.
- Tracks `bebop.yml` config file as input if present.
- Output files are `{name}.bb.swift`.

### BebopCommandPlugin

Implements `CommandPlugin` (SPM) and `XcodeCommandPlugin` (Xcode):

- Verb: `format-bop`.
- Finds `.bop` files, runs `bebopc fmt` on them.
- Supports `--target` filtering.

---

## Build System Integration

### CMake (`CMakeLists.txt`)

- Detects Swift 6.2+ availability.
- Runs `swift build --product bebopc-gen-swift`.
- Copies binary to `CMAKE_RUNTIME_OUTPUT_DIRECTORY`.
- Installs to `CMAKE_INSTALL_BINDIR`.

### `generate.sh`

Bootstrap script for regenerating the Generated/ files:

1. Builds `bebopc-gen-swift`.
2. Runs `bebopc build` on `descriptor.bop` + `plugin.bop` → `descriptor.bb.swift` + `plugin.bb.swift`.
3. Runs `bebopc build` on `rpc.bop` → `rpc.bb.swift` (strips `import SwiftBebop` since it's compiled into the same module).
4. Runs `bebopc build` on `test_service.bop` → `test_service.bb.swift`.

---

## Testing Strategy

### Test Suites

| Suite                      | Count                                                         | Focus                                                                                                            |
| -------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `BebopRuntimeTests`        | ~45 tests                                                     | Reader/Writer round-trips for all types, wire format verification, bounds checking, writer growth, BFloat16 math |
| `BebopPluginTests`         | ~35 tests                                                     | Descriptor type round-trips, ResponseBuilder, BebopAny pack/unpack, helper utilities                             |
| `SwiftGeneratorTests`      | ~60 tests                                                     | Generated code snapshot tests for all definition kinds, option variations                                        |
| `RouterUnaryTests`         | Router unary dispatch, error propagation, unknown methods     |
| `RouterServerStreamTests`  | Server stream dispatch, multiple elements, empty streams      |
| `RouterClientStreamTests`  | Client stream send/finish flow                                |
| `RouterDuplexStreamTests`  | Bidirectional stream flow                                     |
| `RouterBuilderTests`       | Builder configuration, duplicate method detection             |
| `RouterBatchTests`         | Batch execution, dependency chains, error cases               |
| `RouterDiscoveryTests`     | Discovery enable/disable, service listing                     |
| `InterceptorTests`         | Interceptor ordering, rejection, deadline/cancellation checks |
| `WidgetServiceClientTests` | Generated client stub for all method types                    |
| `WidgetServiceBatchTests`  | Generated batch accessor methods                              |
| `BatchClientTests`         | Client-side batch building and execution                      |
| `BatchEncodingTests`       | BatchCall/BatchRequest wire format                            |
| `BatchResultsTests`        | Result decoding from batch responses                          |
| `EndToEndTests`            | Full client→router→handler→client flows                       |
| `FrameTests`               | Frame encode/decode, flags                                    |
| `FrameReaderTests`         | Async frame reading, EOF, truncation                          |
| `FrameWriterTests`         | Frame factory methods                                         |
| `StatusCodeTests`          | Status code names                                             |
| `BebopEmptyTests`          | Empty type round-trip                                         |
| `BebopRpcErrorTests`       | Error construction and wire bridging                          |
| `CallOptionsTests`         | Options construction with timeout                             |
| `DiscoveryClientTests`     | Discovery via loopback                                        |
| `ServiceDefinitionTests`   | Generated service enum properties                             |
| `StreamDecodeTests`        | AsyncThrowingStream.decode extension                          |
| `TimestampDeadlineTests`   | Deadline arithmetic, isPast                                   |
| `WireTypeTests`            | TypeMapper scalar sizes                                       |
| `AsyncIteratorStreamTests` | Stream utilities                                              |

### Test Infrastructure

`RpcTestInfrastructure.swift` provides:

- `TestCallContext`: Minimal `CallContext` implementation.
- `WidgetHandler`: Implements `WidgetServiceHandler` with echo/count/collect/mirror behavior.
- `LoopbackChannel`: In-process `BebopChannel` that dispatches directly to a `BebopRouter`.
- `buildRouter()` / `buildChannel()`: Factory functions for test setup.

### Test Schema

`test_service.bop` defines a `WidgetService` with four methods covering all RPC patterns:

- `GetWidget`: Unary (echo)
- `ListWidgets`: Server stream (count)
- `UploadWidgets`: Client stream (collect) - with `@deprecated` decorator
- `SyncWidgets`: Duplex stream (bidirectional echo)

---

## Design Decisions & Notable Patterns

### 1. Enums as Structs

Bebop enums are generated as `struct` with `RawRepresentable` rather than Swift `enum` with cases. This allows unknown enum values to round-trip without data loss, which is critical for forward compatibility when new members are added to a schema.

### 2. Messages as Classes

Messages are `final class` rather than `struct` because:

- All fields are optional (mutable state).
- Reference semantics match the "partial update" nature of messages.
- `@unchecked Sendable` is acceptable since generated code controls mutation.

### 3. Unions with Unknown Case

Every union includes `case unknown(discriminator: UInt8, data: [UInt8])` for forward compatibility. Unknown branches preserve their discriminator and raw bytes.

### 4. Non-Copyable Writer

`BebopWriter` is `~Copyable` to prevent accidental copying of the malloc-backed buffer. This is a Swift 6.2 feature that eliminates a class of memory bugs.

### 5. Performance Annotations

Extensive use of `@inlinable`, `@inline(__always)`, `@usableFromInline`, `_fastPath`, `_slowPath`, and `&+` (wrapping arithmetic) throughout the reader/writer for maximum performance. Bulk scalar operations use memcpy.

### 6. Insertion Points

Generated code includes `@@bebop_insertion_point(...)` comments at strategic locations (imports, decode start/end, encode start/end, type scope, eof). These enable post-generation code injection by other tools.

### 7. Batch Dependency Resolution

The batch system builds an execution DAG using `inputFrom` references. Independent calls within the same "depth layer" execute concurrently via `TaskGroup`. Calls forward payloads from dependencies without re-serialization.

### 8. Foundation-Free Core

The core `SwiftBebop` module has zero Foundation dependency. `BebopUUID` and `BebopTimestamp` are Foundation-free value types. Foundation bridges are in a separate `SwiftBebopFoundation` module.

### 9. InlineArray (Swift 6.2)

Fixed-size arrays use Swift 6.2's `InlineArray<N, T>` with value-parameter generics (`<let N: Int>`). Since `InlineArray` doesn't conform to `Equatable`/`Hashable`/`Codable`, the generator emits fileprivate helpers and custom conformances when needed.

### 10. Cross-Platform C Shim

The BFloat16 C shim detects native `__bf16` support at compile time across ARM, RISC-V, x86+SSE2, and falls back to manual bit manipulation with round-to-nearest-even.

### 11. Visibility Control

The `Visibility` generator option (`public`/`package`/`internal`) is respected per-definition, but definitions marked `local` in the schema always use `internal` regardless of the global setting.

### 12. Service Generation Modes

The `Services` option allows generating only client stubs, only server handlers, both, or neither. This enables lean builds where only one side is needed.
