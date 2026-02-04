# Bebop 2026

Rewrite of the Bebop compiler in C. New schema language and wire format for the 2026 edition.

## Building

Requires CMake 3.25+ and a C11/C++20 compiler.

```sh
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
```

The compiler will be at `build/bebopc/bebopc`.

## What is Bebop?

Bebop is a schema language for binary serialization. Define data structures in `.bop` files, generate code for your target language.

```bop
edition = "2026"

package Example;

struct Point {
    x: float32;
    y: float32;
}

message Player {
    name(1): string;
    position(2): Point;
    score(3): uint32;
}
```

## What Works So Far?

| Feature | Status | Notes |
|---------|--------|-------|
| Schema parsing | done | Full 2026 edition grammar with error recovery |
| Semantic analysis | done | Type resolution, import handling, cycle detection |
| Validation | done | Compatibility checks, decorator validation via Lua |
| Descriptors | done | Reflection-ready binary format |
| Wire format | done | Encode/decode for all types |
| CLI (`bebopc`) | done | Config files, globbing, diagnostics. macOS only so far. |
| Watch mode | in progress | macOS FSEvents only. No incremental recheck. |
| Plugin system | done | IPC via stdin/stdout, Bebop wire protocol |
| C code generator | in progress | Basic types working, some features incomplete |
| LSP server | in progress | Hover, go-to-definition, completion, formatting, rename |
| VS Code extension | prototype | Language client exists |
| Linux / Windows | not tested | Builds may work, not verified |
| Other language generators | not started | TypeScript, Go, Rust, C#, etc. |

Definitions:

* **done**: Feature complete. OK to log bugs.
* **in progress**: Partially working. Log crashes, not missing features.
* **prototype**: Proof of concept. Don't log bugs.
* **not started**: Not implemented yet.

## Types

| Type | Size | Notes |
|------|------|-------|
| `bool` | 1 | |
| `byte` | 1 | Unsigned. Alias: `uint8` |
| `int8` | 1 | |
| `int16` / `uint16` | 2 | |
| `int32` / `uint32` | 4 | |
| `int64` / `uint64` | 8 | |
| `int128` / `uint128` | 16 | |
| `float16` | 2 | IEEE 754 half. Alias: `half` |
| `float32` | 4 | |
| `float64` | 8 | |
| `bfloat16` | 2 | Alias: `bf16` |
| `string` | variable | Length-prefixed UTF-8 |
| `uuid` | 16 | Alias: `guid` |
| `timestamp` | 12 | Seconds + nanoseconds since epoch |
| `duration` | 12 | Seconds + nanoseconds |

Collections: `T[]`, `T[N]`, `map[K, V]`

Definitions: `struct`, `message`, `union`, `enum`, `const`, `service`

## Configuration

Create `bebop.yml` or `bebop.json` in your project root:

```yaml
sources:
  - "schemas/**/*.bop"
exclude:
  - "**/vendor/**"
include:
  - "vendor/bebop-std"
plugins:
  c: ./generated
options:
  namespace: MyApp
```

Then run:

```sh
bebopc
```

## Documentation

- [Grammar Reference](docs/GRAMMAR.md)
- [Wire Format](docs/WIRE.md)
- [Descriptor Format](docs/DESCRIPTOR.md)
- [Writing Plugins](docs/PLUGINS.md)
- [Whitepaper](docs/bebop-whitepaper.pdf)

## License

See [LICENSE](LICENSE).
