---
name: rust-unsafe-hardening
description: Systematically reduce and isolate unsafe Rust code after behavioral parity is established, or harden an existing unsafe-heavy crate. Use when auditing unsafe blocks, eliminating static mut, replacing raw pointers or setjmp/longjmp with RAII and typed handles, enforcing SAFETY comments, or gating unsafe budgets in CI.
metadata:
  suite: riir-skills
---

# Rust Unsafe Hardening

Mechanical ports from C/C++ or low-level systems code rightly allow `unsafe` initially to preserve exact behavior. Leaving thousands of `unsafe` blocks, `static mut` globals, and raw pointer casts unguarded forever forfeits Rust's core safety guarantees. This skill converts `unsafe`-heavy code into layered, audited safe Rust **without altering observable behavior**.

## Core Rules

1. **Parity First, Hardening Second**: Never combine initial behavioral porting with speculative safe-abstraction rewrites unless the design is greenfield. Hardening happens behind green parity gates.
2. **Isolate, Don't Smear**: Push pure logic into `#![forbid(unsafe_code)]` crates/modules; confine unavoidable OS/FFI `unsafe` to small, audited boundary modules.
3. **Measure With Lexer-Accurate Audits**: Always track progress with `riir-skills unsafe-audit` (not naive `grep`), gating `static mut`, missing `// SAFETY:` comments, and `unsafe` block budgets.

## Staged Hardening Roadmap (U1–U8)

Execute in order; verify full parity tests and `unsafe-audit` after each stage:

| Stage | Objective | Key Action |
| --- | --- | --- |
| **U1** | Baseline & Workspace Graph | Consolidate loose compilation units into a single Cargo workspace with one state owner; record baseline `unsafe-audit` counts |
| **U2** | Pure-Logic Leaf Extraction | Move hashing, compression, parsing, string/codec algorithms, and standalone CLI tools into `#![forbid(unsafe_code)]` + `overflow-checks = true` crates behind thin slice bridges |
| **U3** | Structured Non-Local Exit | Replace `setjmp`/`longjmp` or raw jump buffers with `Result<T, E>` (or strictly bounded internal unwind guards + deterministic `Drop` guards); keep external `extern "C"` non-unwinding |
| **U4** | Explicit Ownership & Handles | Replace raw pointers and LIFO-only raw slice assumptions with RAII guards, scoped borrow tokens (`!Send + !Sync` where thread-affine), and generational handles (preventing ABA/use-after-free) |
| **U5** | Eliminate All `static mut` | Replace every `static mut` with `#[repr(transparent)]` interior-mutable cells (`UnsafeCell`/atomics with exact bit-identical `Atomic*` accessors), `OnceLock`, or explicit locks — killing `&mut` aliasing UB |
| **U6** | Safe Domain Primitives | Upgrade core macros, state accessors, and checked indexers (with release-enabled bounds assertions) so high-level business/domain modules compile under `#![forbid(unsafe_code)]` |
| **U7** | Workspace Lint Lock & Docs | Enable `#![deny(unsafe_op_in_unsafe_fn, static_mut_refs)]` workspace-wide, remove file-level `allow(...)` waivers, strip redundant `unsafe` blocks flagged by `--force-warn unused-unsafe`, and reach 100% `// SAFETY:` coverage |
| **U8** | Manifest Lock & CI Ratchet | Lock the remaining FFI/OS boundary files in `UNSAFE-MANIFEST.json` (caller, invariant, failure path) and enforce CI gates |

See [Staged Hardening Playbook](references/staged-hardening-playbook.md) and [Safety Comment Standard](references/safety-comment-standard.md).

## Verification & CI Ratchet Commands

```sh
# Audit current workspace and enforce strict ratchets
riir-skills unsafe-audit src crates \
  --forbid-static-mut \
  --require-safety-docs \
  --forbid-file-allows \
  --max-unsafe-blocks <budget>

# Detect redundant mechanical unsafe wrappers
RUSTFLAGS="--force-warn unused-unsafe" cargo check --all-targets

# Run undefined-behavior & sanitizer checks on pure/unit subsets
cargo +nightly miri test -p <pure-crate>
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test --target <host-triple>
```
