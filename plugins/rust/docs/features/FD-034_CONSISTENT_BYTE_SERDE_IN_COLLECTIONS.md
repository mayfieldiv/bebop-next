# FD-034: Consistent Byte Serialization in Collections

**Status:** Pending Verification
**Priority:** Medium
**Effort:** Medium (1-4 hours)
**Impact:** Byte arrays work correctly in all contexts (Option, Vec, HashMap); fixes compile error for byte[] in messages

## Problem

The generator emits `#[serde(with = "bebop_runtime::serde_cow_bytes")]` on direct `byte[]` fields (`Cow<'buf, [u8]>`), which uses `serde_bytes` for compact serialization (base64 in JSON, raw bytes in binary formats).

Three problems exist:

### 1. byte[] in messages doesn't compile with serde

Message fields are `Option<T>`. A `byte[]` message field becomes `Option<Cow<'buf, [u8]>>`, but `#[serde(with = "bebop_runtime::serde_cow_bytes")]` expects a bare `Cow<[u8]>`, not `Option<Cow<[u8]>>`. This causes a compile error:

```
expected `Option<Cow<'_, [u8]>>`, found `Cow<'_, [u8]>`
```

No existing test schema has a `byte[]` field in a message, so this was undetected.

### 2. Nested byte arrays serialize verbosely

When `Cow<[u8]>` appears inside a collection — `byte[][]` → `Vec<Cow<[u8]>>`, `map[K, byte[]]` → `HashMap<K, Cow<[u8]>>` — no `serde_bytes` attribute is emitted. The inner byte arrays fall back to default serde, which serializes each as a JSON array of numbers: `[72, 101, 108, 108, 111]`.

### 3. Inconsistency

The same `byte[]` type serializes differently depending on whether it's a direct struct field or nested inside a collection/message.

### Affected patterns

| Bebop type | Rust type | Current serde behavior |
|-----------|-----------|----------------------|
| `byte[]` (struct field) | `Cow<'buf, [u8]>` | ✅ Compact via `serde_bytes` |
| `byte[]` (message field) | `Option<Cow<'buf, [u8]>>` | ❌ Compile error |
| `byte[][]` | `Vec<Cow<'buf, [u8]>>` | ❌ Array of number arrays (verbose) |
| `map[K, byte[]]` | `HashMap<K, Cow<'buf, [u8]>>` | ❌ Map values as number arrays (verbose) |

## Solution

Introduce a `BebopBytes<'buf>` newtype in the runtime crate that wraps `Cow<'buf, [u8]>` and implements `Serialize`/`Deserialize` via `serde_bytes`. The generator emits `BebopBytes<'buf>` instead of `Cow<'buf, [u8]>` for all byte array fields. All collection/Option nesting works automatically because serde impls live on the type itself.

This replaces the `serde_cow_bytes` adapter module entirely.

### Design decisions

| Decision | Resolution | Rationale |
|----------|-----------|-----------|
| Newtype vs adapter modules | Newtype | Adapter approach has combinatorial explosion (Option, Vec, HashMap variants) |
| Runtime vs generated code | Runtime | Analogous to `BebopTimestamp`/`BebopDuration`; one definition, all schemas use it |
| `into_owned()` | Method on newtype | Generator calls `v.into_owned()` uniformly |
| `new()` constructor params | `impl Into<BebopBytes<'static>>` | Callers can pass `Vec<u8>` directly via `From` impl |
| Serde deserialize borrowing | Always owned | Zero-copy path is `BebopDecode`, not serde; keeps impl simple |
| `serde_cow_bytes.rs` | Delete | Plugin is new in this PR, no backward compat needed |
| `#[serde(borrow)]` emission | Remove | Not needed when type owns its serde impl; current deserialize is always-owned |

### Runtime: `BebopBytes<'buf>`

New file `runtime/src/bytes.rs`:

```rust
pub struct BebopBytes<'buf>(pub Cow<'buf, [u8]>);
```

**Unconditional traits:**
- `Deref<Target=[u8]>`, `AsRef<[u8]>` — so `writer.write_byte_array(&field)` works unchanged
- `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Default`
- `From<Vec<u8>>`, `From<&'buf [u8]>`, `From<Cow<'buf, [u8]>>`
- `fn into_owned(self) -> BebopBytes<'static>`
- `fn borrowed(s: &'buf [u8]) -> Self` — convenience for decode path

**Behind `cfg(feature = "serde")`:**
- `Serialize` — delegates to `serde_bytes::serialize`
- `Deserialize` — uses `serde_bytes::ByteBuf`, returns owned

### Generator changes

~9 branches in `type_mapper.rs` that produce `Cow<'buf, [u8]>` switch to `::bebop_runtime::BebopBytes<'buf>`:

| Generated pattern | Before | After |
|-------------------|--------|-------|
| Cow type | `alloc::borrow::Cow<'buf, [u8]>` | `::bebop_runtime::BebopBytes<'buf>` |
| Owned type | `alloc::vec::Vec<u8>` | `::bebop_runtime::BebopBytes<'static>` |
| Decode expr | `Cow::Borrowed(reader.read_byte_slice()?)` | `::bebop_runtime::BebopBytes::borrowed(reader.read_byte_slice()?)` |
| Encode expr | `writer.write_byte_array(&v)` | `writer.write_byte_array(&v)` (unchanged — `Deref`) |
| `new()` params | `Vec<u8>` | `impl Into<BebopBytes<'static>>` |

Remove from generator:
- All `serde_cow_bytes` / `serde(borrow)` emission in `gen_struct.rs` and `gen_message.rs`
- `is_byte_array_cow_field()` no longer needed for serde (still used for type mapping)

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `runtime/src/bytes.rs` | CREATE | `BebopBytes<'buf>` newtype with serde impls |
| `runtime/src/lib.rs` | MODIFY | Add `mod bytes`, `pub use bytes::BebopBytes`, remove `serde_cow_bytes` |
| `runtime/src/serde_cow_bytes.rs` | DELETE | Replaced by `BebopBytes` serde impls |
| `src/generator/type_mapper.rs` | MODIFY | ~9 branches: `Cow<[u8]>` → `BebopBytes`, decode/encode/size exprs |
| `src/generator/gen_struct.rs` | MODIFY | Remove `serde_cow_bytes` / `serde(borrow)` emission |
| `src/generator/gen_message.rs` | MODIFY | Remove `serde_cow_bytes` / `serde(borrow)` emission |

## Verification

- `byte[]` struct field: serde round-trip via JSON
- `byte[]` message field: compiles and serde round-trips (currently compile error)
- `byte[][]` struct/message: inner arrays serialize compactly
- `map[K, byte[]]` struct/message: values serialize compactly
- `BebopBytes` works in `Option`, `Vec`, `HashMap` contexts without extra attributes
- Bebop wire format encode/decode unchanged for all byte array patterns
- `./test.sh` passes

## Related

- FD-032: Serde Round-Trip Test Coverage (will have tests that verify this fix)
- FD-003: SerdeMode (established the serde generation infrastructure)
