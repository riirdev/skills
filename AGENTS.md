# Repository Operating Contract

This repository provides a cross-tool Agent Skills suite (`skills/`) and a companion zero-dependency pure-Rust CLI (`riir-skills`) focused on Rust engineering and Rewrite It In Rust (RIIR) workflows.

## Hard Rules

1. **Zero Third-Party Project References**: Never include any third-party project names, personal or private repository names, author handles outside this repo, local filesystem paths, or tokens in skills, references, code, tests, fixtures, or commit messages.
2. **Pure Standard-Library Rust**: Keep `Cargo.toml` free of external crate dependencies so `riir-skills` compiles deterministically offline in seconds. Maintain `#![forbid(unsafe_code)]` across `src/` and `tests/`.
3. **Standard Skill Format**: Every directory under `skills/<name>/` must define `SKILL.md` with valid YAML frontmatter (`name` matching the directory name, `description` <= 1024 characters) and valid relative links under that skill directory.
4. **Safe Installer Semantics**: Installation and uninstallation commands must default to dry-run and apply disk changes only with `--apply`. Never overwrite unmanaged files or locally modified files without explicit `--force`.
5. **Clean Verification & Hygiene**:
   - Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo run -- validate`, and `cargo run -- unsafe-audit src tests --forbid-static-mut --require-safety-docs --max-unsafe-blocks 0 --forbid-file-allows`.
   - Clean up temporary build artifacts (`target/`) after final verification when preparing a clean working tree.
   - Sign all commits (`git commit -S`).
