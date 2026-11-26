# Benchmarking & Regression Gate Protocol

## 1. Baseline Ledger Format

Commit reviewed baseline numbers alongside workload definitions (for example, `docs/baseline/bench-release.tsv`):

```tsv
workload	mode	rounds	median_ms	p95_ms	max_rss_kb	ref_median_ms
cold_startup	release	7	18.4	21.2	12400	142.0
parse_large_doc	release	7	44.1	46.8	38200	51.3
```

Never fabricate baseline numbers, switch to an unfair comparator, or overwrite a failing gate threshold to make CI pass.

## 2. CLI & Startup Benchmarks (`hyperfine`)

```sh
# Compare cold startup and single-turn execution
cargo build --release
hyperfine --warmup 2 --runs 10 \
  --export-json bench-startup.json \
  './ref-bin --version' \
  './target/release/app --version'
```

## 3. Microbenchmarks (`criterion`)

- Wrap inputs with `std::hint::black_box(...)` so the optimizer cannot constant-fold away the benchmarked work.
- Separate setup/allocation from the measured iteration (`iter_batched`) when setup is not under test.
- Run under identical power/CPU conditions when comparing branches.

## 4. Profiling Workflow

1. **CPU Flamegraphs**: `samply record ./target/release/app <args>` or `cargo flamegraph` to identify top self-time frames.
2. **Heap & Allocations**: Count total allocations and peak live bytes per request (allocation count often correlates better with tail latency than CPU time).
3. **Binary Size**: `cargo bloat --release -n 30` and `twiggy` (for WASM) to trim monomorphization bloat or heavy unused features.
4. **Release Profile Checklist**:
   - Verify `opt-level = 3` (or `"z"`/`"s"` for size-constrained targets).
   - Test `lto = "thin"` vs `"fat"` and `codegen-units = 1` with actual binary size + throughput numbers.
   - Use PGO (`-Cprofile-generate` -> workload -> `-Cprofile-use`) on mature hot interpreters, parsers, and compilers.
