# Implement Service/RPC Code Generation

- [ ] Implement service/RPC code generation #rust-plugin 🔺 🆔 services

The largest missing feature. `gen_service.rs` currently emits only a `// TODO` comment with design notes and logs method info to stderr. Both the [[reflection|reflection metadata]] and this are needed for feature parity with Swift and C.

## Current State

`plugins/rust/src/generator/gen_service.rs` has a placeholder that outlines the needed components but generates no usable code.

## Required Components (per the TODO and Swift reference)

### Service Metadata
- Service definition type with `Method` sub-enum (variants per RPC method)
- Each method variant carries a `u32` MurmurHash3 id (already available in descriptor as `m.id`)
- Method metadata: name, method type (Unary/ServerStream/ClientStream/Duplex), request/response type URLs

### Handler Trait
Generate an async trait per service:
```rust
#[async_trait]
pub trait FooHandler: BebopHandler {
    async fn bar(&self, req: BarRequest<'_>, ctx: Context) -> Result<BarResponse<'static>>;
    // ServerStream, ClientStream, Duplex variants with Stream types
}
```

### Client Stub
```rust
pub struct FooClient<C: BebopChannel> { channel: C }
impl<C: BebopChannel> FooClient<C> {
    pub async fn bar(&self, req: BarRequest<'_>) -> Result<BarResponse<'static>> { ... }
}
```

### Router Registration
Extension method to register a handler with the router by service name + method dispatch table.

### Batch Accessor
For batched unary/server-stream calls with `CallRef` / `StreamRef` return types (see Swift's `Xxx_Batch<C>`).

## Design Considerations
- The runtime will need new traits: `BebopHandler`, `BebopChannel`, `BebopRouter`/`BebopRouterBuilder`
- Streaming types need definition (likely wrapping `futures::Stream` or `tokio::sync::mpsc`)
- Consider whether to require `tokio` or stay runtime-agnostic
- `no_std` compatibility: services likely require `std` or at minimum `alloc` — consider feature-gating
- May want a `bebop-runtime/rpc` feature flag

## RPC Spec Summary (from `docs/RPC.md`)

The RPC spec is substantial. Key elements the generated code must support:

### Wire Types (defined in `bebop/rpc.bop`)
- `StatusCode` enum (byte): OK(0)..UNAUTHENTICATED(16), 17-255 app-defined
- `FrameFlags` flags enum (byte): NONE(0), END_STREAM(1), ERROR(2), COMPRESSED(4), TRAILER(8)
- `MethodType` enum (byte): UNARY(0), SERVER_STREAM(1), CLIENT_STREAM(2), DUPLEX_STREAM(3)
- `FrameHeader` struct: length(uint32) + flags(FrameFlags) + stream_id(uint32) = 9 bytes fixed
- `CallHeader` message: method_id(1:uint32), deadline(2:timestamp), metadata(3:map[string,string])
- `RpcError` message: code(1:StatusCode), detail(2:string), metadata(3:map[string,string])
- `TrailingMetadata` message: metadata(1:map[string,string])
- Batch types: `BatchCall`, `BatchRequest`, `BatchSuccess`, `BatchOutcome` (union), `BatchResult`, `BatchResponse`
- Discovery: `MethodInfo`, `ServiceInfo`, `DiscoveryResponse`

### Call Lifecycle
- **Unary**: CallHeader → Frame[END_STREAM] request → Frame response → Frame[END_STREAM|TRAILER]
- **Server stream**: CallHeader → request[END_STREAM] → N response frames → END_STREAM
- **Client stream**: CallHeader → N requests → END_STREAM → response + trailer
- **Duplex**: both sides independent frames

### Cancellation & Deadlines
- Deadlines are absolute timestamps, not relative
- Already-passed deadline → DEADLINE_EXCEEDED without invoking handler
- Propagate to downstream: use earlier of propagated and local timeout

### Batching (Method ID 1)
- Combines unary + server-stream calls in one round trip
- `input_from` field for pipelining (forward result from call N to call M)
- Server: validate all → build dependency graph → execute layers in parallel
- Client-stream and duplex excluded

### Transport Bindings
- HTTP: `POST /{Service}/{Method}`, Content-Type `application/bebop`
- Binary (TCP/WebSocket/IPC): CallHeader first, then frames
- WebSocket: one protocol element per binary message
- TCP multiplexing: odd IDs = client, even IDs = server

### CallContext
- Request metadata (read-only), response metadata (mutable), deadline, cancellation flag
- Interceptors wrap dispatch in registration order

See [[spec-compliance]] for the full RPC checklist.

## References
- Formal RPC spec: `docs/RPC.md`
- Swift full implementation: `plugins/swift/Sources/bebopc-gen-swift/GenerateService.swift`
- C reflection-only: `plugins/c/src/generator.c` (service reflection descriptors but no stubs)
- Whitepaper §Services: method dispatch via MurmurHash3, four method types, batch pipelining, RPC frame format
