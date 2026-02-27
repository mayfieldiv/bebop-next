# Bebop-Next C Plugin: Comprehensive Research

## Overview

The C plugin (`bebopc-gen-c`) is a code generator that transforms Bebop schema definitions into C source code with full serialization/deserialization support. It is a single-file implementation (`src/generator.c`, ~4660 lines) built as a standalone executable that communicates with the Bebop compiler via stdin/stdout using the Bebop plugin protocol.

The generated C code provides:

- Type-safe struct/enum/union/message/constant definitions
- Binary serialization (encode) and deserialization (decode) functions
- Encoded size calculation functions
- Runtime reflection metadata
- Zero-copy optimizations for primitive arrays
- Cache prefetch hints for high throughput

---

## Build System

**CMakeLists.txt** (5 lines):

```cmake
add_executable(bebopc-gen-c src/generator.c)
target_link_libraries(bebopc-gen-c PRIVATE bebop)
install(TARGETS bebopc-gen-c RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
```

The plugin links against the `bebop` runtime library which provides:

- Schema descriptor APIs (`bebop.h`) for introspecting parsed schemas
- Wire format types and macros (`bebop_wire.h`) used in generated code
- Plugin protocol types for request/response encoding

---

## Plugin Protocol (Entry Point)

### `main()` (line 4616)

1. Sets stdin/stdout to binary mode on Windows (`_setmode`)
2. Creates a `bebop_context_t` with a host allocator and file reader
3. Reads the entire stdin into a buffer (`read_stdin`)
4. Decodes the buffer into a `bebop_plugin_request_t`
5. Calls `generate()` to produce output files
6. Cleans up all resources

The plugin is a pure stdin-to-stdout transformer: it receives a binary-encoded plugin request containing one or more parsed schemas, generates C code, and writes a binary-encoded plugin response containing the output files.

### Host Interface

```c
bebop_host_allocator_t alloc = {.alloc = plugin_alloc};  // realloc-based
bebop_file_reader_t reader = {.read = plugin_file_read,   // stubbed (not needed)
                              .exists = plugin_file_exists}; // stubbed
```

The file reader is stubbed because the plugin only needs the already-parsed schema descriptors.

---

## Configuration Options

Parsed in `opts_parse()` (line 402) from host options in the plugin request:

| Option          | Values                            | Default | Description                                  |
| --------------- | --------------------------------- | ------- | -------------------------------------------- |
| `c_standard`    | `c99`, `c11`, `c23`               | `c11`   | Target C standard                            |
| `output_mode`   | `split`, `unity`, `single_header` | `split` | File output strategy                         |
| `prefix`        | any string                        | `""`    | Prefix prepended to all generated type names |
| `no_reflection` | `true`/`1`                        | `false` | Disable reflection metadata generation       |

### Output Modes

**Split** (`GEN_OUT_SPLIT`, default): Generates separate `.bb.h` (declarations) and `.bb.c` (implementations) files per schema. The header contains type definitions, forward declarations, function prototypes, macros, and reflection externs. The source includes the header and contains function bodies and reflection data.

**Unity** (`GEN_OUT_UNITY`): Generates a single `.bb.c` file per schema containing everything (declarations + implementations). The file still has an include guard and can be included like a header. Import includes use `.bb.c` extension.

**Single Header** (`GEN_OUT_SINGLE_HEADER`): Generates a single `.bb.h` file. Declarations appear normally; implementations are wrapped in `#ifdef <GUARD>_IMPLEMENTATION` so users can control where the implementation lives by defining the macro in exactly one translation unit.

---

## Core Data Structures

### String Builder (`gen_sb_t`, line 29)

A dynamic string buffer used throughout the generator for constructing output:

```c
typedef struct {
  char* data;   // null-terminated buffer
  size_t len;   // current content length
  size_t cap;   // allocated capacity
} gen_sb_t;
```

Growth strategy: starts at 256 bytes, doubles on each resize. Key operations:

- `sb_putc` / `sb_puts` / `sb_printf` - append characters/strings/formatted
- `sb_puts_screaming_raw` - converts `camelCase` to `SCREAMING_SNAKE_CASE`
- `sb_puts_screaming` - like `_raw` but deduplicates consecutive identical words (e.g., `MyEnum_MyEnumValue` becomes `MY_ENUM_VALUE` not `MY_ENUM_MY_ENUM_VALUE`)
- `sb_enum_member` - generates `TYPE_MEMBER` names with deduplication of type/member word overlap
- `sb_escape_string` - escapes `\`, `"`, `\n`, `\r`, `\t` for C string literals
- `sb_steal` - takes ownership of the buffer (returns pointer, resets builder)
- `sb_indent` - emits 4 spaces per indent level

### Type Set (`gen_type_set_t`, line 331)

Hash-based deduplication set for tracking emitted types:

```c
typedef struct {
  const char* fqns[GEN_TYPE_SET_CAP];  // stored names (256 slots)
  uint32_t hashes[GEN_TYPE_SET_CAP];    // FNV1a hashes
  uint16_t count;
} gen_type_set_t;
```

