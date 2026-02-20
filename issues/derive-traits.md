# Add PartialEq / Hash Derives Where Possible

- [ ] Derive PartialEq, Eq, Hash on types that qualify #rust-plugin 🔽

Currently:
- **Enums**: derive `Debug, Clone, Copy, PartialEq, Eq` — correct
- **Flags**: derive `Debug, Clone, Copy, PartialEq, Eq` — correct
- **Structs**: derive `Debug, Clone` only
- **Messages**: derive `Debug, Clone, Default` only
- **Unions**: derive `Debug, Clone` only

Swift generates explicit `==` and `hash(into:)` for messages. Rust types without `PartialEq` can't be compared or used in assertions, which hurts ergonomics.

## Complication: Floats
Types containing `f32`, `f64`, `f16`, or `bf16` cannot derive `Eq` or `Hash` since floats don't implement those traits. Types with `Cow<[u8]>` can't derive Hash trivially either, though `[u8]` does implement Hash.

## Proposed Approach
During code generation, check if a type (transitively) contains any float fields:
- **No floats**: derive `PartialEq, Eq, Hash` in addition to `Debug, Clone`
- **Has floats**: derive `PartialEq` only (via the existing float `PartialEq` impls), skip `Eq` and `Hash`

This could reuse the `LifetimeAnalysis` pattern — do a pre-pass to determine which types contain floats.

## Alternative
Always derive `PartialEq` (it works for floats), and add `Eq + Hash` only for types without floats. This is the minimum useful improvement.
