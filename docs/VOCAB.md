# Bebop vocabulary

Consistent terminology across documentation, code, and error messages.

## Types

| Term | Meaning |
|------|---------|
| **record** | Any aggregate type: struct, message, or union. Use when the specific kind doesn't matter. |
| **struct** | Fixed-layout record. Fields encode in declaration order with no tags or length prefix. |
| **message** | Tagged record. Fields have explicit indices (1-255) and can be added/removed without breaking compatibility. |
| **union** | Discriminated variant. One active branch identified by a discriminator byte. |
| **enum** | Named integer constants. Not a record. |
| **const** | Compile-time constant value. Not a record. |
| **service** | RPC interface definition. Collection of methods with request/response types. |

## Fields

| Term | Meaning |
|------|---------|
| **field** | Named member of a struct or message. |
| **branch** | Variant of a union, identified by discriminator. |
| **index** | Wire tag for message fields (1-255). Struct fields have index 0. |
| **discriminator** | Byte identifying active union branch (1-255). |

## Wire format

| Term | Meaning |
|------|---------|
| **encode** | Serialize a record to bytes. |
| **decode** | Deserialize bytes to a record. |
| **fixed-size** | Struct whose wire size is known at compile time. No length prefix. |
| **variable-size** | Record whose wire size depends on content. Has length prefix. |

## Schema

| Term | Meaning |
|------|---------|
| **FQN** | Fully-qualified name. Package + parent scopes + name, dot-separated: `mypackage.Outer.Inner` |
| **definition** | Top-level or nested declaration: enum, struct, message, union, service, const. |
| **nested** | Definition declared inside another record's body. |
| **package** | Namespace for definitions within a schema file. |
| **edition** | Schema language version (e.g., `edition = "2026"`). |

## Any type

| Term | Meaning |
|------|---------|
| **type_url** | URL identifying a record's type: `{prefix}/{fqn}` |
| **pack** | Serialize a record into an Any with its type_url. |
| **unpack** | Deserialize an Any's value into a typed record. |
