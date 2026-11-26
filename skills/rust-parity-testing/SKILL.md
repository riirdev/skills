---
name: rust-parity-testing
description: Verify behavioral, ABI, error-path, encoding, and artifact equivalence between a reference implementation and a Rust port or rewrite. Use when building differential test harnesses, running L0-L5 acceptance ladders, checking 12-dimension structural parity, or executing fault-injection and fuzzing gates.
metadata:
  suite: riir-skills
---

# Rust Parity Testing

In a Rust rewrite, **"it compiles" proves memory and type rules, not behavioral equivalence**. This skill establishes machine-verifiable parity between the reference system and the Rust implementation.

## Core Parity Rule

> **Reference behavior is the specification; any unapproved divergence is a bug.**
> Never modify, skip, or weaken an upstream test fixture to make the Rust port pass unless the user explicitly approved a specification change.

## 1. The Record → Replay → Diff → Shadow → Gate Loop

1. **Record**: Execute the reference implementation against historical fixtures, boundary generators, and fuzz seeds; serialize inputs, environment, stdin, stdout, stderr, and exit status into deterministic JSONL truth sets (`cases.jsonl`).
   - Always include adversarial edge cases: `-0.0`, `NaN`, `Inf`, `i64::MIN / -1`, negative modulo, integer wrap, empty input, NUL bytes, invalid UTF-8, multi-byte grapheme/emoji boundaries, DST/timezone shifts, and truncated streams.
2. **Replay & Diff**: Run identical cases against the Rust implementation with a hard wall-clock timeout:
   ```sh
   riir-skills diff-run --legacy "<reference-cmd>" --rust "<rust-cmd>" --cases cases.jsonl --timeout-ms 5000
   ```
3. **Shadow (For Live Strangler Migrations)**: Route production-like traffic to both legacy and Rust paths asynchronously, serve the legacy result to callers, and log any byte-level or structural mismatch as a blocking defect.
4. **Gate**: Promotion requires 100% pass rate on the truth set, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and zero untracked regressions.

See [Differential Harness Guide](references/differential-harness.md) for JSONL schema, normalization rules, and property/fuzz integration.

## 2. The 12-Dimension Parity Checklist

For high-fidelity ports (Route A), verify parity across all applicable dimensions in [Twelve Dimensions](references/twelve-dimensions.md):
- **D1–D5 (Structure & Symbols)**: File tree, statement order, functions & linkage, exported/registered APIs, macros/constants/enums.
- **D6–D8 (Memory & Interfaces)**: Struct/union/bitfield memory layouts (`size_of` / `offset_of!`), call graph layering, C headers / CLI flags / module ABI.
- **D9–D12 (Semantics & Artifacts)**: Control flow & error codes, allocator/GC/lifetime semantics, string encodings/charsets, interchangeable binary artifacts & generated outputs.

## 3. The L0–L5 Acceptance Ladder

Advance sequentially; never report an upper rung as passing until lower rungs are green:

| Rung | Scope | Exit Criterion |
| --- | --- | --- |
| **L0** | Smoke & core primitives (arithmetic, error unwinding, basic CLI/API) | Byte-for-byte identical output & exit codes vs reference |
| **L1** | Core standard library / internal modules | Zero load/init errors or warning deltas |
| **L2** | Subsystem bootstrap & cross-module integration | Full subsystem smoke & API table diff empty |
| **L3** | Full application startup / serialization / snapshot round-trip | Clean cold startup and bidirectional artifact load |
| **L4** | Standard upstream test suite (language-neutral suite) | Pass rate >= reference; **0 skipped, 0 deleted tests** |
| **L5** | Expensive/stress tests, differential fuzzing, fault injection (`OOM`, `EINTR`, signals, corrupt inputs), memory/GC stress | 0 unexplained divergences under stress |

## 4. Anti-Fake-Pass Discipline

Guards against misleading "green" test runs discovered in real rewrites:
1. **Negative Control Check**: Verify every new probe or parity fixture can actually fail when the implementation is broken (e.g., mutate expected dimension or inject a known error and confirm non-zero exit).
2. **Clear Caches Between Recovery Tests**: When testing error recovery ("corrupt input followed by valid input"), purge in-memory/on-disk caches first so a stale placeholder isn't mistaken for fresh success.
3. **Assert Both Output AND Clean Diagnostics**: A call returning a fallback value while logging a loader/decoder error is a failure, not a pass.
4. **No Subset Inflation**: Never label a 20% representative subset "100% compatible". Report exact totals (`passed / failed / skipped / blocked`) with input SHA-256 and runtime mode.
5. **Bound Every Execution**: Set explicit per-case and suite timeouts; never let a hung process wait indefinitely.
