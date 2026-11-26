# Paired Dual-Language Parity Workflow

## 1. Layer-by-Layer Synchronization Order

When aligning or building dual-language systems, synchronize bottom-up:
1. **Command / Protocol Parsers**: Deterministic generated input/output corpora covering valid, ambiguous, and malformed commands.
2. **Core State & Routing Engine**: State transitions, validation rules, persistence schemas, and authorization boundaries.
3. **Transport / Platform Adapters**: Outbound formatting, retries, rate limits, and platform-specific edge cases.
4. **Agent / Runtime Integration & Lifecycle**: Process management, timeouts, cancellation, and cleanup.
5. **CLI / Packaging / Diagnostics**: Help text, exit codes, error diagnostics, and installer checks.

## 2. Parity Metrics Contract

Track three decoupled numbers from each language's native tooling:

| Metric | Rust Measurement | Companion Measurement | Gate Rule |
| --- | --- | --- | --- |
| **Executable Test Count** | `cargo test -- --list` | Native test block count / `--list` | Both within target parity tolerance; 0 skipped |
| **Pass / Fail Parity** | `cargo test` | Native full test runner | 100% pass on both sides |
| **Line Coverage** | `cargo` `-Cinstrument-coverage` + `llvm-profdata` | Native coverage tool (e.g., `kcov` / `tarpaulin` / `c8`) | Companion coverage >= baseline threshold without dummy tests |

## 3. Convergence Checklist at Phase Boundary

- [ ] Same directory/file pairs exist on both sides with matching method order.
- [ ] Shared JSONL differential suite passes 100% via `riir-skills diff-run`.
- [ ] All error codes, exit statuses, and edge-case behaviors match without fallback shims.
- [ ] Coverage and test counts are verified via explicit inputs with zero cross-repo path assumptions.