Uses FNV1a hashing (line 337) with hash-only comparison (no collision handling). Three separate sets in the context prevent duplicate generation of array typedefs, map types, and message FQN references.

### Generator Context (`gen_ctx_t`, line 715)

Central state passed through all generation functions:

```c
typedef struct {
  gen_sb_t out;                          // output buffer
  gen_opts_t opts;                       // parsed options
  gen_type_set_t array_types;            // dedup set for array typedefs
  gen_type_set_t map_types;              // dedup set for map types
  gen_type_set_t message_fqns;           // dedup set for message FQNs
  const bebop_plugin_request_t* req;     // plugin request
  const bebop_descriptor_schema_t* schema; // current schema being processed
  int indent;                            // current indentation level
  int loop_depth;                        // for generating unique loop vars (_i0, _i1, ...)
  bool is_mutable;                       // current definition's mutability
  bool in_step_fn;                       // inside a step function (affects error returns)
  gen_emit_mode_t emit_mode;             // DECL or IMPL
  const char* error;                     // sticky error message
} gen_ctx_t;
```

### Work Item (`size_work_t`, line 2816)

Stack frame for iterative type traversal (used in size/encode/decode generation):

```c
typedef struct {
  const bebop_descriptor_type_t* type;   // the type being processed
  char access[GEN_PATH_SIZE];            // C expression to access this value
  int state;                             // 0 = initial, 1 = children done
  int loop_var;                          // loop variable index for this depth
  bool is_ptr;                           // whether access is a pointer
} size_work_t;
```

### Field Sort Entry (`field_sort_entry_t`, line 1873)

Used for struct field memory layout optimization:

```c
typedef struct {
  uint32_t index;  // original field index
  uint32_t align;  // alignment requirement
  uint32_t size;   // C size
} field_sort_entry_t;
```

---

## Type System

### Type Map (`TYPE_MAP[]`, line 473)

Static lookup table mapping Bebop wire types to C types:

| Bebop Type  | C Type            | Wire Get                    | Wire Set                    | Size |
| ----------- | ----------------- | --------------------------- | --------------------------- | ---- |
| `bool`      | `bool`            | `Bebop_Reader_GetBool`      | `Bebop_Writer_SetBool`      | 1    |
| `byte`      | `uint8_t`         | `Bebop_Reader_GetByte`      | `Bebop_Writer_SetByte`      | 1    |
| `int8`      | `int8_t`          | `Bebop_Reader_GetI8`        | `Bebop_Writer_SetI8`        | 1    |
| `int16`     | `int16_t`         | `Bebop_Reader_GetI16`       | `Bebop_Writer_SetI16`       | 2    |
| `uint16`    | `uint16_t`        | `Bebop_Reader_GetU16`       | `Bebop_Writer_SetU16`       | 2    |
| `int32`     | `int32_t`         | `Bebop_Reader_GetI32`       | `Bebop_Writer_SetI32`       | 4    |
| `uint32`    | `uint32_t`        | `Bebop_Reader_GetU32`       | `Bebop_Writer_SetU32`       | 4    |
| `int64`     | `int64_t`         | `Bebop_Reader_GetI64`       | `Bebop_Writer_SetI64`       | 8    |
| `uint64`    | `uint64_t`        | `Bebop_Reader_GetU64`       | `Bebop_Writer_SetU64`       | 8    |
| `int128`    | `Bebop_Int128`    | `Bebop_Reader_GetI128`      | `Bebop_Writer_SetI128`      | 16   |
| `uint128`   | `Bebop_UInt128`   | `Bebop_Reader_GetU128`      | `Bebop_Writer_SetU128`      | 16   |
| `float16`   | `Bebop_Float16`   | `Bebop_Reader_GetF16`       | `Bebop_Writer_SetF16`       | 2    |
| `float32`   | `float`           | `Bebop_Reader_GetF32`       | `Bebop_Writer_SetF32`       | 4    |
| `float64`   | `double`          | `Bebop_Reader_GetF64`       | `Bebop_Writer_SetF64`       | 8    |
| `bfloat16`  | `Bebop_BFloat16`  | `Bebop_Reader_GetBF16`      | `Bebop_Writer_SetBF16`      | 2    |
| `string`    | `Bebop_Str`       | `Bebop_Reader_GetStr`       | `Bebop_Writer_SetStrView`   | 0\*  |
| `uuid`      | `Bebop_UUID`      | `Bebop_Reader_GetUUID`      | `Bebop_Writer_SetUUID`      | 16   |
| `timestamp` | `Bebop_Timestamp` | `Bebop_Reader_GetTimestamp` | `Bebop_Writer_SetTimestamp` | 12   |
| `duration`  | `Bebop_Duration`  | `Bebop_Reader_GetDuration`  | `Bebop_Writer_SetDuration`  | 12   |

\*String size is 0 (variable-length), requiring special handling everywhere.

### Container Type Mapping

