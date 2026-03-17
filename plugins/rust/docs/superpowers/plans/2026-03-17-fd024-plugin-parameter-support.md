# FD-024: Plugin Parameter Support Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire up `--<name>_opt=PARAM` CLI syntax in bebopc so plugins receive language-specific parameters via the `parameter` field.

**Architecture:** Mirrors the existing `--<name>_out=DIR` pattern in the CLI parser. Adds `_try_parse_plugin_opt()` alongside `_try_parse_plugin_out()`, stores the value on the plugin struct, and the runner conditionally sets the parameter field. When no `--<name>_opt` is given, `parameter` is unset (not `out_dir`).

**Tech Stack:** C (bebopc CLI), Bash (generate.sh), Rust (verification only)

**Spec:** `docs/superpowers/specs/2026-03-17-fd024-plugin-parameter-support.md`

**Style mandate:** The bebopc C codebase has extremely consistent style. Every new line of C must be indistinguishable from the existing code. Before writing any code, read the existing function you are mirroring (`_set_plugin_out`, `_try_parse_plugin_out`, the dispatch block, `free_plugin`, the merge function) and match it exactly: brace placement (opening brace on new line for function definitions, same line for `if`/`for`), indentation (2-space indent, 4-space continuation), blank line placement between logical blocks, parameter naming conventions (third param of setter matches field name — `dir` for `out_dir`, `parameter` for `parameter`), `free()` before `bebopc_strdup()` in setters, `free()` ordering matching struct field order in cleanup functions. If in doubt, copy the existing function and change only the names.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `bebopc/src/cli/bebopc_cli.h:29-33` | Modify | Add `char* parameter` to `cli_plugin_t` |
| `bebopc/src/cli/bebopc_config.h:20-26` | Modify | Add `char* parameter` to `bebopc_plugin_t` |
| `bebopc/src/cli/bebopc_cli.c:94-104` | Modify | Add `_set_plugin_opt()` helper |
| `bebopc/src/cli/bebopc_cli.c:138-159` | Modify | Add `_try_parse_plugin_opt()` parser |
| `bebopc/src/cli/bebopc_cli.c:340` | Modify | Dispatch to `_try_parse_plugin_opt()` |
| `bebopc/src/cli/bebopc_cli.c:568-572` | Modify | Free `parameter` in cleanup |
| `bebopc/src/cli/bebopc_config.c:36-42` | Modify | Free `parameter` in `free_plugin` |
| `bebopc/src/cli/bebopc_config.c:563-577` | Modify | Copy `parameter` in merge |
| `bebopc/src/cli/bebopc_runner.c:299` | Modify | Conditional parameter setting |
| `plugins/rust/integration-tests/generate.sh:25-30` | Modify | Add `--rust_opt=serde` |

---

## Chunk 1: bebopc C changes

### Task 1: Add `parameter` field to both plugin structs

**Files:**
- Modify: `bebopc/src/cli/bebopc_cli.h:29-33`
- Modify: `bebopc/src/cli/bebopc_config.h:20-26`

- [ ] **Step 1: Add `parameter` to `cli_plugin_t`**

In `bebopc/src/cli/bebopc_cli.h`, add `char* parameter;` after `char* path;` in the `cli_plugin_t` struct (line 32):

```c
typedef struct {
  char* name;
  char* out_dir;
  char* path;
  char* parameter;
} cli_plugin_t;
```

- [ ] **Step 2: Add `parameter` to `bebopc_plugin_t`**

In `bebopc/src/cli/bebopc_config.h`, add `char* parameter;` after `char* path;` in the `bebopc_plugin_t` struct (line 23):

```c
typedef struct {
  char* name;
  char* out_dir;
  char* path;
  char* parameter;
  bebopc_kv_t* options;
  uint32_t option_count;
} bebopc_plugin_t;
```

- [ ] **Step 3: Verify it compiles**

Run from repo root:
```bash
cd /Users/mayfield/vendor/bebop-next && make debug
```
Expected: Compiles cleanly (no uses of the new field yet).

- [ ] **Step 4: Commit**

```bash
git add bebopc/src/cli/bebopc_cli.h bebopc/src/cli/bebopc_config.h
git commit -m "FD-024: add parameter field to plugin structs"
```

---

### Task 2: Add CLI parsing for `--<name>_opt=VALUE`

