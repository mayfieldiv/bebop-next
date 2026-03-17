# FD-024: Plugin Parameter Support — Design Spec

**Goal:** Wire up the documented but unimplemented `--<name>_opt=PARAM` CLI syntax so plugins can receive language-specific parameters via the `parameter` field in `CodeGeneratorRequest`.

**Context:** The plugin protocol (`plugin.bop`) documents `--${NAME}_opt=PARAM` syntax for the `parameter` field, but bebopc never implemented it. Currently `parameter` is always set to `gen->out_dir`. The Rust plugin's serde parameter parser is dead code because of this.

---

## Scope

**In scope:**
- CLI parsing of `--<name>_opt=VALUE`
- Wiring the parsed value to the `parameter` field in the plugin request
- Updating `generate.sh` to use `--rust_opt=serde`

**Out of scope:**
- YAML config `parameter` field (users can use existing `options` map for YAML workflows)
- Changes to C or Swift plugins (neither reads `parameter`)
- Changes to the Rust plugin's parameter parser (already correct)

## Architecture

Mirrors the existing `--<name>_out=DIR` pattern. A new `_try_parse_plugin_opt()` function in the CLI parser extracts the plugin name and value, stores it on the plugin struct, and the runner passes it through to `bebop_plugin_request_builder_set_parameter()`.

When no `--<name>_opt` is provided, `parameter` is not set at all (plugin receives `None`). This is a clean break from the current behavior of always setting `parameter = out_dir`, but is safe because no plugin depends on that value.

## Changes

### 1. `bebopc/src/cli/bebopc_config.h`

Add `char* parameter` field to both plugin structs:

```c
// cli_plugin_t
typedef struct {
  char* name;
  char* out_dir;
  char* path;
  char* parameter;    // NEW: from --<name>_opt=VALUE
} cli_plugin_t;

// bebopc_plugin_t
typedef struct {
  char* name;
  char* out_dir;
  char* path;
  char* parameter;    // NEW: from --<name>_opt=VALUE
  bebopc_kv_t* options;
  uint32_t option_count;
} bebopc_plugin_t;
```

### 2. `bebopc/src/cli/bebopc_cli.c`

Add `_try_parse_plugin_opt()` mirroring `_try_parse_plugin_out()` (lines 138-159):

- Match options ending in `_opt` suffix
- Extract plugin name by stripping `_opt`
- Store value in `cli_plugin_t.parameter` via a `_set_plugin_opt()` helper (mirrors `_set_plugin_out()`)
- Call from the same long-option dispatch point that calls `_try_parse_plugin_out()` (around line 340)
- Uses `=` syntax only (`--rust_opt=serde`), consistent with `--rust_out=DIR`

Also update `cli_args_cleanup` (line 561) to `free(args->plugins[i].parameter)`.

### 3. `bebopc/src/cli/bebopc_config.c`

In the CLI→config merge function (around lines 543-604):

- Copy `cli_plugin.parameter` to `bebopc_plugin_t.parameter` during merge

Also update `free_plugin` (lines 36-42) to `free(p->parameter)`.

**Note:** `--<name>_opt` without a corresponding `--<name>_out` is silently ignored (the merge skips plugins without `out_dir`). This matches the existing behavior for plugin entries without an output directory.

### 4. `bebopc/src/cli/bebopc_runner.c`

Replace line 299:

```c
// Before:
bebop_plugin_request_builder_set_parameter(builder, gen->out_dir);

// After:
if (gen->parameter)
  bebop_plugin_request_builder_set_parameter(builder, gen->parameter);
```

When no `--<name>_opt` is provided, `parameter` is simply not set.

### 5. `plugins/rust/integration-tests/generate.sh`

Add `--rust_opt=serde` to the bebopc invocation:

```bash
"$BEBOPC" build \
  "$INT_DIR/schemas/test_types.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  --rust_opt=serde \
  -q 2>/dev/null
```

## Verification

- `--rust_opt=serde` passes `"serde"` as the `parameter` field
- `--rust_out=/tmp/out` without `--rust_opt` results in `parameter = None` (not the output dir)
- `generate.sh` produces `test_types.rs` with serde derives
- Existing tests pass (no regressions from `parameter` no longer being `out_dir`)

## Risk

Low. The change is small (< 50 lines of C), mirrors an existing pattern (`_out` → `_opt`), and the only consumer of `parameter` is the Rust plugin which already handles `None`.
