use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let path = env::temp_dir().join(format!("riir-skills-{prefix}-{pid}-{nanos}-{id}"));
        fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_riir-skills")
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn run_cli(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin_path());
    cmd.args(args);
    cmd.env_remove("DSH_HOME");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("execute riir-skills binary")
}

#[test]
fn validate_passes_on_real_repo_and_fails_on_broken_repo() {
    let out = run_cli(
        &[
            "validate",
            "--repo",
            repo_root().to_str().unwrap(),
            "--json",
        ],
        &[],
    );
    assert!(
        out.status.success(),
        "validate failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"ok\": true"));
    assert!(stdout.contains("\"skills_checked\": 7"));
    assert!(stdout.contains("\"manifests_checked\": 3"));

    // Negative control: broken relative link & mismatched skill name must fail.
    let tmp = TempDirGuard::new("validate-bad");
    let bad_skill = tmp.path().join("skills/wrong-dir");
    fs::create_dir_all(&bad_skill).unwrap();
    fs::write(
        bad_skill.join("SKILL.md"),
        "---\nname: other-name\ndescription: test\n---\n\nSee [missing](references/missing.md).\n",
    )
    .unwrap();

    let bad = run_cli(&["validate", "--repo", tmp.path().to_str().unwrap()], &[]);
    assert!(!bad.status.success(), "broken repo must fail validation");
    let bad_out = String::from_utf8_lossy(&bad.stdout);
    assert!(bad_out.contains("does not match directory name"));
    assert!(bad_out.contains("broken relative link"));
}

#[test]
fn list_json_reports_all_skills_and_targets() {
    let out = run_cli(
        &["list", "--repo", repo_root().to_str().unwrap(), "--json"],
        &[],
    );
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    for skill in [
        "\"riir\"",
        "\"rust-parity-testing\"",
        "\"rust-ffi-bridge\"",
        "\"rust-unsafe-hardening\"",
        "\"rust-architecture\"",
        "\"rust-performance\"",
        "\"rust-dual-lang\"",
    ] {
        assert!(stdout.contains(skill), "missing skill {skill}");
    }
    for tool in [
        "\"agents\"",
        "\"antigravity\"",
        "\"agy\"",
        "\"codex\"",
        "\"claude\"",
        "\"cursor\"",
        "\"opencode\"",
        "\"pi\"",
        "\"deepseek-harness\"",
    ] {
        assert!(stdout.contains(tool), "missing tool {tool}");
    }
}

