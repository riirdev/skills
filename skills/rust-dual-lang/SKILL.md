---
name: rust-dual-lang
description: Coordinate long-term dual-language projects where Rust evolves alongside a companion language implementation (such as Zig, C++, Go, or TypeScript) with strict behavioral and structural parity. Use when developing paired features 1-for-1, syncing test counts and line coverage, or validating shared cross-language contracts.
metadata:
  suite: riir-skills
---

# Rust Dual-Language Paired Development

When a project intentionally maintains a Rust implementation alongside a companion language implementation (Route D), letting one side race ahead and attempting to "backfill parity later" creates silent semantic drift and massive reconciliation debt. Keep both implementations converged at every step.

## 1. Paired 1-for-1 Construction

1. **One File/Method Pair at a Time**: Write and align each Rust/companion-language module and method pair together before moving to the next pair. Never build an entire subsystem in one language first and batch-port the other later.
2. **Structural Correspondence**:
   - Mirror directory hierarchy, file names, module responsibilities, and method ordering (adapting only root directory and file extensions).
   - Keep identifier words and word order identical, allowing only idiomatic casing (`snake_case` vs `camelCase`).
   - Keep method counts, error surfaces, and observable behaviors aligned without resorting to padding lines or empty placeholder methods.

## 2. Three-Stage Phase Cadence (Applied Per Pair / Module)

1. **Stage 1 — Complete Paired Business Implementation**: Implement full domain logic, error paths, and wiring on both sides for the current module/feature. Do not write tests first or prematurely run partial verification loops.
2. **Stage 2 — Paired Integration & Functional Acceptance**: Write matching end-to-end integration tests for both implementations, execute them, and fix root-cause divergences until both pass 100%.
3. **Stage 3 — Paired Protective Unit Tests & Metrics Gate**: Add matching edge-case/regression unit tests on both sides and run the decoupled parity metrics check.

## 3. Shared Language-Neutral Contract Layer

- Express CLI schemas, wire protocols, parser cases, and error payloads in language-neutral JSON/JSONL contracts and deterministic generated test corpora.
- Run differential black-box checks across both compiled binaries:
  ```sh
  riir-skills diff-run --legacy "<companion-bin>" --rust "<rust-bin>" --cases parity_cases.jsonl
  ```

## 4. Decoupled Repository & Coverage Gates

- **Zero Cross-Repo Coupling**: Never inspect sibling repository directories via hardcoded relative paths or shared build scripts. Pass baseline metrics (test count, pass/fail totals, line coverage %) explicitly via CLI flags or committed contract snapshots.
- **Anti-Gaming Rules**: Never pad test counts with empty tests, tautological `assert!(true)` blocks, or skipped assertions. Every counted test must exercise real production behavior and assert concrete outputs.

See [Paired Parity Workflow](references/paired-parity-workflow.md).
