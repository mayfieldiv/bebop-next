# FD-008: OOM Handling Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the Bebop Rust runtime decode path against OOM DoS attacks and arithmetic overflow panics.

**Architecture:** Two-layer defense in depth — pre-validate collection counts against buffer size (fast-fail), then use fallible allocation (`try_reserve`) as a safety net. Additionally fix arithmetic overflow in `ensure()`, `read_string`, and `read_str` using `checked_add`.

**Tech Stack:** Rust (`no_std`), `hashbrown::HashMap`, `alloc` crate

**Spec:** `docs/features/FD-008_OOM_HANDLING.md`

---

## Chunk 1: Error Variant and Overflow Fixes

### Task 1: Add `AllocationFailed` variant to `DecodeError`

**Files:**
- Modify: `runtime/src/error.rs:5-27` (enum) and `runtime/src/error.rs:29-56` (Display impl)

- [ ] **Step 1: Add the variant**

In `runtime/src/error.rs`, add `AllocationFailed` after the last existing variant (`InvalidFlags`):

```rust
  InvalidFlags {
    type_name: &'static str,
    bits: u64,
  },
  AllocationFailed {
    requested: usize,
  },
```

- [ ] **Step 2: Add the Display arm**

In the `fmt::Display` impl's `match` block, add after the `InvalidFlags` arm:

```rust
      Self::AllocationFailed { requested } => {
        write!(f, "allocation failed for {} elements", requested)
      }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p bebop-runtime`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add runtime/src/error.rs
git commit -m "feat: add DecodeError::AllocationFailed variant"
```

### Task 2: Fix `ensure()` overflow with `checked_add`

**Files:**
- Modify: `runtime/src/reader.rs:36-45`

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)]` module at the bottom of `runtime/src/reader.rs`:

```rust
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ensure_overflow_returns_error() {
    // A reader with a small buffer, positioned partway through.
    // Requesting usize::MAX bytes must not wrap and falsely succeed.
    let buf = [0u8; 16];
    let mut reader = BebopReader::new(&buf);
    reader.pos = 8;
    let result = reader.skip(usize::MAX);
    assert!(result.is_err(), "ensure must reject count that overflows pos + count");
  }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bebop-runtime --lib reader::tests::ensure_overflow_returns_error`
Expected: FAIL — `8_usize.wrapping_add(usize::MAX)` = 7, which is <= 16 (buf.len()), so `ensure` falsely succeeds and `skip` returns `Ok`. The test assertion `is_err()` fails, confirming the bug.

- [ ] **Step 3: Replace ensure with checked_add version**

In `runtime/src/reader.rs`, replace lines 36-45:

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

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p bebop-runtime --lib reader::tests::ensure_overflow_returns_error`
Expected: PASS

- [ ] **Step 5: Run all existing tests to check for regressions**

Run: `cargo test -p bebop-runtime`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add runtime/src/reader.rs
git commit -m "fix: use checked_add in ensure() to prevent overflow on 32-bit"
```

### Task 3: Fix `read_string` and `read_str` overflow

**Files:**
- Modify: `runtime/src/reader.rs:145-151` (`read_string`) and `runtime/src/reader.rs:257-262` (`read_str`)

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `runtime/src/reader.rs`:

```rust
  #[test]
  fn read_string_len_overflow_returns_error() {
    // Craft a buffer where the u32 length field is 0xFFFFFFFF.
    // On 32-bit targets, len + 1 wraps to 0. ensure(0) would falsely pass.
    // On 64-bit, ensure(0x100000000) correctly fails, but we still want the
    // checked_add path exercised.
    let mut buf = Vec::new();
    buf.extend_from_slice(&u32::MAX.to_le_bytes()); // len = 0xFFFFFFFF
    buf.extend_from_slice(&[0u8; 16]); // some trailing data
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_string();
    assert!(result.is_err(), "read_string must reject len=u32::MAX");
  }

  #[test]
  fn read_str_len_overflow_returns_error() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&u32::MAX.to_le_bytes());
    buf.extend_from_slice(&[0u8; 16]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_str();
    assert!(result.is_err(), "read_str must reject len=u32::MAX");
  }
```

- [ ] **Step 2: Run tests to verify they pass (on 64-bit, ensure already rejects large counts)**

Run: `cargo test -p bebop-runtime --lib reader::tests::read_string_len_overflow_returns_error reader::tests::read_str_len_overflow_returns_error`
Expected: PASS on 64-bit (ensure rejects `0x100000000`). These tests document the expected behavior and will protect 32-bit targets after the fix.

- [ ] **Step 3: Apply checked_add fix to read_string**

Replace `read_string` (lines 145-151):

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

- [ ] **Step 4: Apply checked_add fix to read_str**

Replace `read_str` (lines 257-262):

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

- [ ] **Step 5: Run all tests**

