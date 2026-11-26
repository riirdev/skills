use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerMode {
    Mechanical,
    Incremental,
    CleanRoom,
    DualLang,
}

impl LedgerMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mechanical" => Some(Self::Mechanical),
            "incremental" => Some(Self::Incremental),
            "clean-room" => Some(Self::CleanRoom),
            "dual-lang" => Some(Self::DualLang),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mechanical => "Route A: Mechanical 1:1 Port",
            Self::Incremental => "Route B: Incremental Strangler-Fig",
            Self::CleanRoom => "Route C: Clean-Room Specification Rewrite",
            Self::DualLang => "Route D: Dual-Language Paired Evolution",
        }
    }
}

pub fn init_ledger(
    dir: &Path,
    mode: LedgerMode,
    force: bool,
    apply: bool,
) -> Result<Vec<String>, String> {
    let files = [
        ("PORTING.md", render_porting_md(mode)),
        ("LIFETIMES.tsv", render_lifetimes_tsv()),
        ("PARITY.md", render_parity_md(mode)),
        ("UNSAFE-MANIFEST.json", render_unsafe_manifest()),
    ];

    let mut logs = Vec::new();
    let mode_tag = if apply { "create" } else { "dry-run" };

    for (name, _) in &files {
        let target = dir.join(name);
        if target.exists() && !force {
            return Err(format!(
                "{} already exists (pass --force to overwrite)",
                target.display()
            ));
        }
        logs.push(format!("[{mode_tag}] {}", target.display()));
    }

    if apply {
        fs::create_dir_all(dir).map_err(|e| format!("failed to create {}: {e}", dir.display()))?;
        for (name, content) in files {
            let target = dir.join(name);
            fs::write(&target, content)
                .map_err(|e| format!("failed to write {}: {e}", target.display()))?;
        }
    }

    Ok(logs)
}

fn render_porting_md(mode: LedgerMode) -> String {
    format!(
        "\
# PORTING.md — Source-to-Rust Translation Rulings

- Route: {}
- Reference revision: `<pin exact read-only commit or version>`
- Target Rust edition: `2024`

## P-01: Primitive Types & Arithmetic
- Document integer width mappings and overflow policy (`wrapping_*` vs `checked_*` vs `saturating_*`).

## P-02: Structs, Enums, Unions & ABI Layout
- Shared ABI types use `#[repr(C)]` with compile-time `size_of` / `offset_of!` assertions.

## P-03: Strings & Encodings
- Document canonical internal encoding (UTF-8 / WTF-8 / raw bytes) and invalid-sequence behavior.

## P-04: Error Handling & Cleanup
- Map source error paths faithfully; forbid `todo!()`, silent error swallowing, and unguarded `.unwrap()`.

## P-05: Concurrency & Global State
- Track every global/singleton in `LIFETIMES.tsv`; avoid `static mut` in Phase 2+ code.
",
        mode.label()
    )
}

fn render_lifetimes_tsv() -> String {
    "file\tkind\tcontainer\tfield_or_symbol\tsource_type\townership\tfreed_by\tthread_affinity\tcrosses_nonlocal_jump\tlayout_frozen\tnote\n".to_string()
}

fn render_parity_md(mode: LedgerMode) -> String {
    format!(
        "\
# PARITY.md — Acceptance Ladder & Equivalence Ledger

- Route: {}
- Primary Rule: Reference behavior is the specification; any unapproved divergence is a bug.

## Acceptance Ladder (L0–L5)

| Rung | Status | Evidence Command | Notes |
| --- | --- | --- | --- |
| L0 Smoke & Core Primitives | pending | `riir-skills diff-run ...` | |
| L1 Core Library Modules | pending | `cargo test` | |
| L2 Subsystem Integration | pending | | |
| L3 Cold Startup / Snapshot | pending | | |
| L4 Standard Upstream Suite | pending | | 0 skipped, 0 deleted |
| L5 Stress / Fuzz / Fault Injection | pending | | |
",
        mode.label()
    )
}

fn render_unsafe_manifest() -> String {
    "\
{
  \"schema\": \"riir-unsafe-manifest\",
  \"forbid_static_mut\": true,
  \"require_safety_docs\": true,
  \"allowed_boundary_files\": []
}
"
    .to_string()
}