- **Array** (`BEBOP_TYPE_ARRAY`): Mapped to `TypeName_Array` structs with `{data, length, capacity}` fields. Primitive arrays use built-in types like `Bebop_I32_Array`. Complex element types (fixed arrays, maps, user-defined) get custom typedefs.
- **Fixed Array** (`BEBOP_TYPE_FIXED_ARRAY`): Represented as C arrays `type field[N]`. Multi-dimensional: `type field[N][M]`.
- **Map** (`BEBOP_TYPE_MAP`): Always `Bebop_Map` (opaque hash map from the runtime). Keys and values are void pointers internally.
- **Defined** (`BEBOP_TYPE_DEFINED`): User-defined type referenced by FQN, mapped to prefixed C name.

### Type Name Generation

`type_name()` (line 835) converts a Bebop FQN like `myPackage.MyType` to a C identifier like `PrefixMyPackage_MyType`. Uses a rotating buffer of 4 static `char[512]` arrays to allow up to 4 concurrent name references without overwrites.

`name_from_fqn()` (line 807) is the string-builder variant: capitalizes first letter and letters after dots/underscores, replaces dots with underscores.

### Type Queries

- `is_primitive(kind)` (line 505): true for fixed-size types (size > 0 in TYPE_MAP)
- `type_is_enum(ctx, type)` (line 956): resolves DEFINED types to check if they are enums
- `type_is_message_or_union(ctx, type)` (line 966): checks if a DEFINED type is message/union
- `enum_base_type(ctx, type)` (line 981): gets the underlying integer type of an enum
- `type_alignment(type)` (line 533): returns alignment requirement (1, 2, 4, or 8 bytes)
- `type_c_size(type)` (line 567): returns C `sizeof` equivalent for layout calculations

---

## Code Generation Pipeline

The `generate()` function (line 4416) drives the entire pipeline. For each schema in the request:

### Phase 1: Header Generation (declarations)

Per output mode, the header/declaration phase generates in this order:

1. **File header** (`emit_file_header`, line 1521):
   - Generated-code notice with compiler version, source path, edition
   - Include guard (`#ifndef` / `#define`)
   - `#include <bebop_wire.h>`
   - Import includes (`.bb.h` or `.bb.c` depending on mode)
   - Insertion point: `@@bebop_insertion_point(includes)`
   - `extern "C"` guard for C++ compatibility
   - Insertion point: `@@bebop_insertion_point(forward_declarations)`

2. **Forward declarations** (`gen_forward_decls`, line 1615):
   - `typedef struct TypeName TypeName;` for all structs, messages, and unions
   - Depth-first traversal handles nested definitions

3. **Enums** (`gen_enums_only` -> `gen_enum`, line 1762):
   - `typedef enum { MEMBER = value, ... } TypeName;`
   - Smart member naming via `sb_enum_member` that deduplicates overlapping words
   - Supports signed/unsigned base types and flags (hex values)

4. **Container typedefs** (`gen_container_typedef` -> `emit_container_types`, line 1313):
   - Generates array typedef structs for non-primitive element types
   - Uses depth-first traversal to handle nested containers (array of array of maps, etc.)
   - Deduplication via `type_set_add` prevents duplicate typedefs
   - Guard macros (`#ifndef`/`#define`) provide additional cross-file dedup

5. **Type definitions** (`gen_typedef`, line 2029):
   - Constants (`gen_const`): `extern const Type Name;` declarations
   - Structs (`gen_struct`): field layout with alignment optimization
   - Messages (`gen_message`): tag enum + struct with optional fields
   - Unions (`gen_union`): discriminator enum + tagged union struct

6. **Function declarations** (`gen_functions` in `GEN_EMIT_DECL` mode):
   - Size macros: `#define TYPE_MIN_SIZE` and `#define TYPE_FIXED_SIZE`
   - `size_t Type_EncodedSize(const Type *v);`
   - `Bebop_WireResult Type_Encode(Bebop_Writer *w, const Type *v);`
   - `Bebop_WireResult Type_Decode(Bebop_WireCtx *ctx, Bebop_Reader *rd, Type *v);`

7. **Reflection declarations** (`gen_reflection_decl`, line 2722):
   - `extern const BebopReflection_DefinitionDescriptor Type__refl_descriptor;`
   - `extern const Bebop_TypeInfo Type__type_info;`

8. **File footer** (`emit_file_footer`, line 1562):
   - Insertion points: `@@bebop_insertion_point(declarations)`, `(definitions)`, `(eof)`
   - Close `extern "C"` and include guard

### Phase 2: Implementation Generation

The implementation phase generates function bodies and reflection data:

1. **Constant definitions** - actual values for `extern const` declarations
2. **EncodedSize functions** - calculate wire size of a value
3. **Encode functions** - serialize to a writer
4. **Decode functions** - deserialize from a reader
5. **TypeInfo structs** - function pointers for runtime dispatch
6. **Reflection data** - full schema metadata

---

## Struct Generation

### `gen_struct()` (line 1888)

**Field Layout Optimization**: Fields are sorted by alignment (descending) then size (descending) to minimize padding:

```c
field_sort_entry_t* sorted = malloc(field_count * sizeof(field_sort_entry_t));
for (uint32_t i = 0; i < field_count; i++) {
    sorted[i].index = i;
    sorted[i].align = type_alignment(ftype);
    sorted[i].size = type_c_size(ftype);
}
qsort(sorted, field_count, sizeof(field_sort_entry_t), field_sort_cmp);
```

This ensures 8-byte aligned fields come first, then 4-byte, 2-byte, and 1-byte, minimizing internal padding.

**Mutability**: If the definition is not marked mutable, field types get a `const` qualifier. This enables the compiler to enforce immutability and allows the generator to use zero-copy optimizations.

**Empty structs**: Use `BEBOP_WIRE_EMPTY_STRUCT;` macro (likely expands to a dummy byte for C compliance).

**Insertion point**: `@@bebop_insertion_point(struct_scope:TypeName)` inside the struct body allows users to add custom fields.

### Field Declaration (`emit_field_decl`, line 1811)

Handles several cases:

- **Regular struct fields**: `const Type fieldName;` (or without const if mutable)
- **Fixed arrays in structs**: `const Type fieldName[N][M];` with dimensions appended
- **Message fields**: `BEBOP_WIRE_OPT(Type) fieldName;` (optional wrapper)
- **Message fields with fixed arrays**: `BEBOP_WIRE_OPT_FA(Type, [N], [M]) fieldName;`
- **Message fields with message/union type**: `BEBOP_WIRE_OPT(Type *) fieldName;` (pointer for indirection)
- **Deprecated fields**: Annotated with `BEBOP_WIRE_DEPRECATED` or `BEBOP_WIRE_DEPRECATED_MSG("reason")`
- **Keyword-conflicting names**: Appended with `_` suffix (60+ C keywords checked)

---

## Message Generation

### `gen_message()` (line 1923)

Messages generate two things:

1. **Tag enum**: `typedef enum { TYPE_FIELD_TAG = index, ... } Type_Tag;`
   - Each field gets a unique tag based on its schema-defined index
   - Used during encoding to identify which optional fields are present

2. **Struct**: All fields wrapped in `BEBOP_WIRE_OPT(Type)` macro
   - The OPT wrapper provides `has_value` and `value` members
   - `BEBOP_WIRE_IS_SOME(field)` / `BEBOP_WIRE_UNWRAP(field)` macros for access
   - `BEBOP_WIRE_SET_NONE(field)` to initialize as absent
   - Message/union fields stored as pointers to allow indirection

---

## Union Generation

### `gen_union()` (line 1965)

Unions generate:

1. **Discriminator enum**: `typedef enum { TYPE_NONE = 0, TYPE_BRANCH = disc, ... } Type_Disc;`
   - Discriminator 0 is always reserved as "None" (null/empty)
   - Each branch gets its schema-defined discriminator value

2. **Tagged union struct**:
   ```c
   struct Type {
       Type_Disc discriminator;
       union {
           BranchType branchname;
           ...
       };
   };
   ```

   - Anonymous union (C11 feature) for clean field access
   - Branch names converted to lowercase; keyword conflicts get `_` suffix

---

## Constant Generation

### `gen_const()` (line 1652)

Supports all literal types:

| Literal Kind | Declaration                          | Definition                           |
| ------------ | ------------------------------------ | ------------------------------------ |
| Integer      | `extern const int64_t Name;`         | `const int64_t Name = 42LL;`         |
| Float        | `extern const double Name;`          | `const double Name = 3.14;`          |
| Bool         | `extern const bool Name;`            | `const bool Name = true;`            |
| String       | `extern const char Name[];`          | `const char Name[] = "hello";`       |
| UUID         | `extern const Bebop_UUID Name;`      | `const Bebop_UUID Name = {{0x...}};` |
| Bytes        | `extern const Bebop_Bytes Name;`     | Static data array + pointer struct   |
| Timestamp    | `extern const Bebop_Timestamp Name;` | `{.seconds = X, .nanos = Y}`         |
| Duration     | `extern const Bebop_Duration Name;`  | `{.seconds = X, .nanos = Y}`         |

Bytes emit a `static const uint8_t Name_data_[] = {0x01, 0x02, ...};` array followed by a `Bebop_Bytes` struct pointing to it.

---

## Serialization Pipeline

### Size Calculation (`gen_size_type_ex`, line 2824)

Uses an explicit stack (`size_work_t[64]`) for iterative depth-first traversal:

- **Primitives**: `size += BEBOP_WIRE_SIZE_XXX;` (compile-time constant)
- **Strings**: `size += BEBOP_WIRE_SIZE_LEN + str.length + BEBOP_WIRE_SIZE_NUL;`
- **Enums**: Use the base type's size macro
- **Defined types**: Call `Type_EncodedSize(&value)` recursively
- **Arrays (primitive elements)**: `size += BEBOP_WIRE_SIZE_LEN + arr.length * ELEMENT_SIZE;`
- **Arrays (complex elements)**: Loop with nested size calculation
- **Fixed arrays (primitive)**: `size += count * ELEMENT_SIZE;`
- **Fixed arrays (complex)**: Loop with nested size calculation
- **Maps**: `size += BEBOP_WIRE_SIZE_LEN;` + iterate with `Bebop_MapIter`, accumulating key+value sizes

