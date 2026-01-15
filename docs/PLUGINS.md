# Bebop plugins

Plugins generate target-language code from compiled Bebop schemas. Each plugin is a standalone executable that reads a request from stdin and writes a response to stdout, both encoded using Bebop's wire format.

## Naming and invocation

Plugin executables are named `bebopc-gen-$NAME`. The compiler invokes them via command-line flags:

```
bebopc build schema.bop --c_out=./generated
bebopc build schema.bop --typescript_out=./gen --c_out=./out
```

The `--${NAME}_out=DIR` flag:
1. Locates `bebopc-gen-${NAME}` in PATH
2. Spawns the plugin process
3. Sends a `CodeGeneratorRequest` to stdin
4. Reads a `CodeGeneratorResponse` from stdout
5. Writes generated files to DIR

## Protocol

Plugins communicate using Bebop messages defined in `bebop/plugin.bop`.

### CodeGeneratorRequest

Input message sent to plugin stdin.

```bebop
message CodeGeneratorRequest {
    files_to_generate(1): string[];
    parameter(2): string;
    compiler_version(3): Version;
    schemas(4): SchemaDescriptor[];
    host_options(5): map[string, string];
}
```

**files_to_generate** - Source files explicitly listed on the command line. Generate code only for these files.

**parameter** - Plugin-specific options from `--${NAME}_opt=PARAM`. Format is plugin-defined (commonly `key=value` pairs).

**compiler_version** - Version of the invoking compiler. Check for compatibility.

**schemas** - Schema descriptors for all files in `files_to_generate` plus their imports. Topologically sorted: dependencies before dependents. Type FQNs are fully resolved.

**host_options** - Global compiler options. Plugins can adjust output based on these.

### CodeGeneratorResponse

Output message written to plugin stdout.

```bebop
message CodeGeneratorResponse {
    error(1): string;
    files(2): GeneratedFile[];
    diagnostics(3): Diagnostic[];
}
```

**error** - If non-empty, generation failed. Set this for schema problems that prevent correct code generation. The plugin should still exit zero.

**files** - Generated files to write.

**diagnostics** - Warnings, errors, or info messages for the user.

### GeneratedFile

```bebop
message GeneratedFile {
    name(1): string;
    insertion_point(2): string;
    content(3): string;
    generated_code_info(4): SourceCodeInfo;
}
```

**name** - Output path relative to output directory. Use `/` as separator on all platforms. Must not contain `..` or start with `/`.

**insertion_point** - For extending another plugin's output. See insertion points below.

**content** - File contents or insertion fragment.

**generated_code_info** - Optional source mapping for IDE features.

### Diagnostic

```bebop
message Diagnostic {
    severity(1): DiagnosticSeverity;
    text(2): string;
    hint(3): string;
    file(4): string;
    span(5): int32[4];
}
```

**severity** - ERROR (0), WARNING (1), INFO (2), HINT (3).

**text** - Human-readable diagnostic message.

**hint** - Optional suggestion for fixing the issue.

**file** - Source file this diagnostic relates to.

**span** - Source location as `[start_line, start_col, end_line, end_col]`. 1-based.

## Error handling

Two error paths exist:

| Condition | Action |
|-----------|--------|
| Schema problem (invalid input) | Set `error` field, exit zero |
| Plugin bug or environment problem | Write to stderr, exit non-zero |

The distinction matters. Stderr output with non-zero exit indicates a plugin bug. The `error` field indicates problems in the `.bop` files.

## Insertion points

Plugins can extend files produced by other plugins using insertion points.

A generator emits markers in its output:

```c
// @@bebopc_insertion_point(namespace_scope)
```

A later plugin targets this point:

```
file.name = "types.h"
file.insertion_point = "namespace_scope"
file.content = "// My extension code\n"
```

The content is inserted immediately above the marker. Multiple insertions to the same point appear in plugin execution order.

Use cases:
- Adding custom serialization methods
- Injecting validation logic
- Appending language-specific utilities

## Implementing a plugin

### Minimal structure

```c
int main(void) {
#ifdef _WIN32
    _setmode(_fileno(stdin), _O_BINARY);
    _setmode(_fileno(stdout), _O_BINARY);
#endif

    // Read request from stdin
    uint8_t* buf = read_all_stdin(&len);

    // Decode request
    bebop_context_t* ctx = bebop_context_create(&host);
    bebop_plugin_request_t* req;
    bebop_plugin_request_decode(ctx, buf, len, &req);

    // Process schemas
    uint32_t file_count = bebop_plugin_request_file_count(req);
    for (uint32_t i = 0; i < file_count; i++) {
        const char* file = bebop_plugin_request_file_at(req, i);
        // Find matching schema and generate code
    }

    // Build response
    bebop_plugin_response_builder_t* b =
        bebop_plugin_response_builder_create(&alloc);
    bebop_plugin_response_builder_add_file(b, "output.h", content);

    // Encode and write to stdout
    bebop_plugin_response_t* resp = bebop_plugin_response_builder_finish(b);
    bebop_plugin_response_encode(ctx, resp, &out_buf, &out_len);
    fwrite(out_buf, 1, out_len, stdout);

    return 0;
}
```

### Processing schemas

Iterate schemas and check if each matches `files_to_generate`:

```c
uint32_t schema_count = bebop_plugin_request_schema_count(req);
for (uint32_t i = 0; i < schema_count; i++) {
    const bebop_descriptor_schema_t* s =
        bebop_plugin_request_schema_at(req, i);
    const char* path = bebop_descriptor_schema_path(s);

    // Check if this schema should be generated
    bool should_generate = false;
    for (uint32_t j = 0; j < file_count; j++) {
        if (strcmp(path, bebop_plugin_request_file_at(req, j)) == 0) {
            should_generate = true;
            break;
        }
    }

    if (should_generate) {
        // Generate code for this schema
        generate_schema(s);
    }
}
```

