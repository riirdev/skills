use std::env;
use std::path::{Path, PathBuf};

use crate::diff_run::{self, DiffConfig};
use crate::installer::{self, InstallMode, InstallRequest, UninstallRequest};
use crate::ledger::{self, LedgerMode};
use crate::targets::Scope;
use crate::unsafe_audit::{self, AuditThresholds};
use crate::validate;

pub fn run() -> i32 {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help" | "help") {
        print_help();
        return 0;
    }
    if matches!(args[0].as_str(), "-V" | "--version") {
        println!("riir-skills {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }

    let sub = args[0].as_str();
    let rest = &args[1..];
    match sub {
        "list" => cmd_list(rest),
        "install" => cmd_install(rest),
        "check" | "doctor" => cmd_check(rest),
        "uninstall" => cmd_uninstall(rest),
        "validate" => cmd_validate(rest),
        "unsafe-audit" => cmd_unsafe_audit(rest),
        "diff-run" => cmd_diff_run(rest),
        "init-ledger" => cmd_init_ledger(rest),
        other => {
            eprintln!("error: unknown subcommand '{other}'\n");
            print_help();
            2
        }
    }
}

fn print_help() {
    println!(
        "\
riir-skills {} — Cross-agent Rust & RIIR engineering skills and verification CLI

USAGE:
  riir-skills <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
  list          List bundled skills and supported AI tool targets
  install       Install skills to AI tool directories (dry-run unless --apply)
  check         Verify installed skills, symlinks, drift, or conflicts
  uninstall     Remove manifest-owned installed skills (dry-run unless --apply)
  validate      Validate SKILL.md frontmatter, links, and plugin manifests
  unsafe-audit  Audit Rust unsafe usage and enforce CI ratchets
  diff-run      Run differential black-box parity checks on JSONL cases
  init-ledger   Scaffold PORTING.md, LIFETIMES.tsv, PARITY.md, UNSAFE-MANIFEST.json

Run `riir-skills <SUBCOMMAND> --help` for subcommand options.",
        env!("CARGO_PKG_VERSION")
    );
}

fn parse_flag_value(args: &[String], idx: &mut usize, flag: &str) -> Result<String, String> {
    if *idx + 1 >= args.len() {
        return Err(format!("missing value for {flag}"));
    }
    *idx += 1;
    Ok(args[*idx].clone())
}

fn cmd_list(args: &[String]) -> i32 {
    let mut repo: Option<PathBuf> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("USAGE: riir-skills list [--repo <path>] [--json]");
                return 0;
            }
            "--repo" => match parse_flag_value(args, &mut i, "--repo") {
                Ok(v) => repo = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--json" => json_out = true,
            other => return usage_err(&format!("unknown flag '{other}'")),
        }
        i += 1;
    }

    let repo_root = match installer::discover_repo_root(repo.as_deref()) {
        Ok(p) => p,
        Err(e) => return runtime_err(&e),
    };
    let skills = match installer::discover_skills(&repo_root) {
        Ok(s) => s,
        Err(e) => return runtime_err(&e),
    };

    if json_out {
        println!("{}", installer::format_list_json(&repo_root, &skills));
    } else {
        println!("Skills ({}):", skills.len());
        for s in &skills {
            println!("  - {:<24} {}", s.name, s.description);
        }
        println!(
            "\nSupported AI Tool Targets ({}):",
            crate::targets::ALL_TOOLS.len()
        );
        for t in crate::targets::ALL_TOOLS {
            println!(
                "  - {:<18} global: ~/{:<28} project: {}",
                t.id, t.global_rel_skills, t.project_rel_skills
            );
        }
    }
    0
}