**Struct size** (`gen_size_struct`, line 3007):

- Fixed-size structs return `TYPE_FIXED_SIZE` immediately (the macro is defined to the wire size)
- Variable-size structs sum all field sizes

**Message size** (`gen_size_message`, line 3055):

- Starts with `MIN_SIZE` (4-byte length + 1-byte terminator)
- For each non-deprecated field with `BEBOP_WIRE_IS_SOME`: adds 1 byte (tag) + field size

**Union size** (`gen_size_union`, line 3123):

- Starts with `MIN_SIZE` (4-byte length + 1-byte discriminator)
- Switches on discriminator to add the active branch's size

### Encoding (`gen_encode_type_ex`, line 3183)

Also uses the explicit stack traversal pattern:

- **Primitives/strings**: Single call to wire setter with `BEBOP_WIRE_UNLIKELY` error check
- **Enums**: Cast to base type, use base type's wire setter
- **Defined types**: Call `Type_Encode(w, &value)`
- **Primitive arrays**: Bulk write via `Bebop_Writer_SetXxxArray(w, data, length)` - single memcpy
- **Complex arrays**: Write length as u32, then loop encoding each element
- **Fixed primitive arrays**: `Bebop_Writer_SetFixedXxxArray(w, data, count)`
- **Fixed complex arrays**: Loop encoding each element
- **Maps**: Write length as u32, iterate with `Bebop_MapIter`, encode each key-value pair

**Struct encoding** (`gen_encode_struct`, line 3379):

- Encodes all fields in order (no length prefix)
- Insertion points: `encode_start:Type` and `encode_end:Type`

**Message encoding** (`gen_encode_message`, line 3416):

- Writes a 4-byte length placeholder (`Bebop_Writer_SetLen`)
- Records start position
- For each non-deprecated field with `BEBOP_WIRE_IS_SOME`:
  - Writes 1-byte field tag
  - Encodes field value
- Writes 0-byte terminator
- Fills the length placeholder with actual bytes written (`Bebop_Writer_FillLen`)

**Union encoding** (`gen_encode_union`, line 3480):

- Writes 4-byte length placeholder
- Writes 1-byte discriminator
- Switches on discriminator, encodes the active branch
- Unknown discriminators return `BEBOP_WIRE_ERR_INVALID`
- Fills the length placeholder
- Insertion point: `encode_switch:Type` for custom branches

### Decoding (`gen_decode_type_ex`, line 3622)

The most complex of the three operations, with several optimization paths:

- **Primitives/strings**: Wire getter into the target field, with const-cast if immutable
- **Enums**: Read into a temporary base-type variable, cast-assign to the enum field
- **Defined types**: Call `Type_Decode(ctx, rd, &value)`, with `BEBOP_WIRE_MUTPTR` cast for immutable fields
- **Primitive arrays (zero-copy optimization)**:
  ```c
  arr.data = BEBOP_WIRE_CASTPTR(Type *, Bebop_Reader_Ptr(rd));
  arr.capacity = 0;  // capacity=0 marks as non-owning view
  Bebop_Reader_Skip(rd, length * ELEMENT_SIZE);
  ```
  This avoids allocation entirely by pointing directly into the read buffer.
- **Complex arrays**: Allocate via `Bebop_WireCtx_Alloc`, loop decode with prefetch
- **Fixed arrays (primitive)**: Bulk read via `Bebop_Reader_GetFixedXxxArray`
- **Fixed arrays (complex)**: Loop decode
- **Maps**: Initialize `Bebop_Map` with type-appropriate hash/equality functions, allocate key/value buffers, decode and insert each pair

**Struct decoding** (`gen_decode_struct`, line 3971):

- Prefetches 64 bytes ahead: `BEBOP_WIRE_PREFETCH_R(Bebop_Reader_Ptr(rd) + 64)`
- Decodes fields in order
- Tracks whether allocation context is needed (`def_needs_ctx`)

**Message decoding** (`gen_decode_message`, line 4021):

- Reads 4-byte message length and computes end pointer
- Initializes all fields to `NONE` state
- Loops reading 1-byte tags until 0 or end:
  - Switches on tag
  - Sets `has_value = true`
  - For message/union fields: allocates via context, decodes as pointer
  - For other fields: decodes into `.value` member
  - Unknown tags: seeks to end and breaks (`goto done`)

**Union decoding** (`gen_decode_union`, line 4125):

- Reads 4-byte length and computes end pointer
- Reads 1-byte discriminator
- Switches on discriminator, decodes the matching branch
- Unknown discriminators: seeks to end

---

## Performance Optimizations

### Zero-Copy Array Decoding

For primitive arrays (all types with fixed wire size), the decoder avoids allocation by pointing directly into the read buffer:

```c
arr.data = BEBOP_WIRE_CASTPTR(int32_t *, Bebop_Reader_Ptr(rd));
arr.capacity = 0;  // sentinel: capacity=0 means "view, not owned"
Bebop_Reader_Skip(rd, length * BEBOP_WIRE_SIZE_INT32);
```

