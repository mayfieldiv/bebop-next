# FD-024: Plugin Parameter Support in bebopc

**Status:** Complete
**Completed:** 2026-03-17
**Priority:** Medium
**Effort:** Low (< 1 hour)
**Impact:** Enables language-specific plugin options (e.g. serde mode) via CLI, unblocking reliable code regeneration

## Problem

The plugin protocol (`plugin.bop`) defines a `parameter` field documented as:

> Generator-specific parameter from `--${NAME}_opt=PARAM` or embedded in the output path. Format is plugin-defined (commonly key=value pairs).

But this mechanism was never wired up. Currently `bebopc_runner.c:299` hardcodes:

```c
bebop_plugin_request_builder_set_parameter(builder, gen->out_dir);
```

So the `parameter` field always contains the output directory path. There is no `--<name>_opt=PARAM` CLI syntax, no config file field for it, and no way to pass language-specific parameters to a plugin separate from host options (`-D`).

This means:
- The Rust plugin's parameter parser (which expects comma-separated tokens like `serde`) is dead code — it never receives anything except a directory path
- `generate.sh` scripts cannot pass serde mode to the Rust plugin
- Host options (`-D`) are cross-language and go to ALL plugins, which is wrong for language-specific settings like serde mode

## Solution

Add `--<name>_opt=PARAM` CLI support in bebopc, mirroring the existing `--<name>_out=DIR` pattern.

**bebopc changes:**

| File | Change |
|------|--------|
| `bebopc/src/cli/bebopc_cli.c` | Add `_try_parse_plugin_opt()` alongside `_try_parse_plugin_out()` |
| `bebopc/src/cli/bebopc_config.h` | Add `char* parameter` field to `bebopc_plugin_t` |
| `bebopc/src/cli/bebopc_runner.c:299` | Use `gen->parameter` if set, else fall back to `gen->out_dir` |
| `bebopc/src/cli/bebopc_config.c` | Support `parameter` field in YAML config file |

**Rust plugin changes:**

| File | Change |
|------|--------|
| `src/generator/mod.rs` | Remove path-tolerance hack from parameter parser (parameter will be clean) |
| `integration-tests/generate.sh` | Use `--rust_opt=serde` instead of workarounds |
| `benchmarks/generate.sh` | No change needed (no options) |

## Verification

- `--rust_opt=serde` passes `"serde"` as the `parameter` field (not the output dir)
- `--rust_out=/tmp/out` without `--rust_opt` passes `"/tmp/out"` as parameter (backward compat)
- Config file: `plugins.rust.parameter: "serde"` works
- Rust plugin correctly parses `serde` from parameter field
- `generate.sh` scripts work with the new syntax

## Related

- FD-003 (SerdeMode) — the feature that needs this mechanism to work
- plugin.bop protocol spec — documents the intended `--${NAME}_opt=PARAM` syntax
