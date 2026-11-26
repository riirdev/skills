use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolSpec {
    pub id: &'static str,
    pub aliases: &'static [&'static str],
    pub display_name: &'static str,
    pub global_rel_skills: &'static str,
    pub project_rel_skills: &'static str,
    pub detection_root: &'static str,
}

pub const ALL_TOOLS: &[ToolSpec] = &[
    ToolSpec {
        id: "agents",
        aliases: &["shared"],
        display_name: "Universal Agents Hub",
        global_rel_skills: ".agents/skills",
        project_rel_skills: ".agents/skills",
        detection_root: ".agents",
    },
    ToolSpec {
        id: "antigravity",
        aliases: &[],
        display_name: "Antigravity IDE",
        global_rel_skills: ".gemini/config/skills",
        project_rel_skills: ".agent/skills",
        detection_root: ".gemini/config",
    },
    ToolSpec {
        id: "agy",
        aliases: &["antigravity-cli"],
        display_name: "Antigravity CLI",
        global_rel_skills: ".gemini/antigravity-cli/skills",
        project_rel_skills: ".agents/skills",
        detection_root: ".gemini/antigravity-cli",
    },
    ToolSpec {
        id: "codex",
        aliases: &["chatgpt"],
        display_name: "Codex / ChatGPT",
        global_rel_skills: ".codex/skills",
        project_rel_skills: ".agents/skills",
        detection_root: ".codex",
    },
    ToolSpec {
        id: "claude",
        aliases: &["claude-code"],
        display_name: "Claude Code",
        global_rel_skills: ".claude/skills",
        project_rel_skills: ".claude/skills",
        detection_root: ".claude",
    },
    ToolSpec {
        id: "cursor",
        aliases: &[],
        display_name: "Cursor",
        global_rel_skills: ".cursor/skills",
        project_rel_skills: ".cursor/skills",
        detection_root: ".cursor",
    },
    ToolSpec {
        id: "opencode",
        aliases: &[],
        display_name: "OpenCode",
        global_rel_skills: ".config/opencode/skills",
        project_rel_skills: ".opencode/skills",
        detection_root: ".config/opencode",
    },
    ToolSpec {
        id: "pi",
        aliases: &[],
        display_name: "Pi Agent",
        global_rel_skills: ".pi/agent/skills",
        project_rel_skills: ".pi/skills",
        detection_root: ".pi",
    },
    ToolSpec {
        id: "deepseek-harness",
        aliases: &["dsh"],
        display_name: "DeepSeek Harness",
        global_rel_skills: ".dsh/skills",
        project_rel_skills: ".dsh/skills",
        detection_root: ".dsh",
    },
    ToolSpec {
        id: "kimi",
        aliases: &[],
        display_name: "Kimi",
        global_rel_skills: ".kimi/skills",
        project_rel_skills: ".agents/skills",
        detection_root: ".kimi",
    },
    ToolSpec {
        id: "kimi-code",
        aliases: &[],
        display_name: "Kimi Code",
        global_rel_skills: ".kimi-code/skills",
        project_rel_skills: ".agents/skills",
        detection_root: ".kimi-code",
    },
    ToolSpec {
        id: "copilot",
        aliases: &["github-copilot"],
        display_name: "Copilot",
        global_rel_skills: ".copilot/skills",
        project_rel_skills: ".github/skills",
        detection_root: ".copilot",
    },
    ToolSpec {
        id: "gemini",
        aliases: &["gemini-cli"],
        display_name: "Gemini CLI",
        global_rel_skills: ".gemini/skills",
        project_rel_skills: ".gemini/skills",
        detection_root: ".gemini",
    },
    ToolSpec {
        id: "commandcode",
        aliases: &["command-code"],
        display_name: "Command Code",
        global_rel_skills: ".commandcode/skills",
        project_rel_skills: ".agents/skills",
        detection_root: ".commandcode",
    },
];

#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub tool_ids: Vec<String>,
    pub skills_dir: PathBuf,
}

