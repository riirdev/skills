use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::json::{self, JsonValue};

#[derive(Debug, Clone)]
pub struct DiffConfig {
    pub legacy_cmd: String,
    pub rust_cmd: String,
    pub cases_path: PathBuf,
    pub timeout_ms: u64,
    pub ignore_stderr: bool,
}

#[derive(Debug, Clone)]
struct DiffCase {
    id: String,
    args: Vec<String>,
    stdin: String,
    env: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CaseOutcome {
    pub id: String,
    pub passed: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct DiffReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub outcomes: Vec<CaseOutcome>,
}

impl DiffReport {
    pub fn all_passed(&self) -> bool {
        self.total > 0 && self.failed == 0
    }

    pub fn print_human(&self) {
        println!(
            "Differential Parity: {}/{} passed ({} failed)",
            self.passed, self.total, self.failed
        );
        for outcome in &self.outcomes {
            if !outcome.passed {
                println!("  - [FAIL] {}: {}", outcome.id, outcome.reason);
            }
        }
    }

    pub fn to_json(&self) -> String {
        let mut root = BTreeMap::new();
        root.insert("ok".to_string(), JsonValue::Bool(self.all_passed()));
        root.insert(
            "total".to_string(),
            JsonValue::Number(self.total.to_string()),
        );
        root.insert(
            "passed".to_string(),
            JsonValue::Number(self.passed.to_string()),
        );
        root.insert(
            "failed".to_string(),
            JsonValue::Number(self.failed.to_string()),
        );
        let arr = self
            .outcomes
            .iter()
            .map(|o| {
                let mut m = BTreeMap::new();
                m.insert("id".to_string(), JsonValue::String(o.id.clone()));
                m.insert("passed".to_string(), JsonValue::Bool(o.passed));
                m.insert("reason".to_string(), JsonValue::String(o.reason.clone()));
                JsonValue::Object(m)
            })
            .collect();
        root.insert("outcomes".to_string(), JsonValue::Array(arr));
        json::to_string_pretty(&JsonValue::Object(root))
    }
}

#[derive(Debug, Clone)]
struct ExecResult {
    exit_code: Option<i32>,
    timed_out: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

pub fn run(cfg: &DiffConfig) -> Result<DiffReport, String> {
    let raw = fs::read_to_string(&cfg.cases_path)
        .map_err(|e| format!("failed to read {}: {e}", cfg.cases_path.display()))?;
    let cases = parse_cases(&raw)?;
    if cases.is_empty() {
        return Err("cases file contains zero test cases".to_string());
    }

    let timeout = Duration::from_millis(cfg.timeout_ms);
    let mut outcomes = Vec::with_capacity(cases.len());
    let mut passed = 0usize;
    let mut failed = 0usize;

    for case in &cases {
        let legacy_res = exec_with_timeout(&cfg.legacy_cmd, case, timeout)?;
        let rust_res = exec_with_timeout(&cfg.rust_cmd, case, timeout)?;

        if legacy_res.timed_out || rust_res.timed_out {
            failed += 1;
            outcomes.push(CaseOutcome {
                id: case.id.clone(),
                passed: false,
                reason: format!(
                    "timeout exceeded ({} ms): legacy_timeout={}, rust_timeout={}",
                    cfg.timeout_ms, legacy_res.timed_out, rust_res.timed_out
                ),
            });
            continue;
        }

        if legacy_res.exit_code != rust_res.exit_code {
            failed += 1;
            outcomes.push(CaseOutcome {
                id: case.id.clone(),
                passed: false,
                reason: format!(
                    "exit code mismatch: legacy={:?}, rust={:?}",
                    legacy_res.exit_code, rust_res.exit_code
                ),
            });
            continue;
        }

        if legacy_res.stdout != rust_res.stdout {
            failed += 1;
            outcomes.push(CaseOutcome {
                id: case.id.clone(),
                passed: false,
                reason: format!(
                    "stdout mismatch: legacy='{}' vs rust='{}'",
                    preview_bytes(&legacy_res.stdout),
                    preview_bytes(&rust_res.stdout)
                ),
            });
            continue;
        }

        if !cfg.ignore_stderr && legacy_res.stderr != rust_res.stderr {
            failed += 1;
            outcomes.push(CaseOutcome {
                id: case.id.clone(),
                passed: false,
                reason: format!(
                    "stderr mismatch: legacy='{}' vs rust='{}'",
                    preview_bytes(&legacy_res.stderr),
                    preview_bytes(&rust_res.stderr)
                ),
            });
            continue;
        }

        passed += 1;
        outcomes.push(CaseOutcome {
            id: case.id.clone(),
            passed: true,
            reason: "exact match".to_string(),
        });
    }

    Ok(DiffReport {
        total: cases.len(),
        passed,
        failed,
        outcomes,
    })
}

fn parse_cases(raw: &str) -> Result<Vec<DiffCase>, String> {
    let mut cases = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let val =
            json::parse(trimmed).map_err(|e| format!("invalid JSON on line {}: {e}", idx + 1))?;
        let obj = val
            .as_object()
            .ok_or_else(|| format!("line {} must be a JSON object", idx + 1))?;

        let id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("case_{}", idx + 1));

