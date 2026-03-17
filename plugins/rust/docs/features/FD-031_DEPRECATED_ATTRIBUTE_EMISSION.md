# FD-031: Deprecated Attribute Emission

**Status:** Complete
**Priority:** Medium
**Effort:** Low (< 1 hour)
**Impact:** Compiler warnings guide users away from deprecated fields

## Problem

The Bebop schema `@deprecated` decorator was not being emitted as `#[deprecated]` on generated message fields, despite the generator having code wired for it.

## Root Cause

Two bugs conspiring:

1. **Compiler sends bare names** — `bebop_descriptor.c` uses `d->name` (the as-written name, e.g. `"deprecated"`) instead of `d->resolved->fqn` (e.g. `"bebop.deprecated"`) when populating `DecoratorUsage.fqn`.
2. **`emit_deprecated` used strict FQN matching** — It checked `== Some("bebop.deprecated")` exactly, which never matched the bare `"deprecated"` the compiler sends.

The existing `has_decorator()` function (used for `@forward_compatible`) already had flexible matching that handled both bare and FQN forms. `emit_deprecated` lacked this.

Swift and C plugins both match on bare `"deprecated"` and work correctly.

## Solution

Extracted a shared `decorator_matches(fqn, expected)` helper that handles both bare and fully-qualified names via suffix matching. Used it in both `emit_deprecated()` and `has_decorator()`.

## Files Modified

| File | Change |
|------|--------|
| `src/generator/mod.rs` | Extracted `decorator_matches()`, added `DEPRECATED` const, updated `emit_deprecated()` and `has_decorator()` |
| `integration-tests/src/test_types.rs` | Regenerated — now includes `#[deprecated]` on fields |
| `integration-tests/tests/integration.rs` | Added `#[allow(deprecated)]` to test that accesses deprecated fields |

## Upstream Note

The compiler bug (`bebop_descriptor.c:188` using `d->name` instead of `d->resolved->fqn`) should also be fixed so that `DecoratorUsage.fqn` honors its documented contract of being "always fully qualified after linking."