### Accessing definitions

Use the descriptor API to traverse types:

```c
uint32_t def_count = bebop_descriptor_schema_def_count(s);
for (uint32_t i = 0; i < def_count; i++) {
    const bebop_descriptor_def_t* d = bebop_descriptor_schema_def_at(s, i);
    bebop_def_kind_t kind = bebop_descriptor_def_kind(d);
    const char* name = bebop_descriptor_def_name(d);
    const char* fqn = bebop_descriptor_def_fqn(d);

    switch (kind) {
        case BEBOP_DEF_STRUCT:
            generate_struct(d);
            break;
        case BEBOP_DEF_MESSAGE:
            generate_message(d);
            break;
        case BEBOP_DEF_ENUM:
            generate_enum(d);
            break;
        // ...
    }
}
```

See DESCRIPTOR.md for the full descriptor API.

## C API reference

### Request decoding

```c
bebop_status_t bebop_plugin_request_decode(
    bebop_context_t* ctx,
    const uint8_t* buf,
    size_t len,
    bebop_plugin_request_t** out);

void bebop_plugin_request_free(bebop_plugin_request_t* req);
```

### Request accessors

```c
uint32_t bebop_plugin_request_file_count(const bebop_plugin_request_t* req);
const char* bebop_plugin_request_file_at(
    const bebop_plugin_request_t* req, uint32_t idx);

const char* bebop_plugin_request_parameter(const bebop_plugin_request_t* req);

bebop_version_t bebop_plugin_request_compiler_version(
    const bebop_plugin_request_t* req);

uint32_t bebop_plugin_request_schema_count(const bebop_plugin_request_t* req);
const bebop_descriptor_schema_t* bebop_plugin_request_schema_at(
    const bebop_plugin_request_t* req, uint32_t idx);

uint32_t bebop_plugin_request_host_option_count(
    const bebop_plugin_request_t* req);
const char* bebop_plugin_request_host_option_key(
    const bebop_plugin_request_t* req, uint32_t idx);
const char* bebop_plugin_request_host_option_value(
    const bebop_plugin_request_t* req, uint32_t idx);
```

### Response builder

```c
bebop_plugin_response_builder_t* bebop_plugin_response_builder_create(
    bebop_host_allocator_t* alloc);

void bebop_plugin_response_builder_set_error(
    bebop_plugin_response_builder_t* b, const char* error);

void bebop_plugin_response_builder_add_file(
    bebop_plugin_response_builder_t* b,
    const char* name,
    const char* content);

void bebop_plugin_response_builder_add_insertion(
    bebop_plugin_response_builder_t* b,
    const char* name,
    const char* insertion_point,
    const char* content);

void bebop_plugin_response_builder_add_diagnostic(
    bebop_plugin_response_builder_t* b,
    bebop_plugin_severity_t severity,
    const char* text,
    const char* hint,
    const char* file,
    const int32_t span[4]);

bebop_plugin_response_t* bebop_plugin_response_builder_finish(
    bebop_plugin_response_builder_t* b);

void bebop_plugin_response_builder_free(bebop_plugin_response_builder_t* b);
```

### Response encoding

```c
bebop_status_t bebop_plugin_response_encode(
    bebop_context_t* ctx,
    const bebop_plugin_response_t* resp,
    const uint8_t** out_buf,
    size_t* out_len);

void bebop_plugin_response_free(bebop_plugin_response_t* resp);
```

### Response decoding (for testing)

```c
bebop_status_t bebop_plugin_response_decode(
    bebop_context_t* ctx,
    const uint8_t* buf,
    size_t len,
    bebop_plugin_response_t** out);

const char* bebop_plugin_response_error(const bebop_plugin_response_t* resp);

uint32_t bebop_plugin_response_file_count(const bebop_plugin_response_t* resp);
const char* bebop_plugin_response_file_name(
    const bebop_plugin_response_t* resp, uint32_t idx);
const char* bebop_plugin_response_file_insertion_point(
    const bebop_plugin_response_t* resp, uint32_t idx);
const char* bebop_plugin_response_file_content(
    const bebop_plugin_response_t* resp, uint32_t idx);

uint32_t bebop_plugin_response_diagnostic_count(
    const bebop_plugin_response_t* resp);
bebop_plugin_severity_t bebop_plugin_response_diagnostic_severity(
    const bebop_plugin_response_t* resp, uint32_t idx);
const char* bebop_plugin_response_diagnostic_text(
    const bebop_plugin_response_t* resp, uint32_t idx);
const char* bebop_plugin_response_diagnostic_hint(
    const bebop_plugin_response_t* resp, uint32_t idx);
const char* bebop_plugin_response_diagnostic_file(
    const bebop_plugin_response_t* resp, uint32_t idx);
const int32_t* bebop_plugin_response_diagnostic_span(
    const bebop_plugin_response_t* resp, uint32_t idx);
```

## Version struct

```c
typedef struct {
    int32_t major;
    int32_t minor;
    int32_t patch;
    const char* suffix;  // "alpha.1", "rc.2", or empty for stable
} bebop_version_t;
```

## Severity enum

```c
typedef enum {
    BEBOP_PLUGIN_SEV_ERROR = 0,
    BEBOP_PLUGIN_SEV_WARNING = 1,
    BEBOP_PLUGIN_SEV_INFO = 2,
    BEBOP_PLUGIN_SEV_HINT = 3
} bebop_plugin_severity_t;
```