        let mut args = Vec::new();
        if let Some(arr) = obj.get("args").and_then(|v| v.as_array()) {
            for item in arr {
                let s = item
                    .as_str()
                    .ok_or_else(|| format!("case '{id}': args must contain strings"))?;
                args.push(s.to_string());
            }
        }

        let stdin = obj
            .get("stdin")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut env_map = BTreeMap::new();
        if let Some(env_obj) = obj.get("env").and_then(|v| v.as_object()) {
            for (k, v) in env_obj {
                let val_str = v
                    .as_str()
                    .ok_or_else(|| format!("case '{id}': env values must be strings"))?;
                env_map.insert(k.clone(), val_str.to_string());
            }
        }

        cases.push(DiffCase {
            id,
            args,
            stdin,
            env: env_map,
        });
    }
    Ok(cases)
}

fn exec_with_timeout(
    cmd_prefix: &str,
    case: &DiffCase,
    timeout: Duration,
) -> Result<ExecResult, String> {
    let mut cmd = build_command(cmd_prefix, &case.args);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &case.env {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn '{cmd_prefix}': {e}"))?;

    if let Some(mut stdin_pipe) = child.stdin.take() {
        let input = case.stdin.as_bytes().to_vec();
        thread::spawn(move || {
            let _ = stdin_pipe.write_all(&input);
        });
    }

    let stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| "missing stdout pipe".to_string())?;
    let stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| "missing stderr pipe".to_string())?;

    let out_handle = thread::spawn(move || read_all(stdout_pipe));
    let err_handle = thread::spawn(move || read_all(stderr_pipe));

    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    timed_out = true;
                    let _ = child.kill();
                    break child.wait().ok();
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(format!("wait error: {e}")),
        }
    };

    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();

    Ok(ExecResult {
        exit_code: status.and_then(|s| s.code()),
        timed_out,
        stdout,
        stderr,
    })
}

fn read_all<R: Read>(mut reader: R) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = reader.read_to_end(&mut buf);
    buf
}

#[cfg(unix)]
fn build_command(cmd_prefix: &str, args: &[String]) -> Command {
    let mut cmd = Command::new("sh");
    let script = format!("{cmd_prefix} \"$@\"");
    cmd.arg("-c").arg(script).arg("riir-diff");
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

#[cfg(not(unix))]
fn build_command(cmd_prefix: &str, args: &[String]) -> Command {
    let mut parts = cmd_prefix.split_whitespace();
    let prog = parts.next().unwrap_or(cmd_prefix);
    let mut cmd = Command::new(prog);
    for p in parts {
        cmd.arg(p);
    }
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

fn preview_bytes(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let single = text.replace('\n', "\\n").replace('\r', "\\r");
    if single.chars().count() > 120 {
        let truncated: String = single.chars().take(120).collect();
        format!("{truncated}...")
    } else {
        single
    }
}