#[test]
fn install_check_uninstall_lifecycle_and_conflict_protection() {
    let tmp = TempDirGuard::new("install-lifecycle");
    let home = tmp.path().join("home");
    let dsh_home = tmp.path().join("custom-dsh");
    let project_dir = tmp.path().join("proj");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&dsh_home).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    let repo = repo_root().to_str().unwrap();
    let home_str = home.to_str().unwrap();
    let dsh_str = dsh_home.to_str().unwrap();
    let proj_str = project_dir.to_str().unwrap();

    // 1. Dry-run must create zero files.
    let dry = run_cli(
        &[
            "install",
            "--repo",
            repo,
            "--home",
            home_str,
            "--tools",
            "antigravity,agy,codex,claude,cursor,opencode,pi,deepseek-harness",
        ],
        &[("DSH_HOME", dsh_str)],
    );
    assert!(dry.status.success());
    assert!(!home.join(".claude/skills/riir").exists());
    assert!(!dsh_home.join("skills/riir").exists());

    // 2. Apply global symlink install.
    let apply = run_cli(
        &[
            "install",
            "--repo",
            repo,
            "--home",
            home_str,
            "--tools",
            "antigravity,agy,codex,claude,cursor,opencode,pi,deepseek-harness",
            "--apply",
        ],
        &[("DSH_HOME", dsh_str)],
    );
    assert!(
        apply.status.success(),
        "install apply failed: {}",
        String::from_utf8_lossy(&apply.stderr)
    );
    assert!(home.join(".gemini/config/skills/riir/SKILL.md").is_file());
    assert!(
        home.join(".gemini/antigravity-cli/skills/riir/SKILL.md")
            .is_file()
    );
    assert!(home.join(".codex/skills/riir/SKILL.md").is_file());
    assert!(home.join(".claude/skills/riir/SKILL.md").is_file());
    assert!(home.join(".cursor/skills/riir/SKILL.md").is_file());
    assert!(home.join(".config/opencode/skills/riir/SKILL.md").is_file());
    assert!(home.join(".pi/agent/skills/riir/SKILL.md").is_file());
    assert!(dsh_home.join("skills/riir/SKILL.md").is_file());

    // 3. Strict check must pass.
    let check_ok = run_cli(
        &[
            "check",
            "--repo",
            repo,
            "--home",
            home_str,
            "--tools",
            "antigravity,agy,codex,claude,cursor,opencode,pi,deepseek-harness",
            "--strict",
        ],
        &[("DSH_HOME", dsh_str)],
    );
    assert!(
        check_ok.status.success(),
        "check --strict failed: {}",
        String::from_utf8_lossy(&check_ok.stdout)
    );

    // 4. Project-scope copy install + conflict protection on unmanaged dir.
    let unmanaged_dir = project_dir.join(".cursor/skills/riir");
    fs::create_dir_all(&unmanaged_dir).unwrap();
    fs::write(unmanaged_dir.join("custom.txt"), "keep-me").unwrap();

    let conflict = run_cli(
        &[
            "install",
            "--repo",
            repo,
            "--home",
            home_str,
            "--scope",
            "project",
            "--project-dir",
            proj_str,
            "--tools",
            "cursor",
            "--skills",
            "riir",
            "--apply",
        ],
        &[],
    );
    assert!(
        !conflict.status.success(),
        "must refuse clobbering unmanaged dir"
    );
    assert_eq!(
        fs::read_to_string(unmanaged_dir.join("custom.txt")).unwrap(),
        "keep-me"
    );

    // 5. Uninstall global owned skills cleanly.
    let uninstall = run_cli(
        &[
            "uninstall",
            "--home",
            home_str,
            "--tools",
            "antigravity,agy,codex,claude,cursor,opencode,pi,deepseek-harness",
            "--apply",
        ],
        &[("DSH_HOME", dsh_str)],
    );
    assert!(uninstall.status.success());
    assert!(!home.join(".claude/skills/riir").exists());
    assert!(!dsh_home.join("skills/riir").exists());
    assert!(!home.join(".local/share/riir-skills/manifest.json").exists());
}

#[test]
fn unsafe_audit_enforces_gates_and_ignores_strings_and_comments() {
    // Self-audit on this repo's src and tests must have zero unsafe and zero static mut.
    let self_audit = run_cli(
        &[
            "unsafe-audit",
            repo_root().join("src").to_str().unwrap(),
            repo_root().join("tests").to_str().unwrap(),
            "--forbid-static-mut",
            "--require-safety-docs",
            "--forbid-file-allows",
            "--max-unsafe-blocks",
            "0",
            "--max-unsafe-fns",
            "0",
        ],
        &[],
    );
    assert!(
        self_audit.status.success(),
        "self unsafe-audit failed: {}",
        String::from_utf8_lossy(&self_audit.stdout)
    );

    // Synthetic fixture with comments, raw strings, documented unsafe, undocumented unsafe, static mut.
    let tmp = TempDirGuard::new("unsafe-fixture");
    let rs_file = tmp.path().join("sample.rs");
    fs::write(
        &rs_file,
        r####"#![allow(unsafe_op_in_unsafe_fn)]
// static mut FAKE = 0; unsafe { }
/* unsafe fn commented_out() {} /* nested unsafe { } */ */
const SAMPLE: &str = "unsafe { static mut X: i32 = 1; }";
const RAW: &str = r###"unsafe fn raw_str() {}"###;
static mut REAL_BAD: u32 = 0;
pub unsafe fn raw_op(p: *mut u32) {
    // SAFETY: caller guarantees valid pointer
    unsafe { *p = 1; }
    unsafe { *p = 2; }
}
"####,
    )
    .unwrap();

    let audit = run_cli(
        &[
            "unsafe-audit",
            rs_file.to_str().unwrap(),
            "--forbid-static-mut",
            "--require-safety-docs",
            "--forbid-file-allows",
            "--json",
        ],
        &[],
    );
    assert!(!audit.status.success(), "fixture gates must fail");
    let out = String::from_utf8_lossy(&audit.stdout);
    assert!(out.contains("\"total_unsafe_blocks\": 2"), "got: {out}");
    assert!(
        out.contains("\"total_documented_unsafe_blocks\": 1"),
        "got: {out}"
    );
    assert!(out.contains("\"total_unsafe_fns\": 1"), "got: {out}");
    assert!(out.contains("\"total_static_muts\": 1"), "got: {out}");
    assert!(out.contains("\"total_file_allows\": 1"), "got: {out}");
}