**Files:**
- Modify: `bebopc/src/cli/bebopc_cli.c:94-104,138-159,340-348,568-572`

- [ ] **Step 1: Add `_set_plugin_opt()` helper**

In `bebopc/src/cli/bebopc_cli.c`, add `_set_plugin_opt()` immediately after `_set_plugin_out()` (after line 104). This mirrors `_set_plugin_out` but sets `parameter` instead of `out_dir`:

```c
static bool _set_plugin_opt(cli_args_t* args, const char* name, const char* parameter)
{
  cli_plugin_t* p = _find_or_add_plugin(args, name);
  if (!p) {
    return false;
  }

  free(p->parameter);
  p->parameter = bebopc_strdup(parameter);
  return p->parameter != NULL;
}
```

- [ ] **Step 2: Add `_try_parse_plugin_opt()`**

Add immediately after the new `_set_plugin_opt()`. This mirrors `_try_parse_plugin_out()` (lines 138-159) but matches the `_opt` suffix instead of `_out`:

```c
static bool _try_parse_plugin_opt(const char* opt_name, const char* value, cli_args_t* args)
{
  size_t len = bebopc_strlen(opt_name);
  if (len < 5) {
    return false;
  }

  if (opt_name[len - 4] != '_' || opt_name[len - 3] != 'o' || opt_name[len - 2] != 'p'
      || opt_name[len - 1] != 't')
  {
    return false;
  }

  char* plugin_name = bebopc_strndup(opt_name, len - 4);
  if (!plugin_name) {
    return false;
  }

  bool ok = _set_plugin_opt(args, plugin_name, value);
  free(plugin_name);
  return ok;
}
```

- [ ] **Step 3: Add dispatch call**

In the long-option parsing block, add a `_try_parse_plugin_opt` call immediately after the `_try_parse_plugin_out` block (after line 348). The new block follows the exact same pattern:

```c
      if (_try_parse_plugin_opt(opt_name, value, args)) {
        if (!value) {
          snprintf(error_buf, sizeof(error_buf), "option --%s requires a value", opt_name);
          *error_msg = error_buf;
          return BEBOPC_ERR_INVALID_ARG;
        }
        i++;
        continue;
      }
```

This goes between the existing `_try_parse_plugin_out` block (lines 340-348) and the `cli_find_option_long` call (line 350). No blank line between the two `_try_parse` blocks; preserve the existing blank line after this block before `cli_find_option_long`.

- [ ] **Step 4: Update `cli_args_cleanup` to free `parameter`**

In `cli_args_cleanup()` (line 561), add `free(args->plugins[i].parameter);` after the existing `free(args->plugins[i].path);` on line 571:

```c
  for (uint32_t i = 0; i < args->plugin_count; i++) {
    free(args->plugins[i].name);
    free(args->plugins[i].out_dir);
    free(args->plugins[i].path);
    free(args->plugins[i].parameter);
  }
```

- [ ] **Step 5: Verify it compiles**

```bash
cd /Users/mayfield/vendor/bebop-next && make debug
```
Expected: Compiles cleanly.

- [ ] **Step 6: Commit**

```bash
git add bebopc/src/cli/bebopc_cli.c
git commit -m "FD-024: add _try_parse_plugin_opt for --<name>_opt=VALUE"
```

---

### Task 3: Wire parameter through config merge and runner

**Files:**
- Modify: `bebopc/src/cli/bebopc_config.c:36-42,563-577`
- Modify: `bebopc/src/cli/bebopc_runner.c:299`

- [ ] **Step 1: Update `free_plugin` in `bebopc_config.c`**

Add `free(p->parameter);` after `free(p->path);` on line 40:

```c
static void free_plugin(bebopc_plugin_t* p)
{
  free(p->name);
  free(p->out_dir);
  free(p->path);
  free(p->parameter);
  free_options(p->options, p->option_count);
}
```

- [ ] **Step 2: Copy `parameter` in config merge**

In `bebopc_config_merge_cli()`, after the `path` copy on line 575-576, add a `parameter` copy:

```c
    if (p->path) {
      cfg->plugins[cfg->plugin_count - 1].path = bebopc_strdup(p->path);
    }
    if (p->parameter) {
      cfg->plugins[cfg->plugin_count - 1].parameter = bebopc_strdup(p->parameter);
    }
```