Run: `cargo test -p bebop-runtime`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add runtime/src/reader.rs
git commit -m "fix: use checked_add in read_string/read_str to prevent len+1 overflow"
```

## Chunk 2: Collection Hardening

### Task 4: Harden `read_array` with pre-validation and fallible allocation

**Files:**
- Modify: `runtime/src/reader.rs:198-209`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `runtime/src/reader.rs`:

```rust
  #[test]
  fn read_array_malicious_count_returns_eof() {
    // count = 0xFFFFFFFF but buffer only has 100 bytes after the count field
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 100]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_array(|r| r.read_u32());
    assert!(result.is_err());
    match result.unwrap_err() {
      DecodeError::UnexpectedEof { .. } => {} // expected
      other => panic!("expected UnexpectedEof, got {:?}", other),
    }
  }

  #[test]
  fn read_array_count_boundary_plus_one_returns_eof() {
    // Buffer has 8 bytes after count field. count = 9 (one more than remaining).
    let mut buf = Vec::new();
    buf.extend_from_slice(&9u32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 8]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_array(|r| r.read_byte());
    assert!(result.is_err());
  }

  #[test]
  fn read_array_count_equals_remaining_succeeds() {
    // Buffer has exactly 3 bytes after count field. count = 3, read 3 bytes.
    let mut buf = Vec::new();
    buf.extend_from_slice(&3u32.to_le_bytes());
    buf.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_array(|r| r.read_byte());
    assert_eq!(result.unwrap(), vec![0xAA, 0xBB, 0xCC]);
  }

  #[test]
  fn read_array_empty_succeeds() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u32.to_le_bytes());
    let mut reader = BebopReader::new(&buf);
    let result: core::result::Result<Vec<u8>, _> = reader.read_array(|r| r.read_byte());
    assert_eq!(result.unwrap(), Vec::<u8>::new());
  }
```

- [ ] **Step 2: Run tests to verify the malicious count test fails (panics or OOM)**

Run: `cargo test -p bebop-runtime --lib reader::tests::read_array_malicious_count_returns_eof -- --nocapture 2>&1 | head -20`
Expected: FAIL (the test will likely OOM or take a very long time trying to allocate)

Note: The boundary and empty tests may already pass since they use small counts. The malicious count test is the key one that fails before the fix.

- [ ] **Step 3: Replace read_array implementation**

Replace `read_array` (lines 198-209):

```rust
  /// Read a dynamic array: u32 count + elements.
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

- [ ] **Step 4: Run all reader tests**

Run: `cargo test -p bebop-runtime --lib reader::tests`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add runtime/src/reader.rs
git commit -m "fix: pre-validate and use try_reserve in read_array"
```

### Task 5: Harden `read_map` with pre-validation and fallible allocation

**Files:**
- Modify: `runtime/src/reader.rs:211-223` (line numbers will have shifted from Task 4)

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `runtime/src/reader.rs`:

```rust
  #[test]
  fn read_map_malicious_count_returns_eof() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 100]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_map(|r| {
      let k = r.read_byte()?;
      let v = r.read_byte()?;
      Ok((k, v))
    });
    assert!(result.is_err());
    match result.unwrap_err() {
      DecodeError::UnexpectedEof { .. } => {}
      other => panic!("expected UnexpectedEof, got {:?}", other),
    }
  }

  #[test]
  fn read_map_count_boundary_plus_one_returns_eof() {
    // Buffer has 4 bytes after count. Minimum entry = 2 bytes, so max count = 2.
    // count = 3 should fail.
    let mut buf = Vec::new();
    buf.extend_from_slice(&3u32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 4]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_map(|r| {
      let k = r.read_byte()?;
      let v = r.read_byte()?;
      Ok((k, v))
    });
    assert!(result.is_err());
  }

  #[test]
  fn read_map_count_equals_boundary_succeeds() {
    // Buffer has 4 bytes after count. count = 2, each entry = 2 bytes (1 key + 1 value).
    let mut buf = Vec::new();
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf.extend_from_slice(&[1, 10, 2, 20]); // key=1,val=10, key=2,val=20
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_map(|r| {
      let k = r.read_byte()?;
      let v = r.read_byte()?;
      Ok((k, v))
    });
    let map = result.unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map[&1u8], 10u8);
    assert_eq!(map[&2u8], 20u8);
  }

  #[test]
  fn read_map_empty_succeeds() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u32.to_le_bytes());
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_map(|r| {
      let k = r.read_byte()?;
      let v = r.read_byte()?;
      Ok((k, v))
    });
    assert_eq!(result.unwrap().len(), 0);
  }
```

- [ ] **Step 2: Run malicious count test to verify it fails**

Run: `cargo test -p bebop-runtime --lib reader::tests::read_map_malicious_count_returns_eof -- --nocapture 2>&1 | head -20`
Expected: FAIL (OOM or timeout)

- [ ] **Step 3: Replace read_map implementation**

Replace `read_map`:

```rust
  /// Read a dynamic map: u32 count + (key, value) pairs.
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

- [ ] **Step 4: Run all reader tests**

Run: `cargo test -p bebop-runtime --lib reader::tests`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add runtime/src/reader.rs
git commit -m "fix: pre-validate and use try_reserve in read_map"
```

## Chunk 3: Full Validation

### Task 6: Run full test suite and clippy

**Files:** None (validation only)

- [ ] **Step 1: Run all runtime tests**

Run: `cargo test -p bebop-runtime`
Expected: all pass

- [ ] **Step 2: Run integration tests**

Run: `cargo test`
Expected: all pass (generated code uses `read_array`/`read_map` — confirms no regressions)

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p bebop-runtime -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Run formatter**

Run: `cargo fmt -p bebop-runtime -- --check`
Expected: no formatting issues (fix with `cargo fmt` if needed)

- [ ] **Step 5: Run full validation script**

Run: `./test.sh`
Expected: all checks pass

- [ ] **Step 6: Update FD-008 status to In Progress**

In `docs/features/FD-008_OOM_HANDLING.md`, change line 3:
```
**Status:** In Progress
```

Update `docs/features/FEATURE_INDEX.md` to match.

- [ ] **Step 7: Commit**

```bash
git add docs/features/FD-008_OOM_HANDLING.md docs/features/FEATURE_INDEX.md
git commit -m "FD-008: mark status In Progress"
```