#[test]
fn diff_run_matches_diverges_and_enforces_timeout() {
    let tmp = TempDirGuard::new("diff-run");
    let cases_ok = tmp.path().join("ok.jsonl");
    fs::write(
        &cases_ok,
        "{\"id\":\"echo_line\",\"args\":[],\"stdin\":\"hello parity\\n\"}\n",
    )
    .unwrap();

    let pass = run_cli(
        &[
            "diff-run",
            "--legacy",
            "cat",
            "--rust",
            "cat",
            "--cases",
            cases_ok.to_str().unwrap(),
        ],
        &[],
    );
    assert!(pass.status.success());

    let mismatch = run_cli(
        &[
            "diff-run",
            "--legacy",
            "cat",
            "--rust",
            "tr a-z A-Z",
            "--cases",
            cases_ok.to_str().unwrap(),
        ],
        &[],
    );
    assert!(!mismatch.status.success());
    assert!(String::from_utf8_lossy(&mismatch.stdout).contains("stdout mismatch"));

    let cases_hang = tmp.path().join("hang.jsonl");
    fs::write(
        &cases_hang,
        "{\"id\":\"hang\",\"args\":[],\"stdin\":\"\"}\n",
    )
    .unwrap();
    let timed_out = run_cli(
        &[
            "diff-run",
            "--legacy",
            "true",
            "--rust",
            "sleep 5",
            "--cases",
            cases_hang.to_str().unwrap(),
            "--timeout-ms",
            "150",
        ],
        &[],
    );
    assert!(!timed_out.status.success());
    assert!(String::from_utf8_lossy(&timed_out.stdout).contains("timeout exceeded"));
}

#[test]
fn init_ledger_dry_run_apply_and_no_overwrite() {
    let tmp = TempDirGuard::new("init-ledger");
    let target = tmp.path().join("ledgers");
    let target_str = target.to_str().unwrap();

    let dry = run_cli(&["init-ledger", target_str, "--mode", "incremental"], &[]);
    assert!(dry.status.success());
    assert!(!target.join("PORTING.md").exists());

    let apply = run_cli(
        &[
            "init-ledger",
            target_str,
            "--mode",
            "incremental",
            "--apply",
        ],
        &[],
    );
    assert!(apply.status.success());
    assert!(target.join("PORTING.md").is_file());
    assert!(target.join("LIFETIMES.tsv").is_file());
    assert!(target.join("PARITY.md").is_file());
    assert!(target.join("UNSAFE-MANIFEST.json").is_file());

    let again = run_cli(
        &[
            "init-ledger",
            target_str,
            "--mode",
            "incremental",
            "--apply",
        ],
        &[],
    );
    assert!(
        !again.status.success(),
        "must not overwrite without --force"
    );
}
