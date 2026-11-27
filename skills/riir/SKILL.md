---
name: riir
description: End-to-end orchestration for Rewrite It In Rust (RIIR) projects. Use when planning, scoping, executing, or verifying a full or incremental rewrite/port into Rust from C, C++, Go, TypeScript, JavaScript, Python, Java, Kotlin, or other systems/application languages.
metadata:
  suite: riir-skills
---

# RIIR — Rewrite It In Rust Orchestrator

Rewriting a real codebase in Rust fails when teams treat language translation as a monolithic prompt or rewrite architecture and behavior simultaneously without a verification oracle. This skill enforces a disciplined, evidence-driven pipeline: choose the right migration route, build pre-flight blueprints and risk spikes first, execute with zero stubs and bounded parallelism, treat the "second 90%" (verification and parity) as a first-class phase, and harden into idiomatic safe Rust only after behavioral parity is green.

## 1. Choose the Migration Route First

Before writing any Rust code, classify the project into one of four routes using [Strategy Matrix](references/strategy-matrix.md):

| Route | Choose When | Core Discipline |
| --- | --- | --- |
| **A. Mechanical 1:1 Port** | No live hybrid cutover is needed, AND an independent test suite or reference oracle exists | Mirror directory tree, symbol names, control flow, error model, and ABI first; defer idiomatic refactoring to Phase 2 |
| **B. Incremental Strangler-Fig** | The product has active users or continuous releases and cannot freeze development | Port leaf-first / outside-in behind a temporary FFI/ABI bridge; keep legacy fallback until parity gates pass; lift the bridge completely |
| **C. Clean-Room Spec Rewrite** | The legacy runtime itself is the bottleneck (subprocess/GC tax, memory bloat, unmaintainable coupling) | Freeze observable external contracts (CLI, wire protocol, file formats, public API); design an idiomatic Rust kernel + thin shells; verify with differential suites |
| **D. Dual-Language Paired** | Rust must coexist long-term alongside a companion language implementation | Hand off to [`rust-dual-lang`](../rust-dual-lang/SKILL.md) for 1-for-1 paired construction and decoupled parity gates |

## 2. Phase 0: Pre-Flight Blueprinting (Before Mass Porting)

Never launch bulk porting until these artifacts exist and pass review:

1. **Pin a Read-Only Reference Baseline**: Record the exact commit SHA, toolchain flags, and reference binary/runtime. Never modify reference source or upstream test fixtures to make Rust pass.
2. **Author `PORTING.md`**: Document deterministic translation rulings from the source language to Rust (primitives, integer wrap/overflow, strings/encodings, macros, bitfields, unions, error propagation, non-local jumps, allocators, concurrency). Any construct not yet covered gets a documented ruling before coding so one construct never acquires two conflicting styles. Before bulk fan-out, run an adversarial **source-language leakage audit** on `PORTING.md` to strip unnecessary idioms Rust obviates (e.g., threading allocator parameters everywhere when `#[global_allocator]` suffices outside explicit arenas/FFI, keeping manual `deinit()` where `Drop` suffices, or custom list wrappers where `Vec` is behaviorally equivalent).
3. **Generate `LIFETIMES.tsv`**: Trace every struct field, global/static state, and cross-boundary handle before writing code: who allocates, who frees, whether GC/reference-counting manages it, whether it crosses non-local control flow, and whether binary serialization freezes its layout.
4. **Pre-Translation Crate DAG Cycle-Breaking**: Before wiring a multi-crate workspace, classify every circular back-edge symbol into one of five actions (`DELETE`, `TYPE_ONLY`, `MOVE_DOWN`, `FORWARD_DECL`, `GENUINE`) so crate dependencies stay strictly acyclic.
5. **Run Risk Spikes on Hard Unknowns**: Prove feasibility with tiny reproducible checks on the riskiest assumptions first (e.g., `#[repr(C)]` `sizeof`/`offsetof` parity, non-local jump vs `Drop`, conservative stack scanning, signal/async cancellation, platform GUI/OS hooks).
6. **Run a 3-File Pilot**: Port 3 small files of distinct shapes — (1) pure algorithm, (2) stateful data structure + public registration, (3) lookup table / configuration struct — close the full verification loop on all 3, and recalibrate `PORTING.md` before scaling.
7. **Scaffold Ledgers**: Run `riir-skills init-ledger <project-dir> --mode <mechanical|incremental|clean-room|dual-lang> --apply` and see [Blueprinting Templates](references/blueprinting-templates.md).

## 3. Execution Phase: Throughput with Discipline

1. **Dependency-Ordered Batching**: Work bottom-up along the dependency graph:
   - Batch 1: Core types, headers/constants, memory/allocator primitives, math/encoding utilities
   - Batch 2: Data structures, parsers, codecs, state engines
   - Batch 3: OS/IO, subprocesses, networking, concurrency, platform drivers
   - Batch 4: Top-level runtime, UI/protocol shells, serialization/plugin ABI
