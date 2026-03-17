# FD-033: Compiler Decorator FQN Not Fully Qualified

**Status:** Planned
**Priority:** Medium
**Effort:** Low (< 1 hour)
**Impact:** Fixes a contract violation in the plugin protocol that affects all language plugins

## Problem

`DecoratorUsage.fqn` in the plugin protocol is documented as "Always fully qualified after linking" (`bebop/schemas/bebop/decorators.bop`), but the compiler populates it with the as-written name from source instead of the resolved FQN.

In `bebop/src/bebop_descriptor.c:188`:

```c
DESC_SET_OPT_STR(u->fqn, BEBOP_STR(bctx, d->name));
```

`d->name` is the bare name as written by the user (e.g. `"deprecated"` when the schema says `@deprecated`). After validation, the resolved definition is available at `d->resolved`, which has the fully-qualified name (e.g. `"bebop.deprecated"`), but the descriptor builder doesn't use it.

### Consequences

- All plugins must defensively match both bare and FQN forms for every decorator check
- The Swift plugin checks `== "deprecated"` (bare name — works but fragile)
- The C plugin checks `strcmp(fqn, "deprecated") == 0` (same)
- The Rust plugin was checking `== "bebop.deprecated"` (correct per spec, but broken in practice — fixed in FD-031 with suffix matching)
- User-defined decorators in custom packages (e.g. `@mypackage.validate`) would have the same issue if written with bare names after an import

## Solution

In `bebop_descriptor.c`, use the resolved FQN when available:

```c
// Current (line 188):
DESC_SET_OPT_STR(u->fqn, BEBOP_STR(bctx, d->name));

// Fix:
const char* fqn_str = (d->resolved && !bebop_str_is_null(d->resolved->fqn))
    ? BEBOP_STR(bctx, d->resolved->fqn)
    : BEBOP_STR(bctx, d->name);
DESC_SET_OPT_STR(u->fqn, fqn_str);
```

Falls back to bare name if resolution failed (shouldn't happen after validation, but defensive).

## Files to Modify

| File | Action | Purpose |
|------|--------|---------|
| `bebop/src/bebop_descriptor.c:188` | MODIFY | Use `d->resolved->fqn` instead of `d->name` |

## Verification

- Build bebopc with the fix
- Regenerate Rust test_types.rs — `emit_deprecated` should match on `"bebop.deprecated"` directly (the current suffix-matching fallback would still work, confirming backward compat)
- Run plugin test suites for Rust, Swift, C to ensure no regressions (they all tolerate bare names, so FQN should also work)
- Verify a user-defined decorator in a custom package (e.g. `@validators.range`) also gets its FQN populated correctly

## Related

- FD-031: Deprecated Attribute Emission (worked around this bug with suffix matching in the Rust plugin)
- `bebop/src/bebop_validate.c:1179` — where `dec->resolved = dec_def` is set during validation
- `bebop/schemas/bebop/decorators.bop` — `DecoratorUsage.fqn` documented as "Always fully qualified after linking"
