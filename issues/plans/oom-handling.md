# Plan: OOM Handling — Replace Panic with Fallible Allocation

**Status:** Draft
**Depends on:** None
**Blocked by:** None
**Blocks:** None

## Problem Statement

The Rust runtime panics on out-of-memory (OOM) because it uses standard `Vec`, `String`, and `HashMap` allocating methods that call `handle_alloc_error` on failure. This is Rust's default behavior, but it's unsuitable for:

- **Embedded systems** with limited memory where graceful degradation is required
- **Server applications** processing untrusted input where a malicious payload with `count = 0xFFFFFFFF` triggers a multi-gigabyte allocation attempt
- **Real-time systems** where panicking is unacceptable

The C plugin handles this differently: every allocation is checked and returns `BEBOP_WIRE_ERR_OOM` on failure, allowing the caller to handle the error gracefully.

### State of the Art: Existing Bebop Implementations

Research into the current production Bebop implementations and the plugins in this repo reveals that **this vulnerability is widespread**:

**Production Bebop Rust runtime** (`~/vendor/bebop/Runtime/Rust/src/serialization/mod.rs`):
- `read_array()` calls `Vec::with_capacity(len)` directly from the wire-read `u32` count with **no pre-validation** against the buffer size
- A malicious payload with `count = 0xFFFFFFFF` for a `u64` array triggers a 32 GiB allocation attempt, causing either a panic or the OS killing the process
- Same vulnerability exists for `read_map()` with `HashMap::with_capacity(len)`

**C plugin** (this repo):
- Has OOM error codes (`BEBOP_WIRE_ERR_OOM`) and checks `malloc`/`realloc` return values
- However, does **not** pre-validate array counts against the buffer size
- Has an integer overflow vulnerability: `_len * sizeof(*_d)` can overflow before being passed to `malloc`, potentially allocating a too-small buffer and causing heap corruption

**Swift plugin** (this repo):
- `readLengthPrefixedArray` for scalar arrays **does** validate via `ensureBytes()` before allocation — this is the correct pattern
- However, `readDynamicArray` and `readDynamicMap` for variable-size elements do **not** pre-validate counts, making them vulnerable to the same malicious-count attack as Rust

This means the Rust plugin has an opportunity to be the most robust implementation by adding proper pre-validation, surpassing even the production Bebop runtime.

## Current Code Analysis

### Allocation Sites That Can Panic

#### 1. `reader.rs:204` — `Vec::with_capacity(count)`

```rust
pub fn read_array<T>(...) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;
  let mut items = Vec::with_capacity(count);  // PANICS if count is huge
  // ...
}
```

A malicious payload can set `count` to `u32::MAX` (4,294,967,295), causing `Vec::with_capacity` to request up to `count * size_of::<T>()` bytes. For `T = u64`, that's 32 GiB.

#### 2. `reader.rs:217` — `HashMap::with_capacity(count)`

```rust
pub fn read_map<K: Eq + Hash, V>(...) -> Result<HashMap<K, V>> {
  let count = self.read_u32()? as usize;
  let mut map = HashMap::with_capacity(count);  // PANICS
  // ...
}
```

Same issue as arrays.

#### 3. `reader.rs:150` — `str_bytes.to_vec()`

```rust
String::from_utf8(str_bytes.to_vec()).map_err(|_| DecodeError::InvalidUtf8)
```

The `to_vec()` allocates a copy of the string bytes. The string length is bounded by the preceding `u32` length prefix (max 4 GiB), but in practice is also bounded by `self.ensure(len + 1)` (the buffer must actually contain those bytes).

#### 4. `reader.rs:242` — `read_bytes`

```rust
pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
  self.ensure(count)?;
  let bytes = self.buf[self.pos..self.pos + count].to_vec();  // PANICS
  // ...
}
```

#### 5. `writer.rs:26-27` — `BebopWriter::new()` / `with_capacity()`

