# Implement Visibility (pub vs pub(crate)) Support

- [ ] Respect definition visibility in generated code #rust-plugin 🔽

The descriptor has a `Visibility` enum (`Default`, `Export`, `Local`) on each `DefinitionDescriptor`. The whitepaper specifies:
- Top-level definitions default to exported (`pub`)
- Nested definitions default to local (`pub(crate)` or private)
- `export` keyword on nested types makes them `pub`
- `local` keyword on top-level types makes them module-private

Currently the Rust generator emits `pub` for everything unconditionally. The Swift plugin maps visibility to `public` vs `internal` (configurable). The C plugin uses a configurable visibility prefix.

## Proposed Mapping
| Bebop Visibility | Rust Output |
|---|---|
| `Export` / `Default` (top-level) | `pub` |
| `Local` | `pub(crate)` |

## Implementation
Check `def.visibility` in each generator (`gen_struct`, `gen_message`, `gen_enum`, `gen_union`) and emit `pub(crate)` instead of `pub` for `Visibility::Local` definitions and their fields.
