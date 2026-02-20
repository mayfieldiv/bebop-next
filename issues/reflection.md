# Implement Reflection Metadata

- [ ] Implement reflection metadata generation #rust-plugin 🔽 🆔 reflection

Both C and Swift generate per-type reflection metadata. The Rust plugin generates none. This is needed for runtime introspection, debugging, and service dispatch.

## What C Generates
`BebopReflection_DefinitionDescriptor` structs with: name, FQN, kind, field count, field descriptors (name, tag, type info, constant index, deprecated flag, offset). Optionally disabled via `no_reflection` option.

## What Swift Generates
`BebopTypeReflection` static constants on each type:
```swift
static let bebopReflection = BebopTypeReflection(
    name: "Foo", fqn: "package.Foo", kind: .struct,
    fields: [
        BebopFieldReflection(name: "bar", tag: 1, typeUrl: "uint32"),
        ...
    ]
)
```

## Proposed Rust Approach
Add a `BebopReflectable` trait to the runtime:
```rust
pub trait BebopReflectable {
    fn bebop_reflection() -> &'static BebopTypeReflection;
}

pub struct BebopTypeReflection {
    pub name: &'static str,
    pub fqn: &'static str,
    pub kind: BebopTypeKind, // Enum, Struct, Message, Union, Service
    pub fields: &'static [BebopFieldReflection],
}

pub struct BebopFieldReflection {
    pub name: &'static str,
    pub tag: Option<u8>,       // Some for messages, None for structs
    pub type_url: &'static str,
    pub deprecated: bool,
}
```

Generate `impl BebopReflectable for Foo { ... }` for each type. Consider a `no_reflection` option like C.

## Dependencies
- Blocked by: nothing (can be done independently)
- Related: [[services-rpc]] (services need reflection for method dispatch metadata)
