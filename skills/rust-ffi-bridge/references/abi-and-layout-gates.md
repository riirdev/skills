# ABI & Layout Gates

## 1. Compile-Time Rust Assertions

Pair every shared `#[repr(C)]` struct with compile-time layout assertions so header drift or target ABI changes fail the build immediately rather than corrupting memory at runtime:

```rust
use core::mem::{align_of, offset_of, size_of};

#[repr(C)]
pub struct NativeHeader {
    pub version: u32,
    pub flags: u32,
    pub payload_len: usize,
    pub ptr: *mut u8,
}

const _: () = {
    assert!(size_of::<NativeHeader>() == 24);
    assert!(align_of::<NativeHeader>() == 8);
    assert!(offset_of!(NativeHeader, version) == 0);
    assert!(offset_of!(NativeHeader, flags) == 4);
    assert!(offset_of!(NativeHeader, payload_len) == 8);
    assert!(offset_of!(NativeHeader, ptr) == 16);
};
```

## 2. Bidirectional C-to-Rust Verification

1. Generate or run a small C probe against the exact system/vendored headers (`#include <target.h>`) printing `sizeof(...)`, `_Alignof(...)`, and `offsetof(...)` for every shared struct and ABI version macro.
2. Compare C output against Rust `size_of` / `offset_of!` assertions in CI.
3. Maintain a monotonic ratchet file (`asserted-keys.txt`) so existing layout assertions are never silently deleted when refactoring.

## 3. Common FFI Pitfalls Checklist

- **Wrong Header Version**: Compiling Rust declarations written from upstream docs while linking a system library of a different ABI revision (e.g., ABI v80 vs v100). Always bind-check `ABI_VERSION` at init and verify linked library headers.
- **`#[no_mangle]` Symbol Collisions**: Prefix all internal C-ABI helper exports consistently and verify uniqueness via `nm -g` so two crates never emit colliding unmangled symbols.
- **Allocator Mismatch**: Never allocate in Rust (`Box::into_raw` / `Vec`) and free with C `free()`, or vice versa. Always export a paired `crate_free_*` destructor.
- **Forgotten Re-Signing After `install_name_tool` / Patching**: Rewriting `@rpath` invalidates code signatures on Apple platforms, causing immediate `SIGKILL` at launch. Always re-sign and verify signature + clean dynamic dependency edges in the packaging step.
