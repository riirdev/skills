# Rust Workspace Blueprints

## 1. Library + CLI + Server / Embedded SDK

```text
Cargo.toml               # [workspace] resolver = "2", exclude = ["vendor"], [workspace.lints]
rust-toolchain.toml      # Pinned channel + clippy/rustfmt components
crates/
  app-core/              # #![forbid(unsafe_code)], domain types, pure logic, unit tests
  app-sys/               # Minimal OS/FFI wrappers, compile-time layout assertions
  app-runtime/           # Storage, IO, networking, task lifecycle, shutdown coordination
  app-cli/               # Thin binary entrypoint, machine-readable stdout, diagnostics stderr
  app-ffi/               # Optional thin C ABI / N-API / PyO3 / mobile wrapper over app-runtime
xtask/                   # Repo automation (codegen, layout verification, packaging, release checks)
tests/                   # End-to-end integration tests against compiled artifacts
```

## 2. Workspace Lint Baseline (`Cargo.toml`)

```toml
[workspace.lints.rust]
unsafe_op_in_unsafe_fn = "deny"
unused_must_use = "deny"
unreachable_pub = "deny"
dead_code = "deny"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "warn"
ptr_as_ptr = "deny"
ptr_cast_constness = "deny"
ref_as_ptr = "deny"
borrow_as_ptr = "deny"
undocumented_unsafe_blocks = "deny"
cast_ptr_alignment = "deny"
disallowed_methods = "deny"
disallowed_types = "deny"
disallowed_macros = "deny"
dbg_macro = "deny"
todo = "deny"
unimplemented = "deny"
```

Keep `[lints] workspace = true` in every workspace member, document any intentional mechanical-port `allow` overrides with explicit reasons in `Cargo.toml`, and place vendored dependencies in `[workspace] exclude = ["vendor"]`.

## 3. Compilation Speed Hygiene

- Keep `app-core` free of heavy procedural macros and async runtimes when possible so inner-loop `cargo test -p app-core` finishes in < 1 second.
- Enable `[profile.dev] split-debuginfo = "unpacked"` so DWARF debuginfo stays in `.dwo` files instead of being re-packed into every `.rlib` and static archive.
- Split large crates along compilation seams (`types` / `parser` / `engine` / `transport`) to maximize incremental parallel builds.
- Use `pub(crate)` by default; expose only intentional, documented `pub` APIs.
- Avoid `#[global_allocator]` overrides in libraries or shared runtime crates (let the final binary or host choose).
