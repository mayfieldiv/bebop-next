# FD-008: OOM Handling (Fallible Allocation)

**Status:** Open
**Priority:** High
**Effort:** Medium (2-4 hours)
**Impact:** Security hardening — prevents DoS via malicious payloads with enormous count values

## Problem

The Rust runtime panics on OOM because `Vec::with_capacity` and `HashMap::with_capacity` use infallible allocation. A malicious payload with `count = 0xFFFFFFFF` for a `u64` array triggers a 32 GiB allocation attempt, causing panic or process kill.

This vulnerability exists in: production Bebop Rust runtime, C plugin (with integer overflow too), and Swift plugin (for variable-size elements). Only Swift's scalar array path pre-validates via `ensureBytes()`.

## Scope

**Phase 1 only** — decode path (handles untrusted input). Writer fallibility is deferred because encode sizes are deterministic and user-controlled.

Strings, byte arrays, and byte slices are allocation-safe (bounded by `ensure()`), but `ensure()` itself and two callers have arithmetic overflow bugs on 32-bit targets that can bypass the bounds check. These are fixed as part of this FD. Fixed arrays (`read_fixed_array`) use stack allocation with compile-time size, so they are not affected.

The runtime is `#![no_std]` and uses `hashbrown::HashMap` (re-exported via `lib.rs`). `hashbrown::HashMap::try_reserve` is stable and available, so the fallible allocation approach works without `std`.

## Solution: Pre-validate + Fallible Allocation (Defense in Depth)

Two layers of protection:

### Layer 1: Pre-validation (fast-fail)

Before allocating, check that the decoded `count` is plausible given the remaining buffer bytes. This catches absurdly large counts instantly at zero cost.

- **`read_array`:** `count <= self.remaining()` — an element is at minimum 1 byte, so there cannot be more elements than remaining bytes. Fails with existing `DecodeError::UnexpectedEof`.
- **`read_map`:** `count <= self.remaining() / 2` — a map entry is at minimum 2 bytes (one key byte + one value byte). Fails with existing `DecodeError::UnexpectedEof`.

This is a heuristic, not an exact bound. For a `u64` array with 1000 bytes remaining, it allows `count=1000` even though only 125 elements fit. The over-allocation is bounded by the input buffer size (not a DoS vector), and Layer 2 is the real safety net.

### Layer 2: Fallible allocation (safety net)

Replace `Vec::with_capacity(count)` / `HashMap::with_capacity(count)` with:

```rust
let mut items = Vec::new();
items
  .try_reserve(count)
  .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
```

This catches the case where the count passes pre-validation but the system is genuinely low on memory. Returns a `Result::Err` instead of panicking.

### Error mapping

| Condition | Error |
|-----------|-------|
| `count > remaining()` (array) or `count > remaining() / 2` (map) | `DecodeError::UnexpectedEof` (existing) |
| `try_reserve` / `try_reserve_exact` returns `Err` | `DecodeError::AllocationFailed` (new) |

## Changes

### `runtime/src/error.rs`

Add one variant to `DecodeError`:

```rust
AllocationFailed { requested: usize },
```

The `requested` field carries the element count that failed, useful for diagnostics. The existing `#[non_exhaustive]` attribute makes this addition semver-safe.

Add a `Display` arm:

```rust
Self::AllocationFailed { requested } => {
  write!(f, "allocation failed for {} elements", requested)
}
```

### `runtime/src/reader.rs`

**`ensure` becomes (overflow-safe):**

The existing `self.pos + count > self.buf.len()` overflows when `pos + count` wraps on 32-bit targets. Fix with `checked_add`:

```rust
fn ensure(&self, count: usize) -> Result<()> {
  match self.pos.checked_add(count) {
    Some(end) if end <= self.buf.len() => Ok(()),
    _ => Err(DecodeError::UnexpectedEof {
      needed: count,
      available: self.remaining(),
    }),
  }
}
```

**`read_string` and `read_str` — overflow-safe `len + 1`:**

When `len = u32::MAX` on a 32-bit target, `len + 1` wraps to 0, causing `ensure(0)` to pass trivially, then the subsequent slice panics. Fix with `checked_add`:

```rust
pub fn read_string(&mut self) -> Result<String> {
  let len = self.read_u32()? as usize;
  let total = len.checked_add(1).ok_or(DecodeError::UnexpectedEof {
    needed: usize::MAX,
    available: self.remaining(),
  })?;
  self.ensure(total)?;
  let str_bytes = &self.buf[self.pos..self.pos + len];
  self.pos += total;
  String::from_utf8(str_bytes.to_vec()).map_err(|_| DecodeError::InvalidUtf8)
}
```

**`read_str` becomes (zero-copy variant):**

```rust
pub fn read_str(&mut self) -> Result<&'a str> {
  let len = self.read_u32()? as usize;
  let total = len.checked_add(1).ok_or(DecodeError::UnexpectedEof {
    needed: usize::MAX,
    available: self.remaining(),
  })?;
  self.ensure(total)?;
  let str_bytes = &self.buf[self.pos..self.pos + len];
  self.pos += total;
  core::str::from_utf8(str_bytes).map_err(|_| DecodeError::InvalidUtf8)
}
```

**`read_array` becomes:**

```rust
pub fn read_array<T>(
  &mut self,
  mut read_elem: impl FnMut(&mut Self) -> Result<T>,
) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;
  if count > self.remaining() {
    return Err(DecodeError::UnexpectedEof {
      needed: count,
      available: self.remaining(),
    });
  }
  let mut items = Vec::new();
  items
    .try_reserve(count)
    .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
  for _ in 0..count {
    items.push(read_elem(self)?);
  }
  Ok(items)
}
```

**`read_map` becomes:**

```rust
pub fn read_map<K: Eq + Hash, V>(
  &mut self,
  mut read_entry: impl FnMut(&mut Self) -> Result<(K, V)>,
) -> Result<HashMap<K, V>> {
  let count = self.read_u32()? as usize;
  if count > self.remaining() / 2 {
    return Err(DecodeError::UnexpectedEof {
      needed: count.saturating_mul(2),
      available: self.remaining(),
    });
  }
  let mut map = HashMap::new();
  map
    .try_reserve(count)
    .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
  for _ in 0..count {
    let (k, v) = read_entry(self)?;
    map.insert(k, v);
  }
  Ok(map)
}
```

### What doesn't change

- **Generated code** — the generator calls `reader.read_array(...)` and `reader.read_map(...)`, so it picks up the fix automatically.
- **Public API signatures** — `read_array` and `read_map` already return `Result<T>`.
- **Writer/encode path** — deferred to Phase 2.
- **Fixed arrays** — compile-time size, no wire-controlled allocation.
- **`advance()`, `read_bytes`, `read_raw_bytes`** — these contain `self.pos + count` indexing but are always preceded by `ensure()`, which now rejects overflowing values via `checked_add`. They are transitively safe.

## Verification

| Test | Input | Expected |
|------|-------|----------|
| Malicious array count | `count = 0xFFFFFFFF`, buffer = 100 bytes | `DecodeError::UnexpectedEof` |
| Malicious map count | `count = 0xFFFFFFFF`, buffer = 100 bytes | `DecodeError::UnexpectedEof` |
| Boundary: array count = remaining | count matching remaining bytes exactly | Succeeds (elements decode or fail individually) |
| Boundary + 1: array | count = remaining + 1 | `DecodeError::UnexpectedEof` |
| Boundary: map count = remaining / 2 | count matching remaining / 2 | Succeeds |
| Boundary + 1: map | count = remaining / 2 + 1 | `DecodeError::UnexpectedEof` |
| Empty collection | `count = 0` | Succeeds (empty Vec / HashMap) |
| String len = `u32::MAX` (32-bit) | `len + 1` would wrap to 0 | `DecodeError::UnexpectedEof` (not panic) |
| `ensure` with pos + count overflow | Large count near `usize::MAX` | `DecodeError::UnexpectedEof` (not panic) |
| All existing round-trip tests | Unchanged | Pass |

## Source

Migrated from `../../issues/plans/oom-handling.md`