```rust
pub fn new() -> Self { Self { buf: Vec::new() } }
pub fn with_capacity(cap: usize) -> Self { Self { buf: Vec::with_capacity(cap) } }
```

#### 6. `writer.rs` — All `extend_from_slice` and `push` calls

Every write operation can trigger a reallocation that may OOM.

#### 7. `traits.rs:17` — `BebopEncode::to_bytes()`

```rust
fn to_bytes(&self) -> Vec<u8> {
  let mut writer = BebopWriter::with_capacity(self.encoded_size());
  // ...
}
```

### Existing Mitigation: `ensure()` Bounds Check

```rust
fn ensure(&self, count: usize) -> Result<()> {
  if self.pos + count > self.buf.len() {
    Err(DecodeError::UnexpectedEof { needed: count, available: self.remaining() })
  } else {
    Ok(())
  }
}
```

The `ensure()` check validates that the *buffer* contains enough bytes, but it does NOT prevent the allocation from being too large. For `read_array`, the `count` is read from the wire, and `Vec::with_capacity(count)` is called *before* reading any elements. If `count = 4 billion` but only 100 bytes remain in the buffer, the `Vec` allocation still attempts 4 billion × sizeof(T) bytes before any element reads fail.

**This is the core vulnerability:** the allocation size is derived from untrusted input without validation against the buffer size.

## Design Options

### Option A: Pre-Validate Allocation Size Against Buffer

Before allocating, check that the requested element count is physically possible given the remaining buffer size:

```rust
pub fn read_array<T>(...) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;

  // Sanity check: count elements need at least count bytes (1 byte per element minimum).
  // This prevents huge allocations from malicious count values.
  if count > self.remaining() {
    return Err(DecodeError::UnexpectedEof {
      needed: count,
      available: self.remaining(),
    });
  }

  let mut items = Vec::with_capacity(count);
  for _ in 0..count {
    items.push(read_elem(self)?);
  }
  Ok(items)
}
```

For typed arrays where we know the element size, the check can be tighter:

```rust
pub fn read_scalar_array<T: FixedScalar>(&mut self) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;
  let byte_len = count.checked_mul(core::mem::size_of::<T>())
    .ok_or(DecodeError::UnexpectedEof { needed: usize::MAX, available: self.remaining() })?;

  if byte_len > self.remaining() {
    return Err(DecodeError::UnexpectedEof {
      needed: byte_len,
      available: self.remaining(),
    });
  }

  // Now safe to allocate — we know the buffer contains enough data
  let mut vec = Vec::with_capacity(count);
  // ...
}
```

### Option B: Fallible Allocation via `try_reserve` (Stable)

Use `Vec::try_reserve` instead of `Vec::with_capacity`:

```rust
pub fn read_array<T>(...) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;
  let mut items = Vec::new();
  items.try_reserve(count).map_err(|_| DecodeError::AllocationFailed)?;
  // ...
}
```

`try_reserve` is stable since Rust 1.57. For `HashMap`, `hashbrown::HashMap::try_reserve` is available.

### Option C: Configurable Allocation Limit

Add a maximum allocation size to the reader:

```rust
pub struct BebopReader<'a> {
  buf: &'a [u8],
  pos: usize,
  max_alloc: usize,  // Default: buf.len()
}
```

### Option D: Combine A + B (Recommended)

Pre-validate against buffer size AND use fallible allocation. This provides defense in depth:

1. Buffer size check catches malicious count values cheaply
2. `try_reserve` catches actual OOM on valid-but-large payloads

## Recommended Approach: Option D (Pre-Validate + Fallible Allocation)

### Changes Required

#### 1. `error.rs` — Add `AllocationFailed` Variant and `#[non_exhaustive]`

```rust
#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
  UnexpectedEof { needed: usize, available: usize },
  InvalidUtf8,
  InvalidEnum { type_name: &'static str, value: u64 },
  AllocationFailed,  // NEW
}

impl fmt::Display for DecodeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      // ...existing arms...
      Self::AllocationFailed => write!(f, "memory allocation failed"),
    }
  }
}
```