This is safe because:

1. The reader buffer outlives the decoded struct (caller manages lifetime)
2. Immutable structs have `const` fields preventing modification
3. The `capacity = 0` sentinel tells cleanup code not to free the pointer

### Bulk Array Operations

Primitive arrays use single-call bulk operations for both encoding and decoding:

- `Bebop_Writer_SetI32Array(w, data, length)` - likely a single memcpy
- `Bebop_Reader_GetFixedI32Array(rd, data, count)` - bulk read

### Cache Prefetch Hints

During array decoding, the generator emits software prefetch instructions:

```c
uint32_t pf_dist = calc_prefetch_dist(elem);
// ...
if (_i0 + pf_dist < _len) BEBOP_WIRE_PREFETCH_W(&_d0[_i0 + pf_dist]);
```

`calc_prefetch_dist()` (line 526) computes the prefetch distance based on element size:

- Distance = 128 / element_size, clamped to [1, 16]
- Targets ~128 bytes ahead (two cache lines)
- Uses `BEBOP_WIRE_PREFETCH_W` (prefetch for write) since elements are being decoded into

### Hot/Pure Attributes

- Encode/Decode functions: `BEBOP_WIRE_HOT` (likely `__attribute__((hot))`)
- EncodedSize functions: `BEBOP_WIRE_PURE` (likely `__attribute__((pure))`)

### Branch Prediction Hints

All error checks use `BEBOP_WIRE_UNLIKELY()` (likely `__builtin_expect(x, 0)`) to hint the processor that the error path is cold.

---

## Memory Management

### Generator-Side

The generator itself uses simple `malloc`/`realloc`/`free` with exponential growth in `gen_sb_t`. The `sb_steal()` function transfers ownership of built strings to the response builder.

### Generated Code Patterns

**Allocation**: All dynamic allocations in decode functions go through `Bebop_WireCtx_Alloc(ctx, size)`. The context is stack-scoped by the caller and manages a memory pool. This avoids per-element `malloc` calls.

**Views vs Owned**: The `capacity = 0` pattern distinguishes views (pointing into read buffer) from owned arrays (allocated via context). This is critical for correct cleanup.

**Context-Free Decoding**: Simple structs that contain only primitives, strings, and enums don't need the allocation context. The generator detects this via `def_needs_ctx()` (line 3605) and emits `BEBOP_WIRE_UNUSED(ctx)` to avoid warnings.

**Const-Correctness**: Immutable structs use `BEBOP_WIRE_MUTPTR(Type, &field)` to cast away const only during decode. This is the only place const is violated, and it's hidden behind a macro for auditing.

---

## Reflection Metadata

When `no_reflection` is false (default), the generator emits comprehensive runtime reflection data.

### Generated Reflection Types

**Definition Descriptor** (`BebopReflection_DefinitionDescriptor`):

- `magic` - validation constant (`BEBOP_REFLECTION_MAGIC`)
- `kind` - enum/struct/message/union/service
- `name` - short name, `fqn` - fully qualified name, `package`
- Union of kind-specific data:
  - `enum_def`: base type, member count, member array, is_flags
  - `struct_def`: field count, field array, sizeof, fixed_size, is_mutable
  - `message_def`: field count, field array, sizeof
  - `union_def`: branch count, branch array, sizeof
  - `service_def`: method count, method array

**Field Descriptor** (`BebopReflection_FieldDescriptor`):

- name, type reference, field index, `offsetof(Type, field)`

**Type Descriptor** (`BebopReflection_TypeDescriptor`):

- Scalar types reference shared static instances (`BebopReflection_Type_Bool`, etc.)
- Complex types (array, fixed_array, map, defined) get static local descriptors
- Tree of nested descriptors for `array<map<string, int32>>` etc.

**Type Info** (`Bebop_TypeInfo`):

- `type_fqn` - string identifier
- `size_fn`, `encode_fn`, `decode_fn` - function pointers for dynamic dispatch

This enables runtime schema introspection, dynamic serialization dispatch, and tools that work with arbitrary Bebop types without compile-time knowledge.

### Deduplication

Type descriptors are deduplicated across fields using `gen_type_set_t`. Each descriptor gets a name like `TypeName__type_arr_i32` and is only emitted once per definition scope.

---

## Naming Conventions

### Generated C Identifiers

| Bebop                        | C                                           | Example                          |
| ---------------------------- | ------------------------------------------- | -------------------------------- |
| Type FQN `pkg.MyType`        | `PrefixPkg_MyType`                          | `Bb_Pkg_MyType`                  |
| Enum member `MyEnum.MyValue` | `MY_ENUM_VALUE`                             | Smart dedup: drops repeated word |
| Struct field `fieldName`     | `fieldName`                                 | Appends `_` if C keyword         |
| Message tag                  | `TYPE_FIELD_TAG`                            | `MY_MSG_NAME_TAG = 3`            |
| Union discriminator          | `Type_Disc` enum                            | `MY_UNION_NONE = 0`              |
| Union branch                 | lowercase in union body                     | `myBranch` -> `mybranch`         |
| Include guard                | `PKG_FILE_BOP_H_`                           | SCREAMING with `_H_` suffix      |
| Size macros                  | `TYPE_MIN_SIZE` / `TYPE_FIXED_SIZE`         |                                  |
| Functions                    | `Type_EncodedSize/Encode/Decode`            |                                  |
| Reflection                   | `Type__refl_descriptor` / `Type__type_info` |                                  |
| Array typedef                | `Bebop_I32_Array` / `Type_Array`            |                                  |

