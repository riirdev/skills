# `// SAFETY:` Comment Standard

A valid `// SAFETY:` comment explains **why the caller satisfies every precondition of the unsafe operation right now** — never what the code does or a vague "trust me".

## Required Elements

Every `unsafe { ... }` block and `unsafe fn` must state:
1. **Pointer / Slice Validity**: Why non-null, aligned, dereferenceable, within a single allocated object, and initialized for the read/written type.
2. **Aliasing & Lifetime**: Why no conflicting `&mut` exists and why the borrow cannot outlive the backing allocation.
3. **Thread / Concurrency Safety**: Which lock, single-thread token, or atomic ordering guarantees absence of data races.
4. **FFI / ABI Contract** (if calling extern): Why arguments, layout, and error/unwind expectations match the foreign callee.

## Good vs Bad Examples

**Rejected (Vague / Restates Code):**
```rust
// SAFETY: pointer is valid
unsafe { *ptr = 42; }
```

**Accepted (Concrete Invariants):**
```rust
// SAFETY: `buf` was allocated with `len >= header_len` above, stays uniquely owned
// by this function for `'a`, and `offset + 4 <= len` was verified by `checked_add`.
unsafe {
    let slot = buf.as_mut_ptr().add(offset).cast::<u32>();
    slot.write_unaligned(val.to_le());
}
```

## `unsafe fn` Documentation

Every `pub unsafe fn` (and internal `unsafe fn` under `#![deny(unsafe_op_in_unsafe_fn)]`) requires a `# Safety` doc section listing exact caller preconditions, plus explicit inner `// SAFETY:` blocks around each `unsafe` operation inside the body.
