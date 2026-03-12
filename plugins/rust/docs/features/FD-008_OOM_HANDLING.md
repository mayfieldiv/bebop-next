# FD-008: OOM Handling (Fallible Allocation)

**Status:** Open
**Priority:** High
**Effort:** Medium (2-4 hours)
**Impact:** Security hardening — prevents DoS via malicious payloads with enormous count values

## Problem

The Rust runtime panics on OOM because `Vec::with_capacity` and `HashMap::with_capacity` use infallible allocation. A malicious payload with `count = 0xFFFFFFFF` for a `u64` array triggers a 32 GiB allocation attempt, causing panic or process kill.

This vulnerability exists in: production Bebop Rust runtime, C plugin (with integer overflow too), and Swift plugin (for variable-size elements). Only Swift's scalar array path pre-validates via `ensureBytes()`.

## Solution

**Option D: Pre-validate + fallible allocation (defense in depth)**

Phase 1 (decode path — handles untrusted input):
1. Add `DecodeError::AllocationFailed` variant (+ `#[non_exhaustive]`)
2. Pre-validate `count <= remaining()` in `read_array()` before allocating
3. Pre-validate `count <= remaining() / 2` in `read_map()`
4. Replace `Vec::with_capacity(count)` with `Vec::new()` + `try_reserve(count)`
5. Same for `HashMap`

Phase 2 (future — writer fallibility):
- Add `try_to_bytes()` / `try_encode()` methods
- Deferred because encode sizes are deterministic and user-controlled

Strings and byte arrays are already safe (`ensure()` bounds the allocation to actual data size).

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `runtime/src/error.rs` | MODIFY | Add `AllocationFailed`, `#[non_exhaustive]` |
| `runtime/src/reader.rs` | MODIFY | Pre-validate + `try_reserve` in `read_array`, `read_map` |

## Verification

- Malicious array count (`0xFFFFFFFF`) returns `DecodeError`, not panic
- Malicious map count returns `DecodeError`, not panic
- Boundary: count exactly fitting remaining bytes succeeds
- Boundary + 1 fails with `UnexpectedEof`
- `count * sizeof(T)` overflow returns error, not panic
- Empty array/map (`count = 0`) still works
- All existing round-trip tests pass

## Source

Migrated from `../../issues/plans/oom-handling.md`