### SCREAMING_SNAKE_CASE Conversion

`sb_puts_screaming_raw()` (line 82): Inserts `_` before uppercase letters preceded by lowercase. `sb_puts_screaming()` (line 117): Additionally deduplicates consecutive identical words.

Example: `myHTTPResponse` -> `MY_HTTP_RESPONSE` (raw) -> `MY_HTTP_RESPONSE` (deduped, no change here). But `HttpHttp` -> `HTTP_HTTP` (raw) -> `HTTP` (deduped, second HTTP dropped).

### Enum Member Naming

`sb_enum_member()` (line 222) generates `TYPE_MEMBER` names. It detects when the last word of the type name matches the first word of the member name and drops the duplicate:

- Type `Color`, member `ColorRed` -> `COLOR_RED` (not `COLOR_COLOR_RED`)
- Type `Status`, member `Ok` -> `STATUS_OK` (no overlap, no dedup)

---

## C Keyword Safety

`C_KEYWORDS[]` (line 437) contains 56 reserved words across C99/C11/C23:

- Standard: `auto`, `break`, `case`, `char`, `const`, `continue`, `default`, `do`, `double`, `else`, `enum`, `extern`, `float`, `for`, `goto`, `if`, `int`, `long`, `register`, `return`, `short`, `signed`, `sizeof`, `static`, `struct`, `switch`, `typedef`, `union`, `unsigned`, `void`, `volatile`, `while`
- C99: `inline`, `restrict`, `_Bool`, `_Complex`, `_Imaginary`
- C11: `_Alignas`, `_Alignof`, `_Atomic`, `_Generic`, `_Noreturn`, `_Static_assert`, `_Thread_local`
- C23: `alignas`, `alignof`, `bool`, `constexpr`, `false`, `nullptr`, `static_assert`, `thread_local`, `true`, `typeof`, `typeof_unqual`, `_BitInt`, `_Decimal32`, `_Decimal64`, `_Decimal128`

`safe_field_name()` appends `_` to conflicting names.

---

## Insertion Points

The generator emits special comments that users can search for to add custom code:

| Insertion Point                                 | Location                | Purpose                  |
| ----------------------------------------------- | ----------------------- | ------------------------ |
| `@@bebop_insertion_point(includes)`             | After includes          | Add custom includes      |
| `@@bebop_insertion_point(forward_declarations)` | After `extern "C"`      | Add custom forward decls |
| `@@bebop_insertion_point(declarations)`         | Before footer           | Add custom declarations  |
| `@@bebop_insertion_point(definitions)`          | Before footer           | Add custom definitions   |
| `@@bebop_insertion_point(eof)`                  | End of file             | Add anything at end      |
| `@@bebop_insertion_point(struct_scope:Type)`    | Inside struct body      | Add custom fields        |
| `@@bebop_insertion_point(encode_start:Type)`    | Start of encode fn      | Pre-encode hooks         |
| `@@bebop_insertion_point(encode_end:Type)`      | End of encode fn        | Post-encode hooks        |
| `@@bebop_insertion_point(encode_switch:Type)`   | In union encode switch  | Custom branches          |
| `@@bebop_insertion_point(decode_start:Type)`    | Start of decode fn      | Pre-decode hooks         |
| `@@bebop_insertion_point(decode_end:Type)`      | End of decode fn        | Post-decode hooks        |
| `@@bebop_insertion_point(decode_switch:Type)`   | In union/message switch | Custom branches          |

---

## Import Handling

`import_generates_code()` (line 1441) checks whether an imported schema contains any code-generating definitions (struct, message, union, enum, const). If an import only contains decorators or services, the `#include` is suppressed.

Import path resolution uses suffix matching: the import path is matched against the end of schema paths. The `.bop` extension is stripped and replaced with `.bb.h` or `.bb.c` depending on output mode.

---

## Schema Edition Support

Currently only `BEBOP_ED_2026` is supported. Any other edition causes an immediate error (`"unsupported schema edition"`). This is checked in `emit_generated_notice()` (line 1498).

---

## Platform Compatibility

- **Windows**: Binary mode for stdin/stdout via `_setmode`/`_O_BINARY`, backslash path handling
- **GCC**: `__attribute__((format(printf, ...)))` for format string checking, `-Wformat-truncation` suppressed
- **Clang**: GCC attributes work, no extra suppressions needed
- **MSVC**: `GEN_PRINTF` macro is a no-op
- **C++ interop**: All generated headers wrapped in `extern "C" { ... }`

