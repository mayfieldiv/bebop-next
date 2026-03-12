# FD-001: Service Generation

**Status:** Planned
**Priority:** High
**Effort:** High (> 4 hours)
**Impact:** Enables RPC service support for Rust consumers

## Problem

Services are completely unimplemented. `gen_service.rs` only emits TODO comments and logs method info to stderr. Rust users cannot use Bebop service definitions.

## Solution

Generate full service support:

- Service definition enum with Method sub-enum
- Method enum: one variant per RPC method, with MurmurHash3 id
- Handler trait: async fn per method
  - Unary: `fn(Request, Context) -> Response`
  - ServerStream: `fn(Request, Context) -> Stream<Response>`
  - ClientStream: `fn(Stream<Request>, Context) -> Response`
  - DuplexStream: `fn(Stream<Request>, Context) -> Stream<Response>`
- Client struct: typed RPC call methods
- Batch accessor for batched unary/server-stream calls
- Router registration extension

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src/generator/gen_service.rs` | MODIFY | Implement service code generation |
| `runtime/src/lib.rs` | MODIFY | Add service runtime traits/types |
| `integration-tests/` | MODIFY | Add service round-trip tests |

## Verification

- Generate code from a `.bop` schema with service definitions
- Verify generated service traits, client stubs, and router compile
- Integration tests for service method dispatch

## RPC Spec Summary

Key elements from `docs/RPC.md` the generated code must support:

**Wire types** (defined in `bebop/rpc.bop`): StatusCode, FrameFlags, MethodType, FrameHeader (9 bytes fixed), CallHeader, RpcError, TrailingMetadata, batch types, discovery types.

**Call lifecycle:**
- Unary: CallHeader -> request[END_STREAM] -> response -> trailer[END_STREAM]
- Server stream: request[END_STREAM] -> N responses -> END_STREAM
- Client stream: N requests -> END_STREAM -> response + trailer
- Duplex: both sides independent

**Batching (method ID 1):** Combines unary + server-stream calls in one round trip with `input_from` pipelining.

**Transport bindings:** HTTP (`POST /{Service}/{Method}`), binary TCP/WebSocket/IPC, WebSocket (one element per binary message).

## Design Considerations

- Runtime needs new traits: `BebopHandler`, `BebopChannel`, `BebopRouter`/`BebopRouterBuilder`
- Streaming types: wrapping `futures::Stream` or `tokio::sync::mpsc`
- Consider runtime-agnostic vs requiring `tokio`
- `no_std` compatibility: services likely require `std` or `alloc` — feature-gate
- May want a `bebop-runtime/rpc` feature flag

## Related

- `src/generator/gen_service.rs:8-20` — current TODO stub
- `research.md` §4.6.6, §12.1
- Swift implementation: `plugins/swift/Sources/bebopc-gen-swift/GenerateService.swift`
- C (reflection only): `plugins/c/src/generator.c`

## Source

Migrated from `../../issues/services-rpc.md`
