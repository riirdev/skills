---
name: rust-ffi-bridge
description: Design, verify, and retire FFI and language-interop boundaries in Rust systems and incremental strangler-fig rewrites. Use when bridging Rust with C/C++, TypeScript/N-API, Python/PyO3, Swift, Kotlin/JNI, or WASM, enforcing #[repr(C)] layout assertions, isolating panics, or lifting temporary migration shims.
metadata:
  suite: riir-skills
---

# Rust FFI & Interop Bridge

FFI boundaries are where memory safety, ABI assumptions, panic unwinding, and incremental rewrite schedules succeed or silently corrupt data. This skill enforces **layout-verified, panic-safe, minimal bridges** and ensures temporary migration shims are actually deleted when porting completes.

## 1. Choose the Narrowest Boundary

| Host / Peer | Preferred Mechanism | Boundary Rule |
| --- | --- | --- |
| **C / C++** | `#[repr(C)]`, `extern "C"`, generated headers | Export minimal C ABI or opaque handle pointers; avoid duplicating header constants by hand |
| **TypeScript / JS** | Native addon ABI (`napi-rs`) or WASM | Keep JS objects at the boundary; run domain state & loops inside Rust without per-token roundtrips |
| **Python** | Native extension (`PyO3`) or C ABI | Release the GIL around heavy Rust work; validate shapes once at entry |
| **Swift / Kotlin / Mobile** | Single C ABI + generated Swift/Kotlin wrappers | Never leak Rust allocator lifetimes across mobile boundaries; copy or pin owned buffers explicitly |

## 2. Bidirectional Compile-Time Layout & ABI Gates

Never trust manual header translations or online docs when linking native libraries:

1. **Measure the Actually Linked Header & Library**: Always inspect the headers and libraries resolved on the build host (e.g., `pkg-config`, compiler `-E`), not a different upstream major version. Version mismatches (e.g., struct size 656 vs 664 bytes, shifted fields) fail silently at runtime if unchecked.
2. **Pin Sizes & Field Offsets at Compile Time**: For every `#[repr(C)]` struct shared across FFI, emit Rust compile-time assertions (`const { assert!(size_of::<T>() == ...); }` and `offset_of!(T, field)`) cross-checked against a C `sizeof`/`offsetof` probe in CI.
3. **Verify Bitfields & Unions Byte-for-Byte**: Model C bitfields as fixed integer storage units and test exact byte patterns with all flags set/cleared.
4. **Dynamic Library Relocation Hygiene**: When bundling or rewriting `rpath` / install names on macOS/Linux, re-sign modified binaries (ad-hoc `codesign --force --sign -` + `--verify` on Apple silicon) and keep a single runtime library instance per symbol namespace.
5. **Compiled Bytecode & Final-Archive ABI Parity**:
   - For JVM/Android AAR drop-ins, compare **compiled bytecode public signatures** (`javap -public`), not just Kotlin/Java source compatibility — inherited interface methods can differ from directly declared methods expected by strict binary consumers or reflection.
   - Verify exported symbols on the **final linked archive / `.so`** (`nm -gU` / `otool`), never pre-link intermediate objects.
   - Run actual **R8 / ProGuard shrinking verification** on Android AARs (covering generated UniFFI bindings and JNA runtime) with a negative control proving R8 strips classes when rules are withheld, and guard host apps against stale generated binding sources causing `IncompatibleClassChangeError`.
   - On iOS XCFrameworks, strip dead `__LLVM,__bitcode` sections across **every** archive producer path and assert post-transform absence with `otool`; always rebuild all declared ABIs from fresh source for release.
6. **Two-Stage Mobile FFI Evolution**: Stabilize drop-in compatibility artifacts (preserving artifact names, package coordinates, public error codes, and callback semantics) first, then expose a clean strongly-typed UniFFI/native surface (structs, enums, typed `Result` errors carrying structured numeric `code + message`, never parsing error strings) backed by the same Rust core.

See [ABI & Layout Gates](references/abi-and-layout-gates.md).

## 3. Panic & Unwind Discipline

Unwinding a Rust panic across an `extern "C"` frame is **undefined behavior**:

- **External Boundaries Must Never Unwind**: Wrap FFI entrypoints in `std::panic::catch_unwind`, convert caught panics into explicit status codes or per-thread pending error records, and return cleanly.
- **Explicit Ownership Contracts**: Every FFI signature must document (a) who allocates, (b) who frees, (c) whether pointers may be null, and (d) whether slices/buffers may alias.
- **Keep `extern "C-unwind"` Strictly Internal**: Only use `extern "C-unwind"` inside closed internal subsystems where both caller and callee are controlled and foreign C frames (dynamic plugins, OS callbacks, third-party libraries) are explicitly sealed behind non-unwinding `extern "C"` adapters.

## 4. The Strangler-Fig Bridge Lifecycle

When migrating a live codebase incrementally (Route B), keep the bridge strictly temporary:

1. **Scaffold & Fallback**: Expose the exact existing contract via FFI; unported paths cleanly fall back to the legacy implementation.
2. **Parity Gate Per Unit**: Verify each ported function/module with recorded truth sets and shadow comparison before switching callers.
3. **Burn-Down Accounting**: Track `bridge_exports` and `legacy_call_sites` as explicit countdown metrics in every batch.
4. **Mandatory Bridge Lift**: Once a subsystem reaches 100% Rust, flip the final callers, delete the FFI shim, remove fallback code paths, and eliminate the cross-language serialization/marshal tax.
5. **Eliminate Leftover Rust -> C-API -> Rust Roundtrips**: Audit remaining `*_sys` crates after host migration; whenever a dependency is itself written in Rust and was only wrapped via its C API for the legacy host, delete the C-API glue crate and depend on the native Rust crate directly to remove opaque-handle `unsafe` trampolines, userdata aliasing hazards, and thread-local last-error shims.

See [Strangler Bridge Lifecycle](references/strangler-bridge-lifecycle.md).