---

## Iterative Tree Traversal

A distinctive pattern throughout the generator: **every recursive tree traversal is implemented iteratively using explicit stacks**. This applies to:

- `gen_forward_decls` - traverses nested definitions
- `gen_typedef` / `gen_enums_only` / `gen_container_typedef` / `gen_functions` / `gen_reflection` / `gen_reflection_decl` - all use the same stack frame pattern
- `emit_container_types` - traverses nested types
- `gen_size_type_ex` / `gen_encode_type_ex` / `gen_decode_type_ex` - traverses type trees during code generation

The pattern is:

```c
typedef struct { ...; bool children_pushed; } frame_t;
frame_t stack[GEN_STACK_DEPTH];  // 64 entries
int top = 0;
stack[top++] = (frame_t){root, false};
while (top > 0) {
    frame_t* f = &stack[top - 1];
    if (!f->children_pushed) {
        f->children_pushed = true;
        // push children in reverse order
        continue;
    }
    top--;
    // process this node (post-order)
}
```

Benefits:

- Prevents stack overflow on deeply nested schemas
- Bounded memory usage (GEN_STACK_DEPTH = 64)
- Post-order traversal ensures children are processed before parents

---

## Map Support

Maps use `Bebop_Map` from the runtime, parameterized by type-specific hash and equality functions:

| Key Type  | Hash Function        | Equality Function  |
| --------- | -------------------- | ------------------ |
| `bool`    | `Bebop_MapHash_Bool` | `Bebop_MapEq_Bool` |
| `byte`    | `Bebop_MapHash_Byte` | `Bebop_MapEq_Byte` |
| `int8`    | `Bebop_MapHash_I8`   | `Bebop_MapEq_I8`   |
| `int16`   | `Bebop_MapHash_I16`  | `Bebop_MapEq_I16`  |
| `uint16`  | `Bebop_MapHash_U16`  | `Bebop_MapEq_U16`  |
| `int32`   | `Bebop_MapHash_I32`  | `Bebop_MapEq_I32`  |
| `uint32`  | `Bebop_MapHash_U32`  | `Bebop_MapEq_U32`  |
| `int64`   | `Bebop_MapHash_I64`  | `Bebop_MapEq_I64`  |
| `uint64`  | `Bebop_MapHash_U64`  | `Bebop_MapEq_U64`  |
| `int128`  | `Bebop_MapHash_I128` | `Bebop_MapEq_I128` |
| `uint128` | `Bebop_MapHash_U128` | `Bebop_MapEq_U128` |
| `string`  | `Bebop_MapHash_Str`  | `Bebop_MapEq_Str`  |
| `uuid`    | `Bebop_MapHash_UUID` | `Bebop_MapEq_UUID` |

Map iteration uses `Bebop_MapIter` / `Bebop_MapIter_Init` / `Bebop_MapIter_Next` for both size calculation and encoding.

---

## Error Handling

The generated code uses a consistent error propagation pattern:

```c
Bebop_WireResult r;
if (BEBOP_WIRE_UNLIKELY((r = SomeOperation(...)) != BEBOP_WIRE_OK)) return r;
```

Error codes include:

- `BEBOP_WIRE_OK` - success
- `BEBOP_WIRE_ERR_OOM` - out of memory (allocation failure)
- `BEBOP_WIRE_ERR_INVALID` - invalid data (unknown union discriminator)

The `in_step_fn` flag changes the return pattern to `return -(int)r` for step function contexts.

---

## Service Reflection

Services are handled only in reflection metadata generation (`gen_refl_service`, line 2580). No encode/decode functions are generated for services - they are introspection-only.

Each service method descriptor includes:

- Method name and ID
- Request/response type descriptors
- Method type: `UNARY`, `SERVER_STREAM`, `CLIENT_STREAM`, `DUPLEX_STREAM`

---

## File Filtering

`should_generate()` (line 4378) determines which schemas to process. If the request specifies files (via `bebop_plugin_request_file_count`), only matching schemas are processed. Matching uses:

1. Exact path equality
2. Schema path ends with requested file path
3. Requested file path ends with schema path

If no files are specified, all schemas are generated.

---

## Summary of Key Design Decisions

1. **Single-file generator**: All 4660 lines in one file for simplicity and portability
2. **Iterative over recursive**: Prevents stack overflow, bounded memory
3. **Explicit stack state machines**: Same pattern for all tree traversals
4. **Zero-copy by default**: Primitive arrays decoded as views into read buffer
5. **Struct field reordering**: Optimizes memory layout for minimal padding
6. **Deduplication everywhere**: Type sets, include guards, and word dedup in naming
7. **Const-correctness**: Immutable types use `const` fields, surgical `MUTPTR` for decode
8. **Prefetch hints**: Software prefetch for cache-efficient array processing
9. **Bulk operations**: Single-call array encode/decode for primitives
10. **Insertion points**: Extensibility without modifying generated code
11. **Context-based allocation**: Single `Bebop_WireCtx` manages all decode allocations
12. **Platform portability**: Windows binary mode, cross-compiler attribute macros
