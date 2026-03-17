# FD-028: Map Key Accessor Helpers

**Status:** Planned
**Priority:** High
**Effort:** Low (< 1 hour)
**Impact:** Eliminates the most common Cow ergonomic pain point

## Problem

Looking up a value in a `HashMap<Cow<'buf, str>, V>` field requires verbose casting:

```rust
// Current: painful
items[&Cow::Borrowed("sword") as &Cow<str>]

// What users expect to write:
items["sword"]
```

This is because `HashMap<Cow<str>, V>` doesn't implement `Index<&str>` — `Index` requires the exact key type. The `HashMap::get` method is slightly better via `Borrow` trait but still not great:

```rust
// Works but non-obvious — requires knowing Cow<str>: Borrow<str>
items.get("sword")  // actually works! HashMap::get uses Borrow<Q>
```

The `get()` method actually works with `&str` keys because `Cow<str>` implements `Borrow<str>`. But the `[]` index operator does not, because `Index` trait requires `Q: Eq + Hash` where `Q` is the key type exactly.

## Solution

Two complementary approaches:

### 1. Runtime: NewType Map with `Index<&str>`

Add a `BebopMap<K, V>` wrapper in the runtime that delegates to `HashMap` but also implements `Index<&str>` when `K: Borrow<str>`:

```rust
pub struct BebopMap<K, V>(pub HashMap<K, V>);

impl<V> Index<&str> for BebopMap<Cow<'_, str>, V> {
    type Output = V;
    fn index(&self, key: &str) -> &V {
        self.0.get(key).expect("key not found")
    }
}
```

**Downside:** This is a new type that wraps HashMap, adding complexity. Users would need to learn about BebopMap.

### 2. Generated: Field accessor methods (simpler)

Generate accessor methods on messages/structs that have map fields with string keys:

```rust
impl<'buf> Inventory<'buf> {
    pub fn get_item(&self, key: &str) -> Option<&u32> {
        self.items.as_ref().and_then(|m| m.get(key))
    }
}
```

This handles both the `Option` unwrapping (for message fields) and the `&str` key lookup in one call. The method name follows the pattern `get_<field_name_singular>` or just `get_<field_name>`.

### Recommendation

Start with approach 2 (generated accessors). It's simpler, doesn't introduce new types, and also handles the `Option` layer on message fields. Approach 1 can be added later if there's demand for direct `[]` indexing.

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src/generator/gen_message.rs` | MODIFY | Generate `get_*` accessor for map fields |
| `src/generator/gen_struct.rs` | MODIFY | Generate `get_*` accessor for map fields |
| `integration-tests/tests/integration.rs` | MODIFY | Test map accessor methods |

## Verification

- Generated accessors compile for all map-keyed fields in test_types.bop
- Integration tests verify lookup with `&str` keys
- `./test.sh` passes
