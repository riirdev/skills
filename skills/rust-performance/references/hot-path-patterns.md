# Hot-Path Rust Optimization Patterns

## 1. Zero-Copy Parsing & `Cow<'a, str>`

Avoid allocating a fresh `String` when 95% of inputs need no unescaping or normalization:

```rust
use std::borrow::Cow;

pub fn decode_token(raw: &str) -> Cow<'_, str> {
    if !raw.contains('\\') {
        Cow::Borrowed(raw)
    } else {
        Cow::Owned(unescape(raw))
    }
}
```

## 2. Keeping Slow Paths Out of Hot Frames

Large stack arrays or cold error formatting inside hot recursive functions bloat stack frames and hurt instruction cache locality:

```rust
#[inline(always)]
pub fn dispatch(args: &[Value]) -> Result<Value, Error> {
    if args.len() <= 4 {
        dispatch_fast(args)
    } else {
        dispatch_slow(args)
    }
}

#[cold]
#[inline(never)]
fn dispatch_slow(args: &[Value]) -> Result<Value, Error> {
    // Handles rare >4-arg path without inflating `dispatch` frame size
    eval_extended(args)
}
```

## 3. Thread-Local Counters Over Global Atomic Contention

Updating a shared `AtomicU64` on every allocation or token causes cache-line ping-pong across cores. Accumulate in a thread-local plain `u64` and flush to the shared summary only on thread exit or coarse safepoints.

## 4. Reuse Scratch Buffers in Loops

```rust
let mut line = String::with_capacity(256);
while reader.read_line(&mut line)? > 0 {
    process(line.trim_end());
    line.clear(); // preserves capacity, zero reallocations
}
```

## 5. Enforcing Runtime-Dispatched SIMD & Workspace Helpers via `clippy.toml`

Standard slice/string iterators like `bytes.iter().position(|&b| b == needle)` often inhibit vectorization due to early exit, while `slice::windows(n).position(...)` degrades to $O(n \cdot m)$ scanning — a catastrophic regression when porting code that originally called runtime-dispatched SIMD helpers:

1. Provide central runtime-dispatched byte/substring search, filesystem IO, process spawn, mutex, and fast-hash wrappers in the workspace core/sys crates.
2. Ban unapproved fallbacks in `clippy.toml` (`disallowed-methods`, `disallowed-types`, `disallowed-macros`), pairing each rule with a `reason` pointing directly to the approved wrapper (e.g., banning `str::find`, `str::contains`, `slice::windows`, `std::mem::zeroed`, `std::sync::Mutex`, `std::collections::HashMap`, `std::println`).
3. Cover element-generic iterator patterns (such as `<[u8]>::contains` or `.iter().position(...)`) with a source lint test if `clippy.toml` cannot match generic trait forms.

## 6. Length-Dispatched Comptime String Tables Over Runtime-Hashed Maps

Runtime static perfect-hash maps compute a full key hash and displacement probe even when the input misses — and misses dominate identifier, header, keyword, and CLI option tables:

- Replace static runtime-hashed lookup tables with a compile-time macro that groups entries by `key.len()` first and emits `match key.len()` + short constant slice comparisons (`match key { b"get" => ..., _ => None }`).
- This removes per-lookup hashing overhead, simplifies inlining and branch prediction on x86_64/aarch64, and eliminates hash-crate build/proc-macro dependencies.