fn cmd_install(args: &[String]) -> i32 {
    let mut repo: Option<PathBuf> = None;
    let mut tools = "detected".to_string();
    let mut skills = "all".to_string();
    let mut scope = Scope::Global;
    let mut project_dir: Option<PathBuf> = None;
    let mut home: Option<PathBuf> = None;
    let mut mode: Option<InstallMode> = None;
    let mut from_source = false;
    let mut force = false;
    let mut apply = false;
    let mut json_out = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!(
                    "USAGE: riir-skills install [--repo <path>] [--tools <list|all|detected>] [--skills <list|all>] [--scope global|project] [--project-dir <path>] [--home <path>] [--mode symlink|copy] [--from-source] [--force] [--apply] [--json]"
                );
                return 0;
            }
            "--repo" => match parse_flag_value(args, &mut i, "--repo") {
                Ok(v) => repo = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--tools" => match parse_flag_value(args, &mut i, "--tools") {
                Ok(v) => tools = v,
                Err(e) => return usage_err(&e),
            },
            "--skills" => match parse_flag_value(args, &mut i, "--skills") {
                Ok(v) => skills = v,
                Err(e) => return usage_err(&e),
            },
            "--scope" => match parse_flag_value(args, &mut i, "--scope") {
                Ok(v) if v == "global" => scope = Scope::Global,
                Ok(v) if v == "project" => scope = Scope::Project,
                Ok(v) => {
                    return usage_err(&format!("invalid scope '{v}' (expected global|project)"));
                }
                Err(e) => return usage_err(&e),
            },
            "--project-dir" => match parse_flag_value(args, &mut i, "--project-dir") {
                Ok(v) => project_dir = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--home" => match parse_flag_value(args, &mut i, "--home") {
                Ok(v) => home = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--mode" => match parse_flag_value(args, &mut i, "--mode") {
                Ok(v) if v == "symlink" => mode = Some(InstallMode::Symlink),
                Ok(v) if v == "copy" => mode = Some(InstallMode::Copy),
                Ok(v) => return usage_err(&format!("invalid mode '{v}' (expected symlink|copy)")),
                Err(e) => return usage_err(&e),
            },
            "--from-source" => from_source = true,
            "--force" => force = true,
            "--apply" => apply = true,
            "--json" => json_out = true,
            other => return usage_err(&format!("unknown flag '{other}'")),
        }
        i += 1;
    }

    let resolved_mode = mode.unwrap_or(match scope {
        Scope::Global => InstallMode::Symlink,
        Scope::Project => InstallMode::Copy,
    });

    let req = InstallRequest {
        repo,
        tools,
        skills,
        scope,
        project_dir,
        home,
        mode: resolved_mode,
        from_source,
        force,
        apply,
    };

    match installer::run_install(&req) {
        Ok(report) => {
            if json_out {
                println!("{}", report.to_json());
            } else {
                report.print_human();
            }
            if report.has_conflicts() && apply {
                1
            } else {
                0
            }
        }
        Err(e) => runtime_err(&e),
    }
}

fn cmd_check(args: &[String]) -> i32 {
    let mut repo: Option<PathBuf> = None;
    let mut tools = "detected".to_string();
    let mut skills = "all".to_string();
    let mut scope = Scope::Global;
    let mut project_dir: Option<PathBuf> = None;
    let mut home: Option<PathBuf> = None;
    let mut strict = false;
    let mut json_out = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!(
                    "USAGE: riir-skills check [--repo <path>] [--tools <list|all|detected>] [--skills <list|all>] [--scope global|project] [--project-dir <path>] [--home <path>] [--strict] [--json]"
                );
                return 0;
            }
            "--repo" => match parse_flag_value(args, &mut i, "--repo") {
                Ok(v) => repo = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--tools" => match parse_flag_value(args, &mut i, "--tools") {
                Ok(v) => tools = v,
                Err(e) => return usage_err(&e),
            },
            "--skills" => match parse_flag_value(args, &mut i, "--skills") {
                Ok(v) => skills = v,
                Err(e) => return usage_err(&e),
            },
            "--scope" => match parse_flag_value(args, &mut i, "--scope") {
                Ok(v) if v == "global" => scope = Scope::Global,
                Ok(v) if v == "project" => scope = Scope::Project,
                Ok(v) => return usage_err(&format!("invalid scope '{v}'")),
                Err(e) => return usage_err(&e),
            },
            "--project-dir" => match parse_flag_value(args, &mut i, "--project-dir") {
                Ok(v) => project_dir = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--home" => match parse_flag_value(args, &mut i, "--home") {
                Ok(v) => home = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--strict" => strict = true,
            "--json" => json_out = true,
            other => return usage_err(&format!("unknown flag '{other}'")),
        }
        i += 1;
    }

    match installer::run_check(
        repo.as_deref(),
        &tools,
        &skills,
        scope,
        project_dir.as_deref(),
        home.as_deref(),
    ) {
        Ok(report) => {
            if json_out {
                println!("{}", report.to_json());
            } else {
                report.print_human();
            }
            if strict && !report.all_healthy() {
                1
            } else {
                0
            }
        }
        Err(e) => runtime_err(&e),
    }
}

