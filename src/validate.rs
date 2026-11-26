use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::json::{self, JsonValue};

#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub skills_checked: usize,
    pub manifests_checked: usize,
    pub errors: Vec<String>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn print_human(&self) {
        println!(
            "Validated {} skills and {} plugin manifests: {} error(s)",
            self.skills_checked,
            self.manifests_checked,
            self.errors.len()
        );
        for err in &self.errors {
            println!("  - {err}");
        }
    }

    pub fn to_json(&self) -> String {
        let mut root = BTreeMap::new();
        root.insert("ok".to_string(), JsonValue::Bool(self.errors.is_empty()));
        root.insert(
            "skills_checked".to_string(),
            JsonValue::Number(self.skills_checked.to_string()),
        );
        root.insert(
            "manifests_checked".to_string(),
            JsonValue::Number(self.manifests_checked.to_string()),
        );
        root.insert(
            "errors".to_string(),
            JsonValue::Array(
                self.errors
                    .iter()
                    .map(|e| JsonValue::String(e.clone()))
                    .collect(),
            ),
        );
        json::to_string_pretty(&JsonValue::Object(root))
    }
}

pub fn validate_repo(repo_root: &Path) -> ValidationReport {
    let mut errors = Vec::new();
    let mut skills_checked = 0usize;
    let mut manifests_checked = 0usize;

    let skills_dir = repo_root.join("skills");
    if !skills_dir.is_dir() {
        errors.push("missing skills/ directory at repository root".to_string());
    } else {
        let mut entries: Vec<PathBuf> = match fs::read_dir(&skills_dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect(),
            Err(e) => {
                errors.push(format!("failed to read skills/: {e}"));
                Vec::new()
            }
        };
        entries.sort();

        if entries.is_empty() {
            errors.push("skills/ contains no skill directories".to_string());
        }

        let mut has_riir = false;
        for dir in entries {
            let dir_name = dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            if dir_name == "riir" {
                has_riir = true;
            }
            skills_checked += 1;
            validate_skill_dir(&dir, &dir_name, &mut errors);
        }
        if !has_riir {
            errors.push("required flagship skill skills/riir/SKILL.md is missing".to_string());
        }
    }

    for rel in [
        "plugin.json",
        ".claude-plugin/plugin.json",
        ".cursor-plugin/plugin.json",
    ] {
        manifests_checked += 1;
        let path = repo_root.join(rel);
        if !path.is_file() {
            errors.push(format!("missing plugin manifest {rel}"));
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(raw) => match json::parse(&raw) {
                Ok(JsonValue::Object(map)) => {
                    if !map
                        .get("name")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.trim().is_empty())
                    {
                        errors.push(format!("{rel}: missing non-empty 'name' field"));
                    }
                    if !map
                        .get("description")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.trim().is_empty())
                    {
                        errors.push(format!("{rel}: missing non-empty 'description' field"));
                    }
                }
                Ok(_) => errors.push(format!("{rel}: root JSON value must be an object")),
                Err(e) => errors.push(format!("{rel}: invalid JSON: {e}")),
            },
            Err(e) => errors.push(format!("failed to read {rel}: {e}")),
        }
    }

    ValidationReport {
        skills_checked,
        manifests_checked,
        errors,
    }
}

pub fn parse_skill_meta(skill_dir: &Path) -> Result<SkillMeta, String> {
    let dir_name = skill_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "invalid skill directory name".to_string())?
        .to_string();
    let skill_md = skill_dir.join("SKILL.md");
    let content = fs::read_to_string(&skill_md)
        .map_err(|e| format!("failed to read {}: {e}", skill_md.display()))?;
    let (fm, _) = split_frontmatter(&content)?;
    let name = fm
        .get("name")
        .cloned()
        .ok_or_else(|| format!("{}: missing 'name' in frontmatter", skill_md.display()))?;
    let description = fm.get("description").cloned().ok_or_else(|| {
        format!(
            "{}: missing 'description' in frontmatter",
            skill_md.display()
        )
    })?;
    validate_name_rules(&name, &dir_name)?;
    validate_description_rules(&description)?;
    Ok(SkillMeta {
        name,
        description,
        dir: skill_dir.to_path_buf(),
    })
}

fn validate_skill_dir(skill_dir: &Path, dir_name: &str, errors: &mut Vec<String>) {
    let skill_md = skill_dir.join("SKILL.md");
    if !skill_md.is_file() {
        errors.push(format!("skills/{dir_name}: missing SKILL.md"));
        return;
    }

    let content = match fs::read_to_string(&skill_md) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("skills/{dir_name}/SKILL.md: read error: {e}"));
            return;
        }
    };

    let (fm, body) = match split_frontmatter(&content) {
        Ok(parts) => parts,
        Err(e) => {
            errors.push(format!("skills/{dir_name}/SKILL.md: {e}"));
            return;
        }
    };

    match fm.get("name") {
        Some(name) => {
            if let Err(e) = validate_name_rules(name, dir_name) {
                errors.push(format!("skills/{dir_name}/SKILL.md: {e}"));
            }
        }
        None => errors.push(format!("skills/{dir_name}/SKILL.md: missing 'name' field")),
    }

    match fm.get("description") {
        Some(desc) => {
            if let Err(e) = validate_description_rules(desc) {
                errors.push(format!("skills/{dir_name}/SKILL.md: {e}"));
            }
        }
        None => errors.push(format!(
            "skills/{dir_name}/SKILL.md: missing 'description' field"
        )),
    }

    if body.trim().is_empty() {
        errors.push(format!("skills/{dir_name}/SKILL.md: empty markdown body"));
    }

    check_markdown_links(&skill_md, skill_dir, &content, errors);

    let refs_dir = skill_dir.join("references");
    if refs_dir.is_dir()
        && let Ok(rd) = fs::read_dir(&refs_dir)
    {
        let mut ref_files: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
            .collect();
        ref_files.sort();
        for ref_file in ref_files {
            if let Ok(ref_text) = fs::read_to_string(&ref_file) {
                check_markdown_links(&ref_file, skill_dir, &ref_text, errors);
            }
        }
    }
}