pub fn find_tool(token: &str) -> Option<&'static ToolSpec> {
    let norm = token.trim().to_ascii_lowercase();
    ALL_TOOLS
        .iter()
        .find(|spec| spec.id == norm || spec.aliases.contains(&norm.as_str()))
}

pub fn resolve_home(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "HOME environment variable is not set; pass --home <dir>".to_string())
}

fn tool_global_dir(spec: &ToolSpec, home: &Path, dsh_home_override: Option<&Path>) -> PathBuf {
    if spec.id == "deepseek-harness"
        && let Some(dsh_home) = dsh_home_override
    {
        return dsh_home.join("skills");
    }
    home.join(spec.global_rel_skills)
}

fn is_tool_detected(
    spec: &ToolSpec,
    scope: Scope,
    home: &Path,
    project_dir: &Path,
    dsh_home_override: Option<&Path>,
) -> bool {
    match scope {
        Scope::Global => {
            if spec.id == "deepseek-harness"
                && let Some(dsh_home) = dsh_home_override
            {
                return dsh_home.exists();
            }
            home.join(spec.detection_root).exists()
        }
        Scope::Project => {
            let dir = project_dir.join(spec.project_rel_skills);
            dir.parent().is_some_and(|p| p.exists())
        }
    }
}

pub fn resolve_targets(
    selector: &str,
    scope: Scope,
    home: &Path,
    project_dir: &Path,
    dsh_home_env: Option<&Path>,
) -> Result<Vec<ResolvedTarget>, String> {
    let mut selected: Vec<&'static ToolSpec> = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for raw in selector.split(',') {
        let token = raw.trim().to_ascii_lowercase();
        if token.is_empty() {
            continue;
        }
        if token == "all" {
            for spec in ALL_TOOLS {
                if seen_ids.insert(spec.id) {
                    selected.push(spec);
                }
            }
        } else if token == "detected" {
            for spec in ALL_TOOLS {
                if is_tool_detected(spec, scope, home, project_dir, dsh_home_env)
                    && seen_ids.insert(spec.id)
                {
                    selected.push(spec);
                }
            }
            if selected.is_empty()
                && let Some(hub) = find_tool("agents")
                && seen_ids.insert(hub.id)
            {
                selected.push(hub);
            }
        } else {
            let spec = find_tool(&token).ok_or_else(|| {
                format!(
                    "unknown tool target '{token}'; run `riir-skills list` for supported targets"
                )
            })?;
            if seen_ids.insert(spec.id) {
                selected.push(spec);
            }
        }
    }

    if selected.is_empty() {
        return Err("no tool targets selected".to_string());
    }

    let mut by_dir: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for spec in selected {
        let dir = match scope {
            Scope::Global => tool_global_dir(spec, home, dsh_home_env),
            Scope::Project => project_dir.join(spec.project_rel_skills),
        };
        by_dir.entry(dir).or_default().push(spec.id.to_string());
    }

    Ok(by_dir
        .into_iter()
        .map(|(skills_dir, tool_ids)| ResolvedTarget {
            tool_ids,
            skills_dir,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_normalization_and_project_dedup() {
        let home = Path::new("/tmp/nonexistent-home");
        let proj = Path::new("/tmp/nonexistent-proj");
        let targets = resolve_targets(
            "chatgpt,claude-code,dsh,agy,codex",
            Scope::Project,
            home,
            proj,
            None,
        )
        .unwrap();

        let shared = targets
            .iter()
            .find(|t| t.skills_dir == proj.join(".agents/skills"))
            .expect("shared .agents/skills target");
        assert_eq!(shared.tool_ids, vec!["codex", "agy"]);
        assert!(
            targets.iter().any(
                |t| t.tool_ids == vec!["claude"] && t.skills_dir == proj.join(".claude/skills")
            )
        );
        assert!(targets.iter().any(|t| {
            t.tool_ids == vec!["deepseek-harness"] && t.skills_dir == proj.join(".dsh/skills")
        }));
    }
}
