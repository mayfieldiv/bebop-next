# FD-027: Message Builder Methods

**Status:** Planned
**Priority:** High
**Effort:** Medium (2-3 hours)
**Impact:** Dramatically improves construction ergonomics for the most common Bebop type

## Problem

Messages are Bebop's schema-evolution-friendly type and likely the most commonly used definition kind. But constructing them in Rust is verbose:

```rust
// Current: 7 lines for a 7-field message
let mut profile = UserProfile::default();
profile.display_name = Some(Cow::Owned("Alice".to_string()));
profile.email = Some(Cow::Owned("alice@example.com".to_string()));
profile.age = Some(30);
profile.active = Some(true);
profile.tags = Some(vec![Cow::Owned("admin".to_string())]);
profile.permissions = Some(Permissions::READ | Permissions::WRITE);
```

The pain comes from three layers of wrapping: `Some(...)`, `Cow::Owned(...)`, `.to_string()`. By contrast, structs have ergonomic `new()` with `impl Into<Cow>`.

## Solution

Two complementary APIs: a `new()` constructor for setting all fields at once, and chainable `with_*` setters for partial construction.

### 1. `new()` Constructor (matching struct pattern)

Structs already generate `new()` with `impl Into<Cow<'buf, str>>` for string fields, allowing `Person::new("Alice", 30)`. Messages should get the same — every field becomes a parameter wrapped in `Option`:

```rust
impl<'buf> UserProfile<'buf> {
    pub fn new(
        display_name: Option<impl Into<Cow<'buf, str>>>,
        email: Option<impl Into<Cow<'buf, str>>>,
        age: Option<u32>,
        active: Option<bool>,
        tags: Option<Vec<Cow<'buf, str>>>,
        metadata: Option<HashMap<Cow<'buf, str>, Cow<'buf, str>>>,
        permissions: Option<Permissions>,
    ) -> Self {
        Self {
            display_name: display_name.map(Into::into),
            email: email.map(Into::into),
            age,
            active,
            tags,
            metadata,
            permissions,
        }
    }
}
```

Usage:

```rust
// Set some fields, None for the rest
let profile = UserProfile::new(Some("Alice"), Some("alice@example.com"), Some(30), None, None, None, None);
```

This is ergonomic for messages with few fields. For messages with many fields, the positional `None`s become noisy — that's where `with_*` setters shine.

**Note:** `Option<impl Into<Cow>>` works because `Into<Cow<str>>` is implemented for `&str` and `String`, and `Option<T>` can accept `Some("Alice")` directly — the `impl Into` applies inside the `Some`. For non-string/bytes fields, the param is just `Option<T>` directly with no conversion trait needed.

### 2. Chainable `with_*` Setters

```rust
impl<'buf> UserProfile<'buf> {
    pub fn with_display_name(mut self, value: impl Into<Cow<'buf, str>>) -> Self {
        self.display_name = Some(value.into());
        self
    }
    pub fn with_email(mut self, value: impl Into<Cow<'buf, str>>) -> Self {
        self.email = Some(value.into());
        self
    }
    pub fn with_age(mut self, value: u32) -> Self {
        self.age = Some(value);
        self
    }
    // ... one per field
}
```

Usage:

```rust
let profile = UserProfile::default()
    .with_display_name("Alice")
    .with_email("alice@example.com")
    .with_age(30)
    .with_active(true)
    .with_permissions(Permissions::READ | Permissions::WRITE);
```

### Design Details

**`new()` constructor:**
- One parameter per field, each wrapped in `Option<...>`
- String/bytes fields: `Option<impl Into<Cow<'buf, str>>>` / `Option<impl Into<Cow<'buf, [u8]>>>`
- Scalar/enum/struct fields: `Option<T>` directly
- Internally calls `.map(Into::into)` for conversion fields, passes through for others

**`with_*` setters:**
- Method name: `with_<field_name>` (avoids collision with `set_` which implies `&mut self`)
- Signature: `(mut self, value: T) -> Self` for chainability
- String/bytes fields: accept `impl Into<Cow<'buf, str>>` / `impl Into<Cow<'buf, [u8]>>`
- Scalar fields: accept the raw type (no wrapping needed)
- Array fields: accept `Vec<T>` directly
- Map fields: accept `HashMap<K, V>` directly
- Nested message/struct fields: accept the type directly
- All setters wrap in `Some(...)` internally

### What NOT to generate

- No separate builder type (avoids type explosion)
- No `build()` method (the struct IS the result)
- No validation (messages are inherently partial)

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src/generator/gen_message.rs` | MODIFY | Generate `new()` constructor and `with_*` setter methods |
| `src/generator/type_mapper.rs` | POSSIBLY MODIFY | May need `into_param_type()` for setter signatures |
| `integration-tests/schemas/test_types.bop` | NO CHANGE | Existing messages suffice for testing |
| `integration-tests/tests/integration.rs` | MODIFY | Add tests using builder syntax |

## Verification

- Generated `with_*` methods compile for all message types in test_types.bop
- Integration tests verify round-trip with builder-constructed messages
- no_std integration tests verify builder works without std
- `./test.sh` passes

## Design Notes

- This pattern is used by `prost` for protobuf messages and `serde_json::Map`
- The `with_` prefix is conventional in Rust for consuming builder methods
- Since `Default` is already derived on messages, `UserProfile::default().with_name("x")` is the natural construction idiom
