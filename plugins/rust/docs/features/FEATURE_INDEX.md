# Feature Design Index

Planned features and improvements for bebopc-gen-rust.

See `CLAUDE.md` for FD lifecycle stages and management guidelines.

## Active Features

| FD | Title | Status | Effort | Priority |
|----|-------|--------|--------|----------|
| FD-001 | [Service Generation](FD-001_SERVICE_GENERATION.md) | Planned | High | High |
| FD-002 | [Deprecated Field Handling](FD-002_DEPRECATED_FIELD_HANDLING.md) | Planned | Low | Medium |
| FD-009 | [Reflection Metadata](FD-009_REFLECTION_METADATA.md) | Planned | Medium | Low |
| FD-025 | [Forward-Compat Message Semantics](FD-025_FORWARD_COMPAT_MESSAGE_SEMANTICS.md) | Design | Low | Medium |
| FD-026 | [Optional UUID/Half Features](FD-026_OPTIONAL_UUID_HALF_FEATURES.md) | Planned | Medium | Medium |
| FD-027 | [Message Builder Methods](FD-027_MESSAGE_BUILDER_METHODS.md) | Planned | Medium | High |
| FD-028 | [Map Key Accessor Helpers](FD-028_MAP_KEY_ACCESSOR_HELPERS.md) | Planned | Low | High |
| FD-029 | [Display for Enums & Flags](FD-029_DISPLAY_FOR_ENUMS_AND_FLAGS.md) | Planned | Low | Medium |
| FD-030 | [DecodeError Field Context](FD-030_DECODE_ERROR_FIELD_CONTEXT.md) | Planned | Medium | Medium |
| FD-031 | [Deprecated Attribute Emission](FD-031_DEPRECATED_ATTRIBUTE_EMISSION.md) | Planned | Low | Medium |
| FD-032 | [Serde Round-Trip Test Coverage](FD-032_SERDE_ROUNDTRIP_TEST_COVERAGE.md) | Planned | Low | High |

## Completed

| FD | Title | Completed | Notes |
|----|-------|-----------|-------|
| FD-023 | [Zero-Copy Scalar Array Decode](archive/FD-023_ZERO_COPY_SCALAR_ARRAY_DECODE.md) | 2026-03 | Zero-copy `Cow<'buf, [T]>` decode for scalar arrays; 300x decode improvement on TensorShard |
| FD-010 | [Zero-Copy Bulk Arrays](archive/FD-010_ZERO_COPY_BULK_ARRAYS.md) | 2026-03 | Bulk memcpy encode/decode for scalar arrays on LE; `BulkScalar` trait excludes bool |
| FD-024 | [Plugin Parameter Support](archive/FD-024_PLUGIN_PARAMETER_SUPPORT.md) | 2026-03 | `--<name>_opt=PARAM` CLI support in bebopc; unblocks serde in generate.sh |
| FD-008 | [OOM Handling](archive/FD-008_OOM_HANDLING.md) | 2026-03 | Pre-validation + fallible allocation for read_array/read_map; overflow fixes in ensure/read_string/read_str |
| FD-022 | [Cross-Language Benchmark Expansion](archive/FD-022_CROSS_LANGUAGE_BENCHMARKS.md) | 2026-03 | All 23 scenarios with golden file cross-language verification |
| FD-003 | [SerdeMode](archive/FD-003_SERDE_MODE.md) | 2026-03 | Always/FeatureGated/Disabled serde control |
| FD-007 | [Enum Forward Compat](archive/FD-007_ENUM_FORWARD_COMPAT.md) | 2026-03 | `@forward_compatible` decorator, Unknown(T) variant |
| FD-013 | [Dead Code Cleanup](archive/FD-013_DEAD_CODE_CLEANUP.md) | 2026-02 | Removed dead non-Cow generation paths |
| FD-014 | [Derive Traits](archive/FD-014_DERIVE_TRAITS.md) | 2026-02 | PartialEq/Eq/Hash with float/map awareness |
| FD-015 | [Fixed Array Generics](archive/FD-015_FIXED_ARRAY_GENERICS.md) | 2026-02 | Generic `FixedScalar` read/write for all types |
| FD-016 | [Generator Dogfooding](archive/FD-016_GENERATOR_DOGFOODING.md) | 2026-02 | Qualified paths, no_implicit_prelude, no patching |
| FD-017 | [Insertion Points](archive/FD-017_INSERTION_POINTS.md) | 2026-02 | All marker families matching C/Swift |
| FD-018 | [Rename Cow Functions](archive/FD-018_RENAME_COW_FUNCTIONS.md) | 2026-02 | Removed _cow suffix after dead code cleanup |
| FD-019 | [Temporal Newtypes](archive/FD-019_TEMPORAL_NEWTYPES.md) | 2026-02 | BebopTimestamp/BebopDuration structs |
| FD-020 | [UUID Integration](archive/FD-020_UUID_INTEGRATION.md) | 2026-02 | Feature-gated uuid crate conversions |
| FD-021 | [Visibility Support](archive/FD-021_VISIBILITY_SUPPORT.md) | 2026-02 | pub vs pub(crate) from descriptor |

## Deferred / Closed

| FD | Title | Status | Notes |
|----|-------|--------|-------|
| FD-011 | [Immutability Support](archive/FD-011_IMMUTABILITY_SUPPORT.md) | Closed | No change needed — Rust ownership model handles this |
| FD-012 | [Struct Layout Optimization](archive/FD-012_STRUCT_LAYOUT_OPTIMIZATION.md) | Closed | No change needed — Rust compiler already reorders fields |

## Backlog

Low-priority or blocked items. Promote to Active when ready to design.

| FD | Title | Notes |
|----|-------|-------|
| FD-004 | Nested module generation | Flatten inline defs into same scope; could use nested Rust modules instead |
| FD-005 | Multi-file output per schema | Currently 1 `.bop` → 1 `.rs`; option to split generated code |
| FD-006 | Fixed array non-scalar Default requirement | Non-scalar fixed arrays require `Default` on element type; could use `MaybeUninit` |
