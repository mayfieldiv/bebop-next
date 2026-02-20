# Bulk Scalar Array Read/Write Optimization

- [ ] Optimize arrays of fixed-size scalars with bulk read/write #rust-plugin 🔽

Both C and Swift optimize arrays of fixed-size scalar types (integers, floats, bools) using bulk memory operations instead of element-by-element loops. The Rust plugin currently generates per-element loops for all arrays.

## Current Rust Pattern
```rust
// Generated for Vec<u32>:
let count = reader.read_u32()? as usize;
let mut v = Vec::with_capacity(count);
for _ in 0..count {
    v.push(reader.read_u32()?);
}
```

## Optimized Pattern
For arrays of fixed-size types on little-endian platforms, the read can be a single `memcpy`:
```rust
// Bulk read for Vec<u32> (LE platforms):
let count = reader.read_u32()? as usize;
let byte_count = count * size_of::<u32>();
reader.ensure(byte_count)?;
let v: Vec<u32> = unsafe {
    core::slice::from_raw_parts(reader.ptr() as *const u32, count).to_vec()
};
reader.advance(byte_count);
```

Or, more safely, using `bytemuck` or manual `from_le_bytes` chunking.

## Runtime Changes Needed
Add bulk methods to `BebopReader`/`BebopWriter`:
- `read_scalar_array<T: Copy>(count: usize) -> Result<Vec<T>>`
- `write_scalar_array<T: Copy>(items: &[T])`

## Scope
This is a performance optimization. Applicable to arrays of: `bool`, `u8`/`i8`, `u16`/`i16`, `u32`/`i32`, `u64`/`i64`, `u128`/`i128`, `f16`/`bf16`, `f32`/`f64`. Byte arrays (`Vec<u8>`) already have optimized `read_byte_array`/`write_byte_array`.

## References
- Swift: `TypeMapper.swift` uses `readLengthPrefixedArray`/`writeLengthPrefixedArray` for bulk scalar paths
- C: `generator.c` uses `bebop_wire_read_array_bulk`/`bebop_wire_write_array_bulk`
- Production Rust: uses `SliceWrapper` for zero-copy fixed arrays