fn cmd_uninstall(args: &[String]) -> i32 {
    let mut tools = "all".to_string();
    let mut skills = "all".to_string();
    let mut scope = Scope::Global;
    let mut project_dir: Option<PathBuf> = None;
    let mut home: Option<PathBuf> = None;
    let mut apply = false;
    let mut json_out = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!(
                    "USAGE: riir-skills uninstall [--tools <list|all|detected>] [--skills <list|all>] [--scope global|project] [--project-dir <path>] [--home <path>] [--apply] [--json]"
                );
                return 0;
            }
            "--tools" => match parse_flag_value(args, &mut i, "--tools") {
                Ok(v) => tools = v,
                Err(e) => return usage_err(&e),
            },
            "--skills" => match parse_flag_value(args, &mut i, "--skills") {
                Ok(v) => skills = v,
                Err(e) => return usage_err(&e),
            },
            "--scope" => match parse_flag_value(args, &mut i, "--scope") {
                Ok(v) if v == "global" => scope = Scope::Global,
                Ok(v) if v == "project" => scope = Scope::Project,
                Ok(v) => return usage_err(&format!("invalid scope '{v}'")),
                Err(e) => return usage_err(&e),
            },
            "--project-dir" => match parse_flag_value(args, &mut i, "--project-dir") {
                Ok(v) => project_dir = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--home" => match parse_flag_value(args, &mut i, "--home") {
                Ok(v) => home = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--apply" => apply = true,
            "--json" => json_out = true,
            other => return usage_err(&format!("unknown flag '{other}'")),
        }
        i += 1;
    }

    let req = UninstallRequest {
        tools,
        skills,
        scope,
        project_dir,
        home,
        apply,
    };

    match installer::run_uninstall(&req) {
        Ok(report) => {
            if json_out {
                println!("{}", report.to_json());
            } else {
                report.print_human();
            }
            0
        }
        Err(e) => runtime_err(&e),
    }
}

fn cmd_validate(args: &[String]) -> i32 {
    let mut repo: Option<PathBuf> = None;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("USAGE: riir-skills validate [--repo <path>] [--json]");
                return 0;
            }
            "--repo" => match parse_flag_value(args, &mut i, "--repo") {
                Ok(v) => repo = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--json" => json_out = true,
            other => return usage_err(&format!("unknown flag '{other}'")),
        }
        i += 1;
    }

    let repo_root = match repo {
        Some(p) => p,
        None => match env::current_dir() {
            Ok(d) => d,
            Err(e) => return runtime_err(&format!("failed to read cwd: {e}")),
        },
    };
    let report = validate::validate_repo(&repo_root);
    if json_out {
        println!("{}", report.to_json());
    } else {
        report.print_human();
    }
    if report.is_ok() { 0 } else { 1 }
}

fn cmd_unsafe_audit(args: &[String]) -> i32 {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut thresholds = AuditThresholds::default();
    let mut json_out = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!(
                    "USAGE: riir-skills unsafe-audit <path...> [--json] [--forbid-static-mut] [--require-safety-docs] [--forbid-file-allows] [--max-unsafe-blocks <N>] [--max-unsafe-fns <N>] [--min-forbid-files <N>]"
                );
                return 0;
            }
            "--json" => json_out = true,
            "--forbid-static-mut" => thresholds.forbid_static_mut = true,
            "--require-safety-docs" => thresholds.require_safety_docs = true,
            "--forbid-file-allows" => thresholds.forbid_file_allows = true,
            "--max-unsafe-blocks" => match parse_flag_value(args, &mut i, "--max-unsafe-blocks") {
                Ok(v) => match v.parse::<usize>() {
                    Ok(n) => thresholds.max_unsafe_blocks = Some(n),
                    Err(_) => return usage_err(&format!("invalid integer '{v}'")),
                },
                Err(e) => return usage_err(&e),
            },
            "--max-unsafe-fns" => match parse_flag_value(args, &mut i, "--max-unsafe-fns") {
                Ok(v) => match v.parse::<usize>() {
                    Ok(n) => thresholds.max_unsafe_fns = Some(n),
                    Err(_) => return usage_err(&format!("invalid integer '{v}'")),
                },
                Err(e) => return usage_err(&e),
            },
            "--min-forbid-files" => match parse_flag_value(args, &mut i, "--min-forbid-files") {
                Ok(v) => match v.parse::<usize>() {
                    Ok(n) => thresholds.min_forbid_files = Some(n),
                    Err(_) => return usage_err(&format!("invalid integer '{v}'")),
                },
                Err(e) => return usage_err(&e),
            },
            other if other.starts_with('-') => {
                return usage_err(&format!("unknown flag '{other}'"));
            }
            path => paths.push(PathBuf::from(path)),
        }
        i += 1;
    }

    if paths.is_empty() {
        paths.push(PathBuf::from("."));
    }

    match unsafe_audit::run_audit(&paths, &thresholds) {
        Ok(report) => {
            if json_out {
                println!("{}", report.to_json());
            } else {
                report.print_human();
            }
            if report.failures.is_empty() { 0 } else { 1 }
        }
        Err(e) => runtime_err(&e),
    }
}

