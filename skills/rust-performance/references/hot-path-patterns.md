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
