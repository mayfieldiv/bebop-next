# Benchmark C++ Style Consistency Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make all C++ code changed on this branch perfectly consistent with the pre-branch style in bench_bebop.cpp.

**Architecture:** Rename-only changes in encode_helpers.cpp. No behavioral changes.

**Tech Stack:** C++17, CMake

---

## Review Findings

Compared the pre-branch `bench_bebop.cpp` (at `d24b7634`) against all C/C++ files changed on this branch. Findings organized by severity:

### Must Fix

| # | File | Issue | Fix |
|---|------|-------|-----|
| 1 | `encode_helpers.cpp:21` | `h_ctx` — uses `h_` prefix instead of `g_` | Rename to `g_ctx` (static linkage, no conflict with bench_bebop's `g_ctx`) |
| 2 | `encode_helpers.cpp:22` | `h_writer` — same prefix issue | Rename to `g_writer` |
| 3 | `encode_helpers.cpp:23` | `H_WRITER_SIZE` — prefixed constant | Rename to `WRITER_SIZE` (matches bench_bebop.cpp:27) |
| 4 | `encode_helpers.cpp:25` | `ensure_helper_ctx()` — qualifier not needed | Rename to `ensure_ctx()` (both static, no conflict) |

### Acceptable As-Is (no changes needed)

| # | File | Observation | Rationale |
|---|------|-------------|-----------|
| 5 | `harness.cpp:194` | `{ }` block scope around nested JSON | Matches pre-branch pattern: alice chunks section (line 219) uses the same block scope for complex fixture construction with many temporaries. |
| 6 | `harness.cpp:197,211,236` | Section comments in fixture data (`// 10x10 number matrix`, etc.) | Navigational value for a 70-line nested-loop block. Comparable to section separators in encode_helpers.cpp (#8). |
| 7 | `bench_bebop.cpp:554` | Comment before nested JSON storage section | Follows same pattern as pre-branch line 44 (`// Create a reader once - we'll reuse it with Bebop_Reader_Reset`). Explains a non-obvious architectural decision. |
| 8 | `encode_helpers.h` in `src/` | Different location from `bench_harness.h` in `include/` | Correct: `include/` is for shared headers (all benchmark libs), `src/` is for Bebop-private headers. CMakeLists.txt reflects this (`PUBLIC include` vs `PRIVATE src`). |
| 9 | `encode_helpers.cpp:46,119,...` | Section separator comments `// ---- Title ----` | Helpful for navigating a 550-line file organized by schema type. The original had no sections because it was monolithic. |
| 10 | `encode_helpers.cpp:38-44` | `flush_writer()` helper | DRY improvement — replaces 4 identical lines repeated 15 times in the original. |
| 11 | `encode_helpers.cpp:8` + `bench_bebop.cpp:10` | `BEBOP_CHECK` macro duplicated | Each TU defines its own `(void)(expr)`. Not worth sharing for a one-liner. |
| 12 | `encode_helpers.cpp:10-18` + `bench_bebop.cpp:12-21` | `libc_alloc` duplicated | 9-line callback function. Sharing via header requires `static inline` or an extra TU — not worth the complexity. |

### Removed from "Must Fix" after review

| # | Original item | Reason removed |
|---|---------------|----------------|
| — | `harness.cpp:194` block scope | Pre-branch alice chunks (line 219) uses identical pattern. Keeping is consistent. |
| — | `harness.cpp` inline comments | Section markers provide navigational value for a dense 70-line block. |

---

## Task 1: Rename encode_helpers.cpp statics to match g_ convention

**Files:**
- Modify: `lab/benchmark/c/src/encode_helpers.cpp`

- [ ] **Step 1: Rename all four symbols using replace-all**

These are all file-scoped `static` symbols. bench_bebop.cpp has identically-named statics — that's fine, `static` gives each TU its own copy.

- `h_ctx` → `g_ctx`
- `h_writer` → `g_writer`
- `H_WRITER_SIZE` → `WRITER_SIZE`
- `ensure_helper_ctx` → `ensure_ctx`

- [ ] **Step 2: Build to verify no conflicts**

Run: `cd lab/benchmark/c && cmake --build build --target run_benchmarks --target dump_golden`
Expected: clean build, no errors

- [ ] **Step 3: Verify golden output unchanged**

Run: `mkdir -p /tmp/claude/golden-rename && ./build/dump_golden /tmp/claude/golden-rename && cmp /tmp/claude/golden-fixed/JsonNested.bin /tmp/claude/golden-rename/JsonNested.bin`
Expected: identical bytes

## Task 2: Commit

- [ ] **Step 1: Stage and commit**

```bash
git add lab/benchmark/c/src/encode_helpers.cpp
git commit -m "chore: align encode_helpers.cpp naming with g_ convention"
```

- [ ] **Step 2: Verify golden output unchanged**

Rebuild dump_golden target, dump to a temp dir, cmp against known-good golden.
