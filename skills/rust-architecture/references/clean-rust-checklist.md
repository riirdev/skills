# Clean Rust Architecture Checklist

Before closing a feature or refactoring phase, confirm:

- [ ] **Layering**: `core` has zero dependency on CLI flags, HTTP frameworks, or UI shells; interfaces delegate directly to shared core operations.
- [ ] **No Internal Sprawl**: Zero `v1`/`v2` internal branches, dead fallback paths, or unused feature flags; obsolete shims deleted completely.
- [ ] **Error Discipline**: Zero `.unwrap()` on untrusted input, network payloads, env vars, or external process output; all error codes/messages remain actionable.
- [ ] **Resource & Process Hygiene**: Every spawned child process, background thread, lockfile, and temp directory is bounded by timeout and cleaned up on drop/shutdown.
- [ ] **Clean-Checkout Rebuild**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` succeed from a fresh clone without pre-seeded local caches.
- [ ] **Secrets & Paths**: Zero hardcoded user paths, tokens, or machine-specific assumptions in source, tests, or fixtures.