Adding `#[non_exhaustive]` is good practice and allows future error variants to be added freely.

#### 2. `reader.rs:199-209` — `read_array()`

```rust
pub fn read_array<T>(
  &mut self,
  mut read_elem: impl FnMut(&mut Self) -> Result<T>,
) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;

  // Pre-validate: each element needs at least 1 byte
  if count > self.remaining() {
    return Err(DecodeError::UnexpectedEof {
      needed: count,
      available: self.remaining(),
    });
  }

  let mut items = Vec::new();
  items.try_reserve(count).map_err(|_| DecodeError::AllocationFailed)?;
  for _ in 0..count {
    items.push(read_elem(self)?);
  }
  Ok(items)
}
```

#### 3. `reader.rs:212-223` — `read_map()`

```rust
pub fn read_map<K: Eq + Hash, V>(
  &mut self,
  mut read_entry: impl FnMut(&mut Self) -> Result<(K, V)>,
) -> Result<HashMap<K, V>> {
  let count = self.read_u32()? as usize;

  // Pre-validate: each entry needs at least 2 bytes (1 key + 1 value minimum)
  if count > self.remaining() / 2 {
    return Err(DecodeError::UnexpectedEof {
      needed: count * 2,
      available: self.remaining(),
    });
  }

  let mut map = HashMap::new();
  map.try_reserve(count).map_err(|_| DecodeError::AllocationFailed)?;
  for _ in 0..count {
    let (k, v) = read_entry(self)?;
    map.insert(k, v);
  }
  Ok(map)
}
```

#### 4. `reader.rs:240-245` — `read_bytes()`

Already safe: `ensure(count)` is called before `to_vec()`, and `count` is bounded by buffer size. The `to_vec()` allocation is proportional to data actually present. However, we can add `try_reserve`:

```rust
pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
  self.ensure(count)?;
  // Buffer contains the data, so allocation size == data size (reasonable)
  let bytes = self.buf[self.pos..self.pos + count].to_vec();
  self.pos += count;
  Ok(bytes)
}
```

The `to_vec()` here is fine because `ensure()` already validated the buffer has `count` bytes. The allocation size matches the actual data size, so OOM would only occur if the system truly has no memory.

For defense-in-depth, we could use:

```rust
let mut bytes = Vec::new();
bytes.try_reserve(count).map_err(|_| DecodeError::AllocationFailed)?;
bytes.extend_from_slice(&self.buf[self.pos..self.pos + count]);
```

#### 5. `reader.rs:145-151` — `read_string()`

```rust
pub fn read_string(&mut self) -> Result<String> {
  let len = self.read_u32()? as usize;
  self.ensure(len + 1)?;
  let str_bytes = &self.buf[self.pos..self.pos + len];
  self.pos += len + 1;
  // ensure() guarantees len <= remaining, so this allocation is bounded by actual data
  String::from_utf8(str_bytes.to_vec()).map_err(|_| DecodeError::InvalidUtf8)
}
```

Already safe: the allocation size equals the actual string data size (validated by `ensure`). OOM here means system is truly out of memory, not a malicious payload.

#### 6. Writer — No Change for Phase 1

The writer is under the user's control (they choose what to encode), so OOM in the writer is the user's responsibility. The writer's allocation pattern is:

```rust
let mut writer = BebopWriter::with_capacity(self.encoded_size());
```

This pre-allocates exactly the right amount, so no reallocation occurs during encoding. If the initial allocation fails, that's a genuine OOM that can't be gracefully handled without fallible allocation on the writer side.

**Phase 2 (future):** Add `try_to_bytes()` and `try_encode()` methods that return `Result`.

### Summary of Pre-Validation Logic

