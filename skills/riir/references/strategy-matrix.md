# RIIR Strategy Matrix & Language Migration Traps

## 1. Route Comparison

| Criterion | Route A: Mechanical 1:1 Port | Route B: Incremental Strangler-Fig | Route C: Clean-Room Spec Rewrite | Route D: Dual-Language Paired |
| --- | --- | --- | --- | --- |
| **Live production users during rewrite** | No (or greenfield replacement) | Yes — cannot freeze releases | Either (cutover per contract) | Yes — both targets remain active |
| **Independent test oracle exists?** | Required (language-neutral suite) | Helpful; augmented by record/replay | Required (black-box spec & differential suite) | Shared contract + dual suites |
| **Temporary FFI / interop bridge?** | Avoided (single cutover) | Required (leaf-first / outside-in) | Optional (only at process/SDK boundary) | Not needed if standalone binaries |
| **First-pass Rust style** | C/source-shaped (`unsafe` allowed for fidelity) | Bridge-shaped per module, converged later | Idiomatic safe Rust from day one | Idiomatic per language, aligned structure |
| **Primary failure mode** | Subtle semantic drift hidden under compiling code | Permanent hybrid state ("two systems forever") | Re-inventing undocumented edge cases | One side racing ahead and drifting |
| **Exit criterion** | 100% inventory + L0–L5 upstream suite green | 100% callers flipped + bridge deleted + shadow green | 100% spec & differential suite + perf gates green | Parity metrics + coverage gates green on both |

## 2. Source-Language Migration Traps

### C and Preprocessor-Heavy Codebases
- **Macros as l-values**: Macros that expand to assignable slots (not just r-values) silently change semantics if ported as value-returning Rust functions. Verify whether callers assign to or take the address of macro expansions.
- **Integer overflow & wrap**: C unsigned arithmetic wraps modulo $2^W$; signed overflow is UB in C but often relied upon via two's complement. Document explicit `.wrapping_*` vs checked arithmetic rules in `PORTING.md`.
- **Non-local jumps (`setjmp`/`longjmp`)**: Rust `Drop` destructors do NOT run across raw `longjmp`. Never hold `Drop` values across non-local jump frames; convert to `Result<T, E>` or explicit cleanup stacks in Phase 2.
- **Bitfields & `union` padding**: Rust has no built-in C bitfield syntax. Match exact storage-unit width and bit masks under `#[repr(C)]`, and assert both `size_of` and `offset_of!` against the C compiler on the target platform.
- **Conservative stack scanning / custom allocators**: If the legacy runtime inspects raw stack frames or uses a custom GC heap, disabling Rust's global allocator mismatch and keeping tagged objects pinned/rooted across FFI is mandatory until the GC itself is replaced.

### Manual-Memory Systems Languages (e.g., C++ / Zig)
- **Implicit RAII vs manual `defer`**: Porting manual `defer`/error-cleanup paths to Rust `Drop` and `Result` eliminates entire classes of error-path leaks and use-after-free bugs, **provided** cleanup ordering on early return is preserved.
- **Side-Effecting Assertions Ported to `debug_assert!`**: In languages where `assert(expr)` is a regular function whose argument always evaluates (only the trap is compiled out in release), porting `assert(register_item() == expected)` to Rust's `debug_assert!(...)` silently removes the mutation in `--release`! Always hoist side effects out before asserting.
- **Arena / Bump-Allocator `Drop` Hazard**: Resetting or clearing a bump/arena allocator does **not** run `Drop` on arena-allocated objects. Owned heap buffers, file descriptors, or refcounts placed inside arenas silently leak unless explicitly freed before reset or restricted to `Copy`/arena-backed values.
- **Leftover Rust -> C-API -> Rust Roundtrips**: When rewriting a host runtime whose legacy implementation called a Rust dependency through C FFI glue (`*_sys`), delete the C-API wrapper after host migration and depend on the native Rust crate directly to remove `unsafe` callback trampolines and thread-local error shims.
- **Exceptions & error-path divergence**: C++ exceptions or mixed error unions must map to a single explicit error contract; never swallow errors with `.unwrap_or_default()`.
- **Pre-computed lifetime maps**: For large codebases (>100k LOC), extract struct field ownership and pointer lifetimes into `LIFETIMES.tsv` before translating files so workers do not guess borrow vs owned vs index-handle patterns inconsistently.

