<p align="center">
  <a href="https://riir.dev">
    <img src="docs/pages/assets/logo.svg" alt="riir.dev logo" width="96" height="96">
  </a>
</p>

# riirdev/skills

A specialized suite of **7 Agent Skills** and a **zero-dependency pure-Rust CLI (`riir-skills`)** for planning, executing, verifying, and hardening Rust systems and Rewrite It In Rust (RIIR) migrations.

Website & live documentation: **[https://riir.dev](https://riir.dev)** · Source: **[github.com/riirdev/skills](https://github.com/riirdev/skills)**

## Included Skills

| Skill | Role | When to Use |
| --- | --- | --- |
| [`riir`](skills/riir/SKILL.md) | Flagship RIIR orchestrator | Choosing a rewrite strategy, pre-flight blueprinting (`PORTING.md`, `LIFETIMES.tsv`, risk spikes, 3-file pilot), batching, parallel agent discipline, and Phase-2 hardening |
| [`rust-parity-testing`](skills/rust-parity-testing/SKILL.md) | Behavioral & ABI equivalence | Record/replay/diff harnesses, 12-dimension parity checks, L0–L5 acceptance ladders, fault injection, and anti-fake-pass gates |
| [`rust-ffi-bridge`](skills/rust-ffi-bridge/SKILL.md) | Incremental interop & ABI gates | `#[repr(C)]` layout assertions, `extern "C"` panic boundaries, N-API/PyO3/WASM/C ABI bridges, and retiring strangler shims |
| [`rust-unsafe-hardening`](skills/rust-unsafe-hardening/SKILL.md) | Staged `unsafe` elimination | U1–U8 hardening roadmap, killing `static mut`, wrapping raw pointers in RAII/generational handles, `// SAFETY:` invariants, and audit ratchets |
| [`rust-architecture`](skills/rust-architecture/SKILL.md) | Clean idiomatic Rust design | Workspace crate layering, CLI-as-kernel / thin-shell design, domain errors, graceful shutdown, and deterministic rebuildability |
| [`rust-performance`](skills/rust-performance/SKILL.md) | Latency, throughput & memory | Separating latency/throughput/memory axes, fair baseline gates, profiling, and fixing common Rust port regressions |
| [`rust-dual-lang`](skills/rust-dual-lang/SKILL.md) | Paired dual-language evolution | 1-for-1 paired file/method construction, shared language-neutral contracts, and decoupled test/coverage parity gates |

## Installation

### Option 1: Universal Skills CLI

```sh
npx skills add riirdev/skills
# Or pick individual skills:
npx skills add riirdev/skills --skill riir
```

### Option 2: Native Plugin Manifests

The repository includes native plugin metadata at the root and tool subdirectories:
- `plugin.json`
- `.claude-plugin/plugin.json`
- `.cursor-plugin/plugin.json`

### Option 3: Built-in `riir-skills` Rust Installer & Engineering CLI

The bundled `riir-skills` binary has zero external crate dependencies (`std`-only Rust) and installs symlinks or verified copies across all major AI coding tools:

```sh
# Inspect bundled skills and supported targets
cargo run -- list

# Preview installation for detected tools (dry-run by default)
cargo run -- install --tools detected

# Apply global installation to specific tools (or all)
cargo run -- install --tools antigravity,agy,codex,claude,cursor,opencode,pi,deepseek-harness --apply

# Or install into the current project's tool directories
cargo run -- install --scope project --tools all --apply

# Verify health, drift, broken symlinks, or unmanaged conflicts
cargo run -- check --strict
```

#### Supported AI Tool Targets

| Target Flag | Global Skills Directory | Project Skills Directory |
| --- | --- | --- |
| `agents` | `~/.agents/skills` | `.agents/skills` |
| `antigravity` | `~/.gemini/config/skills` | `.agent/skills` |
| `agy` | `~/.gemini/antigravity-cli/skills` | `.agents/skills` |
| `codex` (alias `chatgpt`) | `~/.codex/skills` | `.agents/skills` |
| `claude` (alias `claude-code`) | `~/.claude/skills` | `.claude/skills` |
| `cursor` | `~/.cursor/skills` | `.cursor/skills` |
| `opencode` | `~/.config/opencode/skills` | `.opencode/skills` |
| `pi` | `~/.pi/agent/skills` | `.pi/skills` |
| `deepseek-harness` (alias `dsh`) | `$DSH_HOME/skills` or `~/.dsh/skills` | `.dsh/skills` |
| `kimi` | `~/.kimi/skills` | `.agents/skills` |
| `kimi-code` | `~/.kimi-code/skills` | `.agents/skills` |
| `copilot` | `~/.copilot/skills` | `.github/skills` |
| `gemini` | `~/.gemini/skills` | `.gemini/skills` |
| `commandcode` | `~/.commandcode/skills` | `.agents/skills` |

## Engineering CLI Tools

Beyond multi-tool installation, `riir-skills` provides deterministic verification utilities for RIIR and Rust projects:

- `riir-skills validate` — Verify `SKILL.md` YAML frontmatter, naming rules, relative links, and plugin manifests.
- `riir-skills unsafe-audit <path...>` — Lexer-accurate Rust `unsafe` inventory (tracks `#![forbid(unsafe_code)]`, `unsafe {}`, `// SAFETY:` comments, `unsafe fn`, `unsafe impl/trait`, `static mut`, and file-level `allow` attributes) with CI ratchet flags (`--forbid-static-mut`, `--require-safety-docs`, `--max-unsafe-blocks N`, `--min-forbid-files N`, `--forbid-file-allows`).
- `riir-skills diff-run --legacy <cmd> --rust <cmd> --cases <cases.jsonl>` — Timeout-bounded differential black-box test runner comparing exit codes, stdout, and stderr byte-for-byte.
- `riir-skills init-ledger <dir> [--mode mechanical|incremental|clean-room|dual-lang] --apply` — Scaffold `PORTING.md`, `LIFETIMES.tsv`, `PARITY.md`, and `UNSAFE-MANIFEST.json` templates.

## Repository Layout

```text
.
├── AGENTS.md
├── Cargo.toml
├── README.md
├── plugin.json
├── .claude-plugin/plugin.json
├── .cursor-plugin/plugin.json
├── skills/
│   ├── riir/
│   ├── rust-parity-testing/
│   ├── rust-ffi-bridge/
│   ├── rust-unsafe-hardening/
│   ├── rust-architecture/
│   ├── rust-performance/
│   └── rust-dual-lang/
├── src/
└── tests/
```
