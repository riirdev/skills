# Staged Unsafe Hardening Playbook

## 1. Killing `static mut` Without Breaking Binary/FFI Layout (U5)

In Rust 2024+, creating references to `static mut` is a hard soundness hazard (`static_mut_refs`). When global variables must retain exact C linkage or memory layout, wrap them in a `#[repr(transparent)]` cell backed by `UnsafeCell` and atomic or single-owner accessors:

- Maintain `size_of`, `align_of`, and linker section equivalence.
- Expose `.get()` / `.set()` using relaxed atomic loads/stores for scalar and pointer-sized globals (preserving exact bit patterns for floats, `-0.0`, `NaN`, and tagged pointers), or lock-guarded access for composite structs.
- Target **0 `static mut`** across the entire active product closure.

## 2. Eliminating ABA & LIFO Drop Bugs in Scoped Roots (U4)

A common trap when wrapping raw arena/handle tables is assuming handles are always dropped in strict LIFO order (`vec.truncate(saved_len)`). Early returns, hash map updates, or reordered local variables violate LIFO order and silently corrupt inner live handles.
- **Fix**: Back handle tables with free-list slots + 64-bit generation counters. On `Drop`, release the specific slot and bump its generation; on `get()`, validate both bounds and generation, panicking immediately on stale/use-after-free reads rather than returning stale data.

## 3. Guarding Conservative Stack Scanners & `MaybeUninit`

If any runtime component scans stack memory conservatively or reuses custom stack arenas:
- Never leave uninitialized or dead pointer-sized slots across early exits or unwinds.
- Zero/clear reclaimed stack blocks on every nested scope exit via RAII `Drop` guards, not only when the outermost stack empties.

## 4. Handling `unsafe` Infectiousness in Macros

Macros that expand into `unsafe` blocks force every caller module to allow `unsafe` even when the macro arguments are safe.
- Split safe fixed-arity macro arms from raw variadic/pointer arms, placing `unsafe` strictly inside a small, checked helper function rather than expanding `unsafe {}` at every call site.
- This allows high-level domain files using those macros to enforce `#![forbid(unsafe_code)]` at the crate/file top.

## 5. Manifest-Gated Boundary (`UNSAFE-MANIFEST.json`)

For the minimal OS/FFI files that legitimately require `unsafe`, track each file with:
- `category` (`ffi_abi`, `os_syscall`, `allocator`, `sync_primitive`)
- `owners` / allowed callers
- `invariants` upheld
- `failure_path` (what happens on null, error, or panic)
Reject any PR that introduces `unsafe` in an unlisted file or adds `#![allow(unsafe_op_in_unsafe_fn)]`.
