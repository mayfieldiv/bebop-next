# bebopc-gen-rust

Rust code generator plugin for the Bebop serialization framework. See `research.md` for detailed architecture docs.

## Build & Test

Always run `./test.sh` to validate changes before considering work done. It is the single source of truth for whether the code is correct — covers fmt, compiler checks, clippy, unit tests (generator + runtime), integration tests (std + no_std), benchmark crate tests, and golden file cross-language verification.

```bash
cd plugins/rust
./test.sh          # required: full validation
cargo test         # quick: unit + integration tests only
cargo bench        # criterion benchmarks (separate from test.sh)
```

## Commit Style

Conventional commits: `feat:`, `fix:`, `chore:`, `test:`, `docs:`

## Repository

This is a fork (`mayfieldiv/bebop-next`). Never create PRs or issues against the upstream repository. All PRs and issues must target `mayfieldiv/bebop-next` only.

---

## Features

Tracked as [GitHub Issues](https://github.com/mayfieldiv/bebop-next/issues).
