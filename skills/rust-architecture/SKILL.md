---
name: rust-architecture
description: Design and modernize clean, idiomatic, fast-compiling Rust workspaces. Use when structuring greenfield Rust services/CLIs, refactoring a post-rewrite Rust codebase, designing workspace crate layering, defining error/lifecycle boundaries, or enforcing clean-slate contracts without legacy sprawl.
metadata:
  suite: riir-skills
---

# Rust Architecture & Clean Systems Design

Whether starting a clean-room Rust project (Route C) or modernizing a freshly ported codebase after parity passes, long-term velocity depends on crisp crate boundaries, explicit error/lifecycle ownership, and zero legacy sprawl.

## 1. Layered Workspace Architecture

Organize multi-module projects into a strict acyclic DAG of crates (see [Workspace Blueprints](references/workspace-blueprints.md)):

```text
  [ cli ]      [ http/rpc/mcp ]      [ native-host / gui ]   <- Thin shells (IO, UX, wiring)
     \                |                       /
      +---------------+----------------------+
                      |
                      ▼
            [ runtime / engine ]                             <- State machines, async/threads, persistence
             /                 \
            ▼                   ▼
        [ core ]            [ sys / ffi ]                    <- core: #![forbid(unsafe_code)], IO-free
                                                                sys: minimal isolated OS/C boundary
```

- **Pure Core (`#![forbid(unsafe_code)]`)**: Domain models, parsers, planners, state transitions, and validation live in deterministic, IO-free crates. Easy to test in milliseconds, fuzz, and reuse across CLI, server, and embedded targets.
- **CLI as Kernel, Interfaces as Thin Shells**: Build every capability as a white-box, scriptable, deterministic CLI/core operation first. Keep stdout machine-readable (JSON/structured text) and send diagnostics to stderr. Layer HTTP, RPC, MCP, or GUI adapters thinly over the core without duplicating business or authorization logic.

## 2. Clean-Slate Engineering Discipline

1. **Single Canonical Representation**: Never introduce internal `v1`/`v2` format switches, dual-read decoders, field aliases, or legacy fallback shims for unshipped or internal schemas. When a model changes, update all producers, consumers, specs, and tests together in one clean move; Git preserves history.
2. **Protect Real External Contracts**: Publicly released wire protocols, published file formats, and documented APIs *are* compatibility boundaries — protect them explicitly rather than guessing or silently breaking external callers.
3. **No Production Stubs or Shortcuts**: Never ship `todo!()`, silent error fallbacks, or production mocks.

## 3. Error & Lifecycle Boundaries

- **Typed Domain Errors**: Use explicit error enums (`thiserror` or `std::error::Error`) at crate boundaries; attach context at application boundaries; reserve panics strictly for violated internal invariants (`unreachable!` / bounds bugs), never untrusted input or IO errors.
- **Bounded Concurrency & Shutdown**:
  - Use bounded channels, explicit network/process timeouts, and documented lock acquisition order.
  - Long-running services must hold explicit singleton/pid locks if single-instance, release locks cleanly on `SIGINT`/`SIGTERM`/stdin close, and leave zero orphaned child processes.
  - One-shot CLI commands must not bind daemon ports or acquire daemon singleton locks.

## 4. Reproducible Builds & Deterministic Testing

- **Git as Sole Rebuild Truth**: Checking out a clean working copy on a fresh machine with the locked toolchain (`rust-toolchain.toml` + `Cargo.lock`) must build and pass all tests without machine-local caches or untracked artifacts.
- **Workspace Automation (`xtask`)**: Put code generators, layout probes, packaging, and multi-step CI gates into a pure-Rust `xtask` crate rather than brittle shell scripts.
- **Deterministic Offline Tests**: Unit and fast integration tests must run offline using mocked IO/HTTP; gate live external integration tests behind explicit opt-in flags or `#[ignore]`.

## 5. Post-Cutover Consolidation & Source-Authority Hygiene

- **Inventory External Contracts Before Deletion**: Before removing a retired legacy implementation tree, inventory all stable external contracts — artifact filenames, Maven/Swift package coordinates, compiled public symbols, and persisted DB filenames — and adapt thin wrappers to the Rust core instead of breaking downstream consumers.
- **Separate Active Truth From Historical Logs**: Keep active architecture/API docs and agent rules strictly synchronized with the sole remaining core, while preserving immutable historical logs and retrospectives clearly separated from current specifications.
- **Verify Guardrail Claims Against Code**: Never assert that a test, index, or invariant protects a boundary from memory; inspect the actual code and confirm it is table-driven or tested before relying on it.

See [Clean Rust Checklist](references/clean-rust-checklist.md) for pre-merge checks.
