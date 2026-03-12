# FD-003: SerdeMode (Generator-side --serde flag)

**Status:** Complete
**Priority:** High
**Effort:** Medium (1-4 hours)
**Impact:** Enables always-on serde derives without requiring Cargo feature flags

## Problem

Serde derives were only available behind `#[cfg(feature = "serde")]`, requiring consumers to use Cargo feature flags. Some users want serde always enabled or completely disabled at generation time.

## Solution

Added `SerdeMode` enum (`Always`, `FeatureGated`, `Disabled`) parsed from the `--serde` generator parameter. Threaded through all generation modules to control serde attribute emission.

## Verification

- E2E binary tests for all three serde modes
- Unit tests for SerdeMode parsing
- Regenerated test fixtures with Always mode

## Related

- Commits: d3ed1fd through 325d82f