| Method | Untrusted `count` | Pre-validation |
|---|---|---|
| `read_array()` | Yes (u32 from wire) | `count <= remaining()` |
| `read_map()` | Yes (u32 from wire) | `count <= remaining() / 2` |
| `read_scalar_array()` | Yes (u32 from wire) | `count * sizeof(T) <= remaining()` |
| `read_bytes()` | Depends on caller | Already bounded by `ensure()` |
| `read_string()` | Yes (u32 from wire) | Already bounded by `ensure(len+1)` |
| `read_byte_array()` | Yes (u32 from wire) | Calls `read_bytes` after `read_u32`, bounded by `ensure()` |

The key insight is that **strings and byte arrays are already safe** because `ensure()` validates the length against the buffer before allocation. The vulnerability is in **arrays and maps** where the count is used for `with_capacity` without verifying that the buffer could possibly contain that many elements.

## Cost-Benefit Analysis

### Benefits

- **Security:** Prevents denial-of-service via malicious payloads with enormous count values
- **Correctness:** `DecodeError` instead of panic for allocation failure
- **Embedded compatibility:** Graceful degradation instead of abort
- **Minimal API change:** `read_array`, `read_map` signatures unchanged, just different error behavior
- **Best-in-class:** Surpasses production Bebop Rust, C plugin, and Swift plugin for robustness

### Costs

- **New error variant:** `DecodeError::AllocationFailed`
- **Slight overhead:** One comparison per array/map decode (`count > remaining()`)
- **Incomplete:** Writer-side OOM still panics (deferred to Phase 2)
- **`try_reserve` overhead:** Negligibly more expensive than `with_capacity` (one extra branch in the allocator)

### Generated Code Size Impact

**Zero.** All changes are in the runtime. Generated code calls the same methods.

## Phase 2: Writer Fallibility (Future)

```rust
impl BebopWriter {
  pub fn try_write_byte(&mut self, v: u8) -> Result<(), EncodeError> {
    self.buf.try_reserve(1).map_err(|_| EncodeError::AllocationFailed)?;
    self.buf.push(v);
    Ok(())
  }
}

pub trait BebopEncode {
  fn encode(&self, writer: &mut BebopWriter);  // existing, panics
  fn try_encode(&self, writer: &mut BebopWriter) -> Result<(), EncodeError>;  // new, fallible
}
```

This is significantly more work because:
1. Every `push`/`extend_from_slice` in the writer needs a fallible counterpart
2. Generated `encode()` methods need fallible counterparts
3. The `BebopEncode` trait needs a new method

Deferring this to Phase 2 is appropriate because:
- Encode sizes are deterministic and controlled by the user
- The `with_capacity(encoded_size())` pattern means encode typically needs zero reallocations
- Decode is the more critical path for handling untrusted input

## Implementation Order

1. Add `#[non_exhaustive]` to `DecodeError` and add `AllocationFailed` variant
2. Add pre-validation to `read_array()` and `read_map()`
3. Replace `with_capacity` with `try_reserve` in `read_array()` and `read_map()`
4. Add tests for malicious payloads
5. (Future) Add `try_reserve` to `read_bytes()` and `read_string()` for defense-in-depth
6. (Future) Add fallible writer methods

## Test Plan

1. **Malicious array count:** Create payload with `count = 0xFFFFFFFF` for a `u64` array, verify `DecodeError::UnexpectedEof` (not panic)
2. **Malicious map count:** Same for maps
3. **Valid large array:** Create a payload with a large but valid array, verify successful decode
4. **Boundary:** Array count exactly equal to remaining bytes ÷ element size — should succeed
5. **Boundary + 1:** One more than boundary — should fail with UnexpectedEof
6. **Empty array/map:** `count = 0` — should still work
7. **AllocationFailed:** Mock or limit allocation to test try_reserve failure (harder to test, may need a custom global allocator in test)
8. **Overflow:** `count * sizeof(T)` overflows `usize` — should error, not panic
9. **Regression:** All existing round-trip tests still pass
