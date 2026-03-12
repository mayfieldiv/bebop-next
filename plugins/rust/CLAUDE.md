# bebopc-gen-rust

Rust code generator plugin for the Bebop serialization framework. See `research.md` for detailed architecture docs.

## Build & Test

```bash
cd plugins/rust
./test.sh          # full validation: fmt, check, test, clippy
cargo test         # unit + integration tests
cargo bench        # criterion benchmarks
```

## Commit Style

Conventional commits: `feat:`, `fix:`, `chore:`, `test:`, `docs:`

---

## Feature Design (FD) Management

Features are tracked in `docs/features/`. Each FD has a dedicated file (`FD-XXX_TITLE.md`) and is indexed in `FEATURE_INDEX.md`.

### FD Lifecycle

| Stage | Description |
|-------|-------------|
| **Planned** | Identified but not yet designed |
| **Design** | Actively designing (exploring code, writing plan) |
| **Open** | Designed and ready for implementation |
| **In Progress** | Currently being implemented |
| **Pending Verification** | Code complete, awaiting verification |
| **Complete** | Verified working, ready to archive |
| **Deferred** | Postponed (low priority or blocked) |
| **Closed** | Won't implement (superseded or not needed) |

### FD Skills

| Skill | Purpose |
|-------|---------|
| `fd-new` | Create a new feature design |
| `fd-explore` | Explore project - overview, FD history, recent activity |
| `fd-deep` | Deep parallel analysis — 4 agents explore a hard problem from different angles, verify claims, synthesize |
| `fd-status` | Show active FDs with status and grooming |
| `fd-verify` | Post-implementation: commit, proofread, verify |
| `fd-close` | Complete/close an FD, archive file, update index |

### Conventions

- **FD files**: `docs/features/FD-XXX_TITLE.md` (XXX = zero-padded number)
- **Commit format**: `FD-XXX: Brief description`
- **Numbering**: Next number = highest across all index sections + 1
- **Source of truth**: FD file status > index (if discrepancy, file wins)
- **Archive**: Completed FDs move to `docs/features/archive/`

### Managing the Index

The `FEATURE_INDEX.md` file has four sections:

1. **Active Features** — All non-complete FDs, sorted by FD number
2. **Completed** — Completed FDs, newest first
3. **Deferred / Closed** — Items that won't be done
4. **Backlog** — Low-priority or blocked items parked for later

### Inline Annotations (`%%`)

Lines starting with `%%` in any file are **inline annotations from the user**. When you encounter them:
- Treat each `%%` annotation as a direct instruction — answer questions, develop further, provide feedback, or make changes as requested
- Address **every** `%%` annotation in the file; do not skip any
- After acting on an annotation, remove the `%%` line from the file
- If an annotation is ambiguous, ask for clarification before acting