fn cmd_diff_run(args: &[String]) -> i32 {
    let mut legacy: Option<String> = None;
    let mut rust: Option<String> = None;
    let mut cases: Option<PathBuf> = None;
    let mut timeout_ms: u64 = 5000;
    let mut ignore_stderr = false;
    let mut json_out = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!(
                    "USAGE: riir-skills diff-run --legacy <cmd> --rust <cmd> --cases <cases.jsonl> [--timeout-ms <ms>] [--ignore-stderr] [--json]"
                );
                return 0;
            }
            "--legacy" => match parse_flag_value(args, &mut i, "--legacy") {
                Ok(v) => legacy = Some(v),
                Err(e) => return usage_err(&e),
            },
            "--rust" => match parse_flag_value(args, &mut i, "--rust") {
                Ok(v) => rust = Some(v),
                Err(e) => return usage_err(&e),
            },
            "--cases" => match parse_flag_value(args, &mut i, "--cases") {
                Ok(v) => cases = Some(PathBuf::from(v)),
                Err(e) => return usage_err(&e),
            },
            "--timeout-ms" => match parse_flag_value(args, &mut i, "--timeout-ms") {
                Ok(v) => match v.parse::<u64>() {
                    Ok(n) if n > 0 => timeout_ms = n,
                    _ => return usage_err(&format!("invalid positive integer '{v}'")),
                },
                Err(e) => return usage_err(&e),
            },
            "--ignore-stderr" => ignore_stderr = true,
            "--json" => json_out = true,
            other => return usage_err(&format!("unknown flag '{other}'")),
        }
        i += 1;
    }

    let cfg = DiffConfig {
        legacy_cmd: match legacy {
            Some(c) => c,
            None => return usage_err("missing required --legacy <cmd>"),
        },
        rust_cmd: match rust {
            Some(c) => c,
            None => return usage_err("missing required --rust <cmd>"),
        },
        cases_path: match cases {
            Some(p) => p,
            None => return usage_err("missing required --cases <file.jsonl>"),
        },
        timeout_ms,
        ignore_stderr,
    };

    match diff_run::run(&cfg) {
        Ok(report) => {
            if json_out {
                println!("{}", report.to_json());
            } else {
                report.print_human();
            }
            if report.all_passed() { 0 } else { 1 }
        }
        Err(e) => runtime_err(&e),
    }
}

fn cmd_init_ledger(args: &[String]) -> i32 {
    let mut dir: Option<PathBuf> = None;
    let mut mode = LedgerMode::Mechanical;
    let mut apply = false;
    let mut force = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!(
                    "USAGE: riir-skills init-ledger <dir> [--mode mechanical|incremental|clean-room|dual-lang] [--force] [--apply]"
                );
                return 0;
            }
            "--mode" => match parse_flag_value(args, &mut i, "--mode") {
                Ok(v) => match LedgerMode::parse(&v) {
                    Some(m) => mode = m,
                    None => return usage_err(&format!("invalid mode '{v}'")),
                },
                Err(e) => return usage_err(&e),
            },
            "--force" => force = true,
            "--apply" => apply = true,
            other if other.starts_with('-') => {
                return usage_err(&format!("unknown flag '{other}'"));
            }
            path => {
                if dir.is_some() {
                    return usage_err("expected a single target directory");
                }
                dir = Some(PathBuf::from(path));
            }
        }
        i += 1;
    }

    let target = dir.unwrap_or_else(|| Path::new(".").to_path_buf());
    match ledger::init_ledger(&target, mode, force, apply) {
        Ok(messages) => {
            for line in messages {
                println!("{line}");
            }
            0
        }
        Err(e) => runtime_err(&e),
    }
}

fn usage_err(msg: &str) -> i32 {
    eprintln!("error: {msg}");
    2
}

fn runtime_err(msg: &str) -> i32 {
    eprintln!("error: {msg}");
    1
}