### GC Runtimes (Go, Java, JVM, C#)
- **GC pauses vs lock/allocator stalls**: Escaping GC stops tail latency spikes, but naive Rust ports that allocate per message or hold `Mutex` across `.await` or heavy work simply replace GC pauses with allocator/lock stalls.
- **Goroutine / green-thread blocking**: Translating blocking synchronous calls directly into an async Rust executor starves worker threads. Explicitly separate CPU-bound / blocking pools from async IO tasks.
- **Reference cycles**: Cyclic object graphs trivial under tracing GC require arenas, generational indices, or explicit `Weak` / graph ownership in Rust.
- **Go/Dynamic JSON `null` vs. Rust `serde` Zero-Value Coercion**: Go `encoding/json` coerces explicit JSON `null` into zero values for non-pointer strings, numbers, and structs, whereas Rust `serde` rejects `null` for non-`Option` fields. Matching type names do not prove parity — test all six shapes separately: missing key, explicit `null`, empty value, zero value, malformed payload, and wrong type.

### Local Persistence, Database Identity & Concurrent Sync Traps
- **Production DB Filename & Schema Identity**: Never rename a released production SQLite/database file or schema path to match an incubating rewrite's convention; existing users would lose local state on upgrade. Always align the incubating rewrite to the shipped production DB path and schema contract.
- **Fresh Schema vs. Legacy Upgrade Migrations**: Model fresh DB initialization and legacy-upgrade migrations separately; keep `ALTER` / migration side effects strictly gated on actual missing columns and idempotent across restarts, with transactional rollback tests.
- **Concurrent Snapshot-Write Staleness**: A helper that safely overwrites stored rows in serial bootstrap code becomes destructive once reused concurrently beside live message/event writes (e.g., overwriting newer previews or clearing unread counts with stale snapshots). Decide staleness atomically inside SQL (`UPDATE ... WHERE incoming_seq >= stored_seq` or empty-target fill), and re-verify every sibling column written in the same statement.

### Dynamic / Event-Loop Runtimes (TypeScript, JavaScript, Python, Ruby)
- **Subprocess & runtime memory tax**: Eliminating an embedded VM often yields 10x+ startup and memory improvements if the Rust core loads in-process via a thin C ABI / native binding instead of spawning child processes over JSON-RPC.
- **Dynamic type coercion & numeric edge cases**: `-0.0`, `NaN` equality, negative modulo, integer-vs-float division, `null` vs `undefined` vs missing key, and UTF-16 vs UTF-8 string indexing are the #1 source of silent parity bugs. Record real inputs/outputs into differential truth sets.
- **Timezone, locale, and regex engines**: Host-runtime regex flags, Unicode property classes, and local timezone rules frequently differ from Rust crates. Pin explicit conformance cases for every locale/regex boundary.

## 3. Common Failure Postmortems to Prevent

1. **"It Compiles, Therefore It Works"**: Rust's borrow checker catches memory corruption, not business logic inversion, off-by-one slicing, or wrong default flags. Always pair compilation with behavioral differential tests.
2. **Big-Bang Rewrite Without an Oracle**: Rewriting without frozen reference tests or recorded truth fixtures guarantees months of regression chasing.
3. **Premature Idiomization**: Re-architecting data structures and renaming every symbol during a mechanical port destroys `git bisect`, breaks diff review, and hides semantic bugs. Port faithfully first, refactor behind green parity gates second.
4. **Permanent Strangler Bridge**: Leaving temporary N-API / FFI / IPC shims alive indefinitely creates double maintenance and latency overhead. Track bridge call sites as a burn-down metric with a hard deletion milestone.
5. **Under-Budgeting the "Second 90%"**: Reaching 100% file translation is roughly the halfway point in effort; integration debugging, encoding/platform edge cases, fuzzing, and performance tuning consume the second half.
6. **Hidden Separate-Repo Incubation & Reviving Retired Seams**: Incubating a rewrite in a separate repository hides subtle API, DB, and error-code drift until cutover explodes. Colocate during convergence, retire broken compatibility experiments permanently instead of reviving them, and switch cleanly to a single production track.
