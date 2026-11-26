---
name: rust-performance
description: Measure, gate, and optimize latency, throughput, cold startup, memory footprint, and binary size in Rust programs and RIIR migrations. Use when benchmarking against a legacy baseline, setting CI performance regression gates, diagnosing allocator or lock contention bottlenecks, or tuning hot paths without breaking parity.
metadata:
  suite: riir-skills
---

# Rust Performance & Regression Gating

Rewriting in Rust does **not** automatically make every workload faster. Escaping a garbage collector or subprocess VM removes runtime pauses and startup bloat, yet naive ports frequently regress on allocation churn, UTF-8 re-validation, cloning, or lock contention. Measure fairly, separate the axes, and gate against baselines.

## 1. Separate the Three Performance Axes

Never summarize performance with a single mean number:
- **Latency**: Cold startup time, interactive input-to-render response, tail percentiles (`p95`, `p99`, `max`), and runtime/GC/lock stalls.
- **Throughput**: Batch operations/sec, requests/sec, MB/sec parsing or compilation speed.
- **Memory & Footprint**: Peak RSS, private dirty memory across N concurrent clients, allocations per operation, live heap size, and stripped binary size.

## 2. Fair Benchmarking & CI Regression Gates

1. **Identical Machine, Inputs & Mode**: Compare matched execution modes (e.g., interpreted vs interpreted, compiled vs compiled, release build `--release` only — never debug mode).
2. **Cold vs Warm Separation**: Measure cold process launch (`hyperfine --prepare` clearing warmup if applicable) separately from warm steady-state loops (`criterion`).
3. **Multi-Round Medians & Variance**: Run multiple rounds (e.g., 7+ rounds), record median, variance, and sample count, and compare against committed baseline TSV/JSON files.
4. **Never Cherry-Pick or Waive Regressions**: Gate CI on explicit regression thresholds; if a subsystem is slower than reference, treat it as an open engineering defect until fixed or explicitly scoped.

See [Benchmarking Protocol](references/benchmarking-protocol.md).

## 3. Top 10 Rust Rewrite Bottlenecks & Fixes

1. **`.clone()` Storms**: Borrow slices (`&str`, `&[u8]`, `Cow<'_, str>`) or use `Arc`/`Bytes`/interned IDs instead of cloning owned `String`/`Vec` across layers.
2. **Redundant UTF-8 Validation**: When parsing ASCII protocols or validated buffers, parse `&[u8]` directly or validate once at the boundary rather than re-validating per token.
3. **Micro-Allocation Churn**: Reuse scratch buffers (`buf.clear()`), pre-size collections (`Vec::with_capacity`), or use arena/bump allocators for AST/IR passes.
4. **Lock Contention & False Sharing**: Replace global atomic/mutex counters updated on hot paths with thread-local accumulators merged at phase boundaries; never hold a `Mutex` across `.await` or blocking IO.
5. **Unbuffered IO**: Wrap raw `File`/`TcpStream`/`Stdout` handles in `BufReader`/`BufWriter` or batch writes.
6. **Async Tax on CPU-Bound Loops**: Keep tight synchronous compute off async executors or batch before crossing channels; avoid spawning tasks per tiny item.
7. **Chatty FFI / Subprocess Serialization**: Cross FFI or IPC at request/batch granularity; embedding a Rust core in-process via thin C ABI eliminates JSON-RPC + child VM memory overhead (often 10x+ faster startup and memory).
8. **Error-Path Stack Bloat**: Mark slow/error/fallback paths `#[cold]` and `#[inline(never)]` so hot loops stay compact in instruction cache and avoid unnecessary stack spills.
9. **Hashing Overhead**: Switch non-adversarial internal maps from SipHash to faster deterministic hashers (`FxHash`/`AHash`) or flat/sorted slices for tiny tables.
10. **Unconfigured Release Profiles**: Benchmark with tuned `[profile.release]` (`lto = "thin"` or `"fat"`, `codegen-units = 1`, `strip = true` where symbols aren't needed) and profile before guessing (`samply`, `perf`, `flamegraph`, `cargo-bloat`).

See [Hot-Path Patterns](references/hot-path-patterns.md).

## 4. Mobile Binary & Archive Bloat Traps

- **Dead LLVM Bitcode Sections in iOS Archives**: Preview and release XCFramework scripts frequently drift; unstripped `__LLVM,__bitcode` sections can inflate iOS static archives by 70–85%+. Strip dead bitcode in **every** archive producer and verify absence on the final archive via `otool`.
- **Feature & Duplicate-Code Folding**: Audit Cargo feature flags (avoid pulling unnecessary default features into mobile FFI crates) and fold duplicate generated wrappers before accepting multi-megabyte binary growth.
- **Release-LTO CI Compile Budget**: Aggressive LTO (`lto = "fat"`, `codegen-units = 1`) substantially increases release unit-test compile times in CI; keep fast inner-loop dev tests separate from release-profile artifact gates.