- [ ] **Step 3: Update runner to conditionally set parameter**

In `bebopc/src/cli/bebopc_runner.c`, replace line 299:

```c
  bebop_plugin_request_builder_set_parameter(builder, gen->out_dir);
```

with:

```c
  if (gen->parameter) {
    bebop_plugin_request_builder_set_parameter(builder, gen->parameter);
  }
```

- [ ] **Step 4: Verify it compiles**

```bash
cd /Users/mayfield/vendor/bebop-next && make debug
```
Expected: Compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add bebopc/src/cli/bebopc_config.c bebopc/src/cli/bebopc_runner.c
git commit -m "FD-024: wire parameter through config merge and runner"
```

---

### Task 4: Build, install, and smoke test

- [ ] **Step 1: Build release and install to `bin/`**

```bash
cd /Users/mayfield/vendor/bebop-next && make release && cp build/bin/bebopc bin/bebopc
```

- [ ] **Step 2: Smoke test — `--rust_opt` passes parameter**

Run bebopc with `--rust_opt=serde` and check that the Rust plugin receives it. Build the Rust plugin first, then invoke bebopc with a test schema:

```bash
cd /Users/mayfield/vendor/bebop-next/plugins/rust && cargo build -q
PLUGIN="$(pwd)/target/debug/bebopc-gen-rust"
TMPDIR=$(mktemp -d)
../../bin/bebopc build \
  integration-tests/schemas/test_types.bop \
  -I ../../bebop/schemas \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  --rust_opt=serde \
  -q 2>&1
echo "Exit code: $?"
# Check output has serde derives
grep -c 'serde::Serialize' "$TMPDIR/out/test_types.rs" || echo "NO SERDE DERIVES FOUND"
rm -rf "$TMPDIR"
```

Expected: Exit code 0, serde derive count > 0.

- [ ] **Step 3: Smoke test — no `--rust_opt` means parameter is unset**

```bash
cd /Users/mayfield/vendor/bebop-next/plugins/rust
PLUGIN="$(pwd)/target/debug/bebopc-gen-rust"
TMPDIR=$(mktemp -d)
../../bin/bebopc build \
  integration-tests/schemas/test_types.bop \
  -I ../../bebop/schemas \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  -q 2>&1
echo "Exit code: $?"
# Should NOT have serde derives
grep -c 'serde::Serialize' "$TMPDIR/out/test_types.rs" && echo "UNEXPECTED SERDE" || echo "OK: no serde"
rm -rf "$TMPDIR"
```

Expected: Exit code 0, "OK: no serde".

- [ ] **Step 4: Commit updated binary**

```bash
git add bin/bebopc
git commit -m "FD-024: rebuild bebopc with --<name>_opt support"
```

---

## Chunk 2: Rust plugin integration

### Task 5: Update `generate.sh` and regenerate `test_types.rs`

**Files:**
- Modify: `plugins/rust/integration-tests/generate.sh:25-30`
- Modify: `plugins/rust/integration-tests/src/test_types.rs` (regenerated)

- [ ] **Step 1: Add `--rust_opt=serde` to `generate.sh`**

In `plugins/rust/integration-tests/generate.sh`, add `--rust_opt=serde \` after the `--rust_out` line (line 29):

```bash
echo "Generating test_types.rs..."
"$BEBOPC" build \
  "$INT_DIR/schemas/test_types.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  --rust_opt=serde \
  -q 2>/dev/null
```

- [ ] **Step 2: Run `generate.sh` to regenerate `test_types.rs`**

```bash
cd /Users/mayfield/vendor/bebop-next/plugins/rust
./integration-tests/generate.sh
```

Expected: "Done." with no errors.

- [ ] **Step 3: Verify serde derives are present**

```bash
grep -c 'serde::Serialize' integration-tests/src/test_types.rs
```

Expected: Count > 0 (confirms serde derives were generated).

- [ ] **Step 4: Run full test suite**

```bash
cd /Users/mayfield/vendor/bebop-next/plugins/rust && ./test.sh
```

Expected: All checks pass (fmt, check, test, clippy).

- [ ] **Step 5: Commit**

```bash
git add plugins/rust/integration-tests/generate.sh plugins/rust/integration-tests/src/test_types.rs
git commit -m "FD-024: pass --rust_opt=serde in generate.sh, regenerate test_types.rs"
```