2. **Mechanical Fidelity Before Idiomatic Polish (Routes A & B)**:
   - In Route A/B initial porting, `unsafe` is permitted and often necessary to match layout, error codes, and evaluation order. **Inventing behavior not present in the reference is forbidden.**
   - Keep `#[repr(C)]` layouts exact and lock them with compile-time `size_of` / `offset_of!` assertions.
   - Defer "making it more idiomatic Rust" until Phase 2 after behavioral parity is proven.
3. **Zero Stubs and Zero Silent Error Swallowing**:
   - Never leave `todo!()`, `unimplemented!()`, empty function bodies, stub default returns, or commented-out logic.
   - Never ignore source error codes or replace checked error paths with `.unwrap()`/`.expect()`.
4. **Compiler & Inventory Errors as a Work Queue**:
   - Do not read an entire legacy codebase linearly looking for bugs. Let `cargo check`, symbol/ABI inventory diffs, and layout gates emit the exact deficit list as a deterministic work queue.
5. **Parallel & Multi-Agent Discipline**:
   - Compute a **deterministic output file-path mapping** (`<name>.rs` vs `<name>/mod.rs` vs `lib.rs`) upfront so workers never invent conflicting module layouts.
   - Isolate parallel workers in separate Git worktrees; never edit the same file concurrently.
   - Commit scoped single-file or single-unit changes; **forbid `git stash`, `git reset --hard`, `git checkout --`, and `git push --force`** in automated loops.
   - Reserve independent diff-only adversarial review for high-risk boundaries, tricky lifetimes, and batch gates rather than churning review loops on trivial edits. See [Multi-Agent Orchestration](references/multi-agent-orchestration.md).
6. **Colocation vs. Separate-Repo Convergence**:
   - Do not incubate a replacement SDK or runtime hidden in a separate repository; colocate the incubating Rust workspace beside the legacy implementation during convergence so every PR forces parity visibility while keeping ONE designated production release source until cutover.
   - When an early drop-in compatibility architecture accumulates broken assumptions, explicitly retire it and forbid reviving it rather than stacking workarounds. Allow source-selectable release rehearsal *only* during cutover, then consolidate cleanly.
7. **Main-Branch Drift Parity Sweep**:
   - On long-running port branches, systematically audit every legacy `main` commit landed since the branch-point before cutover, verify whether the Rust equivalent carries the same fix, and port any missing behavior with regression tests.

## 4. The "Second 90%" — Verification & Parity

Compiling without errors proves Rust type and borrow rules, **not semantic parity**. Budget verification as a full, dedicated phase:

- Walk the **L0–L5 Acceptance Ladder** and run black-box differential testing via [`rust-parity-testing`](../rust-parity-testing/SKILL.md) (and `riir-skills diff-run`).
- Verify performance, tail latency, startup time, and memory footprint via [`rust-performance`](../rust-performance/SKILL.md).
- Perform clean-checkout rebuild verification (reclaim disposable build caches so stale artifacts never mask missing inputs or broken build scripts).

## 5. Phase 2: Post-Parity Hardening & Bridge Retirement

Only after behavioral parity is locked behind green automated gates:

1. Eliminate `static mut`, raw non-local jumps, and unnecessary `unsafe` stage-by-stage via [`rust-unsafe-hardening`](../rust-unsafe-hardening/SKILL.md) (and track progress with `riir-skills unsafe-audit`).
2. **Staged Reference-Source & Shim Retirement**: Optionally retain legacy sources unbuilt in-tree during an initial post-cutover stabilization window solely as a side-by-side behavioral oracle; once stabilized, delete the retired source tree, legacy codegen outputs, temporary export shims, re-export shim files, source-selection switches, and empty placeholder files in dedicated cleanup passes while keeping thin adapters only for external public contracts via [`rust-ffi-bridge`](../rust-ffi-bridge/SKILL.md).
3. **Eliminate Leftover Rust -> C-API -> Rust Roundtrips**: When the legacy host consumed a Rust dependency through its C FFI API, drop the C-API wrapper crate once the host is Rust and depend on the native Rust crate directly.
4. **Workspace-Wide Visibility & Lint Sweeps**: Run `unreachable_pub` (`pub` -> `pub(crate)`) demotion, `dead_code` sweeps, and workspace-wide `clippy` deny-lint passes to prune orphaned port scaffolding and converge workspace crate layering via [`rust-architecture`](../rust-architecture/SKILL.md).

## 6. Companion Skill Routing

| Need | Skill |
| --- | --- |
| Differential testing, L0–L5 ladder, 12 parity dimensions | [`rust-parity-testing`](../rust-parity-testing/SKILL.md) |
| Incremental C/JS/Python/mobile FFI bridges & ABI layout gates | [`rust-ffi-bridge`](../rust-ffi-bridge/SKILL.md) |
| Staged U1–U8 `unsafe` elimination & `// SAFETY:` audits | [`rust-unsafe-hardening`](../rust-unsafe-hardening/SKILL.md) |
| Workspace layering, error design, clean-room Rust architecture | [`rust-architecture`](../rust-architecture/SKILL.md) |
| Latency/throughput/memory benchmarking & hot-path tuning | [`rust-performance`](../rust-performance/SKILL.md) |
| Long-term dual-language paired development & coverage parity | [`rust-dual-lang`](../rust-dual-lang/SKILL.md) |