fn split_frontmatter(content: &str) -> Result<(BTreeMap<String, String>, &str), String> {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    let rest = if let Some(r) = normalized.strip_prefix("---\n") {
        r
    } else if let Some(r) = normalized.strip_prefix("---\r\n") {
        r
    } else {
        return Err("missing opening YAML frontmatter '---'".to_string());
    };

    let mut end_idx = None;
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            end_idx = Some((offset, offset + line.len()));
            break;
        }
        offset += line.len();
    }
    if end_idx.is_none() && rest.trim_end_matches(['\r', '\n']) == "---" {
        end_idx = Some((rest.len(), rest.len()));
    }

    let (fm_end, body_start) =
        end_idx.ok_or_else(|| "missing closing YAML frontmatter '---'".to_string())?;
    let fm_text = &rest[..fm_end];
    let body = &rest[body_start..];

    let mut map = BTreeMap::new();
    for line in fm_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_string();
            let val = unquote_yaml_scalar(v.trim());
            map.insert(key, val);
        }
    }
    Ok((map, body))
}

fn unquote_yaml_scalar(val: &str) -> String {
    if val.len() >= 2
        && ((val.starts_with('"') && val.ends_with('"'))
            || (val.starts_with('\'') && val.ends_with('\'')))
    {
        val[1..val.len() - 1].to_string()
    } else {
        val.to_string()
    }
}

fn validate_name_rules(name: &str, dir_name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err(format!("invalid skill name length ({})", name.len()));
    }
    if !name
        .bytes()
        .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-'))
    {
        return Err(format!(
            "skill name '{name}' must contain only lowercase [a-z0-9-]"
        ));
    }
    if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
        return Err(format!("skill name '{name}' has invalid hyphen placement"));
    }
    if name != dir_name {
        return Err(format!(
            "skill name '{name}' does not match directory name '{dir_name}'"
        ));
    }
    Ok(())
}

fn validate_description_rules(desc: &str) -> Result<(), String> {
    let trimmed = desc.trim();
    if trimmed.is_empty() {
        return Err("description must not be empty".to_string());
    }
    if trimmed.chars().count() > 1024 {
        return Err("description exceeds 1024 characters".to_string());
    }
    Ok(())
}

fn check_markdown_links(md_path: &Path, skill_dir: &Path, content: &str, errors: &mut Vec<String>) {
    let mut in_fence = false;
    let parent = md_path.parent().unwrap_or(skill_dir);
    let skill_canon = fs::canonicalize(skill_dir).unwrap_or_else(|_| skill_dir.to_path_buf());

    for (line_idx, raw_line) in content.lines().enumerate() {
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        let stripped = strip_inline_code(raw_line);
        let bytes = stripped.as_bytes();
        let mut pos = 0usize;
        while pos < bytes.len() {
            if let Some(open_rel) = stripped[pos..].find("](") {
                let link_start = pos + open_rel + 2;
                if let Some(close_rel) = stripped[link_start..].find(')') {
                    let target = stripped[link_start..link_start + close_rel].trim();
                    pos = link_start + close_rel + 1;
                    if target.is_empty()
                        || target.starts_with("http://")
                        || target.starts_with("https://")
                        || target.starts_with("mailto:")
                        || target.starts_with('#')
                    {
                        continue;
                    }
                    let file_part = target.split('#').next().unwrap_or("").trim();
                    if file_part.is_empty() {
                        continue;
                    }
                    if Path::new(file_part).is_absolute() {
                        errors.push(format!(
                            "{}:{}: absolute link target '{}' is forbidden",
                            md_path.display(),
                            line_idx + 1,
                            target
                        ));
                        continue;
                    }
                    let joined = normalize_rel_join(parent, Path::new(file_part));
                    match fs::canonicalize(&joined) {
                        Ok(canon) => {
                            let outside_skill = !canon.starts_with(&skill_canon);
                            let is_cross_skill = file_part.starts_with("../")
                                && canon
                                    .parent()
                                    .and_then(|_| skill_canon.parent())
                                    .is_some_and(|skills_root| canon.starts_with(skills_root));
                            if outside_skill && !is_cross_skill {
                                errors.push(format!(
                                    "{}:{}: link '{}' escapes skills directory",
                                    md_path.display(),
                                    line_idx + 1,
                                    target
                                ));
                            }
                        }
                        Err(_) => {
                            errors.push(format!(
                                "{}:{}: broken relative link '{}'",
                                md_path.display(),
                                line_idx + 1,
                                target
                            ));
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
}

fn strip_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_tick = false;
    for ch in line.chars() {
        if ch == '`' {
            in_tick = !in_tick;
            out.push(' ');
        } else if in_tick {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

fn normalize_rel_join(base: &Path, rel: &Path) -> PathBuf {
    let mut out = base.to_path_buf();
    for comp in rel.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    out
}
