use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::json::{self, JsonValue};
use crate::sha256;
use crate::targets::{self, Scope};
use crate::validate::{self, SkillMeta};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMode {
    Symlink,
    Copy,
}

impl InstallMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Copy => "copy",
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallRequest {
    pub repo: Option<PathBuf>,
    pub tools: String,
    pub skills: String,
    pub scope: Scope,
    pub project_dir: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub mode: InstallMode,
    pub from_source: bool,
    pub force: bool,
    pub apply: bool,
}

#[derive(Debug, Clone)]
pub struct UninstallRequest {
    pub tools: String,
    pub skills: String,
    pub scope: Scope,
    pub project_dir: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub apply: bool,
}

#[derive(Debug, Clone)]
pub struct PlannedEntry {
    pub tools: Vec<String>,
    pub skill: String,
    pub target_path: PathBuf,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct InstallReport {
    pub apply: bool,
    pub mode: InstallMode,
    pub canonical_store: Option<PathBuf>,
    pub manifest_path: PathBuf,
    pub entries: Vec<PlannedEntry>,
}

impl InstallReport {
    pub fn has_conflicts(&self) -> bool {
        self.entries.iter().any(|e| e.action == "conflict")
    }

    pub fn print_human(&self) {
        let mode_label = if self.apply { "applied" } else { "dry-run" };
        println!(
            "Install ({mode_label}, mode={}): {} action(s)",
            self.mode.as_str(),
            self.entries.len()
        );
        if let Some(store) = &self.canonical_store {
            println!("  canonical_store: {}", store.display());
        }
        println!("  manifest: {}", self.manifest_path.display());
        for entry in &self.entries {
            println!(
                "  - [{}] {} ({}) -> {} ({})",
                entry.action,
                entry.skill,
                entry.tools.join(","),
                entry.target_path.display(),
                entry.detail
            );
        }
    }

    pub fn to_json(&self) -> String {
        let mut root = BTreeMap::new();
        root.insert("apply".to_string(), JsonValue::Bool(self.apply));
        root.insert(
            "mode".to_string(),
            JsonValue::String(self.mode.as_str().to_string()),
        );
        root.insert(
            "has_conflicts".to_string(),
            JsonValue::Bool(self.has_conflicts()),
        );
        root.insert(
            "manifest_path".to_string(),
            JsonValue::String(self.manifest_path.display().to_string()),
        );
        let arr = self
            .entries
            .iter()
            .map(|e| {
                let mut m = BTreeMap::new();
                m.insert(
                    "tools".to_string(),
                    JsonValue::Array(
                        e.tools
                            .iter()
                            .map(|t| JsonValue::String(t.clone()))
                            .collect(),
                    ),
                );
                m.insert("skill".to_string(), JsonValue::String(e.skill.clone()));
                m.insert(
                    "target_path".to_string(),
                    JsonValue::String(e.target_path.display().to_string()),
                );
                m.insert("action".to_string(), JsonValue::String(e.action.clone()));
                m.insert("detail".to_string(), JsonValue::String(e.detail.clone()));
                JsonValue::Object(m)
            })
            .collect();
        root.insert("entries".to_string(), JsonValue::Array(arr));
        json::to_string_pretty(&JsonValue::Object(root))
    }
}

#[derive(Debug, Clone)]
pub struct CheckEntry {
    pub tools: Vec<String>,
    pub skill: String,
    pub target_path: PathBuf,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct CheckReport {
    pub manifest_path: PathBuf,
    pub entries: Vec<CheckEntry>,
}

impl CheckReport {
    pub fn all_healthy(&self) -> bool {
        self.entries.iter().all(|e| e.status == "installed")
    }

    pub fn print_human(&self) {
        println!(
            "Check ({}): {} target-skill pair(s)",
            self.manifest_path.display(),
            self.entries.len()
        );
        for entry in &self.entries {
            println!(
                "  - [{}] {} ({}) at {} ({})",
                entry.status,
                entry.skill,
                entry.tools.join(","),
                entry.target_path.display(),
                entry.detail
            );
        }
    }

    pub fn to_json(&self) -> String {
        let mut root = BTreeMap::new();
        root.insert("healthy".to_string(), JsonValue::Bool(self.all_healthy()));
        root.insert(
            "manifest_path".to_string(),
            JsonValue::String(self.manifest_path.display().to_string()),
        );
        let arr = self
            .entries
            .iter()
            .map(|e| {
                let mut m = BTreeMap::new();
                m.insert(
                    "tools".to_string(),
                    JsonValue::Array(
                        e.tools
                            .iter()
                            .map(|t| JsonValue::String(t.clone()))
                            .collect(),
                    ),
                );
                m.insert("skill".to_string(), JsonValue::String(e.skill.clone()));
                m.insert(
                    "target_path".to_string(),
                    JsonValue::String(e.target_path.display().to_string()),
                );
                m.insert("status".to_string(), JsonValue::String(e.status.clone()));
                m.insert("detail".to_string(), JsonValue::String(e.detail.clone()));
                JsonValue::Object(m)
            })
            .collect();
        root.insert("entries".to_string(), JsonValue::Array(arr));
        json::to_string_pretty(&JsonValue::Object(root))
    }
}

#[derive(Debug, Clone)]
pub struct UninstallReport {
    pub apply: bool,
    pub manifest_path: PathBuf,
    pub entries: Vec<PlannedEntry>,
}

impl UninstallReport {
    pub fn print_human(&self) {
        let label = if self.apply { "applied" } else { "dry-run" };
        println!("Uninstall ({label}): {} entry(ies)", self.entries.len());
        for entry in &self.entries {
            println!(
                "  - [{}] {} ({}) at {} ({})",
                entry.action,
                entry.skill,
                entry.tools.join(","),
                entry.target_path.display(),
                entry.detail
            );
        }
    }

    pub fn to_json(&self) -> String {
        let mut root = BTreeMap::new();
        root.insert("apply".to_string(), JsonValue::Bool(self.apply));
        root.insert(
            "manifest_path".to_string(),
            JsonValue::String(self.manifest_path.display().to_string()),
        );
        let arr = self
            .entries
            .iter()
            .map(|e| {
                let mut m = BTreeMap::new();
                m.insert(
                    "tools".to_string(),
                    JsonValue::Array(
                        e.tools
                            .iter()
                            .map(|t| JsonValue::String(t.clone()))
                            .collect(),
                    ),
                );
                m.insert("skill".to_string(), JsonValue::String(e.skill.clone()));
                m.insert(
                    "target_path".to_string(),
                    JsonValue::String(e.target_path.display().to_string()),
                );
                m.insert("action".to_string(), JsonValue::String(e.action.clone()));
                m.insert("detail".to_string(), JsonValue::String(e.detail.clone()));
                JsonValue::Object(m)
            })
            .collect();
        root.insert("entries".to_string(), JsonValue::Array(arr));
        json::to_string_pretty(&JsonValue::Object(root))
    }
}

#[derive(Debug, Clone)]
struct ManifestEntry {
    kind: String,
    source_or_target: String,
    files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct Manifest {
    entries: BTreeMap<String, ManifestEntry>,
}

impl Manifest {
    fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .map_err(|e| format!("failed to read manifest {}: {e}", path.display()))?;
        let val = json::parse(&raw)
            .map_err(|e| format!("invalid manifest JSON {}: {e}", path.display()))?;
        let obj = val
            .as_object()
            .ok_or_else(|| "manifest root must be a JSON object".to_string())?;
        let mut entries = BTreeMap::new();
        if let Some(JsonValue::Object(map)) = obj.get("entries") {
            for (k, v) in map {
                let item = v
                    .as_object()
                    .ok_or_else(|| "manifest entry must be an object".to_string())?;
                let kind = item
                    .get("kind")
                    .and_then(|x| x.as_str())
                    .unwrap_or("copy")
                    .to_string();
                let source_or_target = item
                    .get("target")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut files = BTreeMap::new();
                if let Some(JsonValue::Object(fmap)) = item.get("files") {
                    for (rel, hash_val) in fmap {
                        if let Some(h) = hash_val.as_str() {
                            files.insert(rel.clone(), h.to_string());
                        }
                    }
                }
                entries.insert(
                    k.clone(),
                    ManifestEntry {
                        kind,
                        source_or_target,
                        files,
                    },
                );
            }
        }
        Ok(Self { entries })
    }

    fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
        let mut root = BTreeMap::new();
        let mut entries_obj = BTreeMap::new();
        for (k, entry) in &self.entries {
            let mut item = BTreeMap::new();
            item.insert("kind".to_string(), JsonValue::String(entry.kind.clone()));
            if !entry.source_or_target.is_empty() {
                item.insert(
                    "target".to_string(),
                    JsonValue::String(entry.source_or_target.clone()),
                );
            }
            if !entry.files.is_empty() {
                let mut fmap = BTreeMap::new();
                for (rel, h) in &entry.files {
                    fmap.insert(rel.clone(), JsonValue::String(h.clone()));
                }
                item.insert("files".to_string(), JsonValue::Object(fmap));
            }
            entries_obj.insert(k.clone(), JsonValue::Object(item));
        }
        root.insert("entries".to_string(), JsonValue::Object(entries_obj));
        fs::write(path, json::to_string_pretty(&JsonValue::Object(root)))
            .map_err(|e| format!("failed to write manifest {}: {e}", path.display()))
    }
}

pub fn discover_repo_root(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        if p.join("skills").is_dir() {
            return Ok(p.to_path_buf());
        }
        return Err(format!(
            "repository path {} does not contain a skills/ directory",
            p.display()
        ));
    }
    let mut cur = env::current_dir().map_err(|e| format!("failed to read cwd: {e}"))?;
    loop {
        if cur.join("skills").join("riir").join("SKILL.md").is_file() {
            return Ok(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    Err("could not locate skills/riir/SKILL.md; pass --repo <path>".to_string())
}

pub fn discover_skills(repo_root: &Path) -> Result<Vec<SkillMeta>, String> {
    let skills_root = repo_root.join("skills");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&skills_root)
        .map_err(|e| format!("failed to read {}: {e}", skills_root.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    for d in dirs {
        out.push(validate::parse_skill_meta(&d)?);
    }
    if out.is_empty() {
        return Err(format!("no skills found under {}", skills_root.display()));
    }
    Ok(out)
}

pub fn format_list_json(repo_root: &Path, skills: &[SkillMeta]) -> String {
    let mut root = BTreeMap::new();
    root.insert(
        "repo_root".to_string(),
        JsonValue::String(repo_root.display().to_string()),
    );
    let skill_arr = skills
        .iter()
        .map(|s| {
            let mut m = BTreeMap::new();
            m.insert("name".to_string(), JsonValue::String(s.name.clone()));
            m.insert(
                "description".to_string(),
                JsonValue::String(s.description.clone()),
            );
            JsonValue::Object(m)
        })
        .collect();
    root.insert("skills".to_string(), JsonValue::Array(skill_arr));

    let tool_arr = targets::ALL_TOOLS
        .iter()
        .map(|t| {
            let mut m = BTreeMap::new();
            m.insert("id".to_string(), JsonValue::String(t.id.to_string()));
            m.insert(
                "display_name".to_string(),
                JsonValue::String(t.display_name.to_string()),
            );
            m.insert(
                "global_rel_skills".to_string(),
                JsonValue::String(t.global_rel_skills.to_string()),
            );
            m.insert(
                "project_rel_skills".to_string(),
                JsonValue::String(t.project_rel_skills.to_string()),
            );
            JsonValue::Object(m)
        })
        .collect();
    root.insert("tools".to_string(), JsonValue::Array(tool_arr));
    json::to_string_pretty(&JsonValue::Object(root))
}

fn select_skills<'a>(all: &'a [SkillMeta], selector: &str) -> Result<Vec<&'a SkillMeta>, String> {
    if selector.trim() == "all" {
        return Ok(all.iter().collect());
    }
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for raw in selector.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        validate_safe_skill_name(token)?;
        let found = all
            .iter()
            .find(|s| s.name == token)
            .ok_or_else(|| format!("unknown skill '{token}'"))?;
        if seen.insert(found.name.clone()) {
            out.push(found);
        }
    }
    if out.is_empty() {
        return Err("no skills selected".to_string());
    }
    Ok(out)
}

fn validate_safe_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.contains('\0')
    {
        return Err(format!("unsafe skill name '{name}'"));
    }
    Ok(())
}

fn dsh_home_from_env() -> Option<PathBuf> {
    env::var_os("DSH_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn resolve_project_dir(explicit: Option<&Path>) -> Result<PathBuf, String> {
    match explicit {
        Some(p) => Ok(p.to_path_buf()),
        None => env::current_dir().map_err(|e| format!("failed to read cwd: {e}")),
    }
}

fn manifest_location(scope: Scope, home: &Path, project_dir: &Path) -> PathBuf {
    match scope {
        Scope::Global => home
            .join(".local")
            .join("share")
            .join("riir-skills")
            .join("manifest.json"),
        Scope::Project => project_dir.join(".riir-skills-manifest.json"),
    }
}

fn canonical_store_root(home: &Path) -> PathBuf {
    home.join(".local")
        .join("share")
        .join("riir-skills")
        .join("skills")
}

pub fn run_install(req: &InstallRequest) -> Result<InstallReport, String> {
    let repo_root = discover_repo_root(req.repo.as_deref())?;
    let all_skills = discover_skills(&repo_root)?;
    let selected_skills = select_skills(&all_skills, &req.skills)?;
    let home = targets::resolve_home(req.home.as_deref())?;
    let project_dir = resolve_project_dir(req.project_dir.as_deref())?;
    let dsh_home = dsh_home_from_env();
    let targets = targets::resolve_targets(
        &req.tools,
        req.scope,
        &home,
        &project_dir,
        dsh_home.as_deref(),
    )?;

    let manifest_path = manifest_location(req.scope, &home, &project_dir);
    let mut manifest = Manifest::load(&manifest_path)?;

    let use_canonical_store = !req.from_source && req.scope == Scope::Global;
    let store_root = if use_canonical_store {
        Some(canonical_store_root(&home))
    } else {
        None
    };

    let mut skill_sources: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut skill_hashes: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for s in &selected_skills {
        let hashes = collect_dir_hashes(&s.dir)?;
        skill_hashes.insert(s.name.clone(), hashes);
        if let Some(store) = &store_root {
            skill_sources.insert(s.name.clone(), store.join(&s.name));
        } else {
            skill_sources.insert(s.name.clone(), s.dir.clone());
        }
    }

    let mut entries = Vec::new();
    let mut pending_ops: Vec<(String, PathBuf, PathBuf, &SkillMeta)> = Vec::new();

    for target in &targets {
        for skill in &selected_skills {
            let dst = target.skills_dir.join(&skill.name);
            let link_src = &skill_sources[&skill.name];
            let expected_hashes = &skill_hashes[&skill.name];
            let key = dst.display().to_string();
            let owned = manifest.entries.get(&key);

            let (action, detail) = classify_install_target(
                &dst,
                req.mode,
                link_src,
                expected_hashes,
                owned,
                req.force,
            )?;
            entries.push(PlannedEntry {
                tools: target.tool_ids.clone(),
                skill: skill.name.clone(),
                target_path: dst.clone(),
                action: action.clone(),
                detail,
            });
            if matches!(action.as_str(), "install" | "update") {
                pending_ops.push((action, dst, link_src.clone(), *skill));
            }
        }
    }

    let report = InstallReport {
        apply: req.apply,
        mode: req.mode,
        canonical_store: store_root.clone(),
        manifest_path: manifest_path.clone(),
        entries,
    };

    if !req.apply || report.has_conflicts() {
        return Ok(report);
    }

    if let Some(store) = &store_root {
        for skill in &selected_skills {
            let store_skill_dir = store.join(&skill.name);
            sync_copy_dir(&skill.dir, &store_skill_dir)?;
        }
    }

    for (_action, dst, link_src, skill) in pending_ops {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
        remove_existing_path(&dst)?;
        match req.mode {
            InstallMode::Symlink => {
                create_symlink(&link_src, &dst)?;
                manifest.entries.insert(
                    dst.display().to_string(),
                    ManifestEntry {
                        kind: "symlink".to_string(),
                        source_or_target: link_src.display().to_string(),
                        files: BTreeMap::new(),
                    },
                );
            }
            InstallMode::Copy => {
                sync_copy_dir(&skill.dir, &dst)?;
                let hashes = skill_hashes[&skill.name].clone();
                manifest.entries.insert(
                    dst.display().to_string(),
                    ManifestEntry {
                        kind: "copy".to_string(),
                        source_or_target: String::new(),
                        files: hashes,
                    },
                );
            }
        }
    }

    manifest.save(&manifest_path)?;
    Ok(report)
}

pub fn run_check(
    repo: Option<&Path>,
    tools_sel: &str,
    skills_sel: &str,
    scope: Scope,
    project_dir: Option<&Path>,
    home: Option<&Path>,
) -> Result<CheckReport, String> {
    let repo_root = discover_repo_root(repo)?;
    let all_skills = discover_skills(&repo_root)?;
    let selected_skills = select_skills(&all_skills, skills_sel)?;
    let home_dir = targets::resolve_home(home)?;
    let proj_dir = resolve_project_dir(project_dir)?;
    let dsh_home = dsh_home_from_env();
    let targets =
        targets::resolve_targets(tools_sel, scope, &home_dir, &proj_dir, dsh_home.as_deref())?;
    let manifest_path = manifest_location(scope, &home_dir, &proj_dir);
    let manifest = Manifest::load(&manifest_path)?;

    let mut source_hashes = BTreeMap::new();
    for s in &selected_skills {
        source_hashes.insert(s.name.clone(), collect_dir_hashes(&s.dir)?);
    }

    let mut entries = Vec::new();
    for target in &targets {
        for skill in &selected_skills {
            let dst = target.skills_dir.join(&skill.name);
            let key = dst.display().to_string();
            let owned = manifest.entries.get(&key);
            let expected = &source_hashes[&skill.name];

            let (status, detail) = match fs::symlink_metadata(&dst) {
                Err(_) => ("missing".to_string(), "not installed".to_string()),
                Ok(meta) if meta.file_type().is_symlink() => {
                    let link_target = fs::read_link(&dst)
                        .map_err(|e| format!("failed to read link {}: {e}", dst.display()))?;
                    let resolved = resolve_link_target(&dst, &link_target);
                    if !resolved.exists() {
                        (
                            "broken_link".to_string(),
                            format!("symlink target {} does not exist", link_target.display()),
                        )
                    } else if let Some(m) = owned {
                        if m.kind == "symlink" && Path::new(&m.source_or_target) == resolved {
                            match collect_dir_hashes(&resolved) {
                                Ok(actual) if actual == *expected => (
                                    "installed".to_string(),
                                    format!("symlink -> {}", resolved.display()),
                                ),
                                Ok(_) => (
                                    "drifted".to_string(),
                                    "canonical store differs from source".to_string(),
                                ),
                                Err(e) => ("broken_link".to_string(), e),
                            }
                        } else {
                            (
                                "unmanaged_conflict".to_string(),
                                format!("symlink points to {}", resolved.display()),
                            )
                        }
                    } else {
                        (
                            "unmanaged_conflict".to_string(),
                            format!("unmanaged symlink -> {}", resolved.display()),
                        )
                    }
                }
                Ok(meta) if meta.is_dir() => {
                    if let Some(m) = owned {
                        if m.kind == "copy" {
                            let actual = collect_dir_hashes(&dst)?;
                            if actual != m.files {
                                (
                                    "drifted".to_string(),
                                    "installed copy modified on disk".to_string(),
                                )
                            } else if actual != *expected {
                                (
                                    "drifted".to_string(),
                                    "installed copy is older than source".to_string(),
                                )
                            } else {
                                ("installed".to_string(), "verified copy".to_string())
                            }
                        } else {
                            (
                                "unmanaged_conflict".to_string(),
                                "directory replaced an expected symlink".to_string(),
                            )
                        }
                    } else {
                        (
                            "unmanaged_conflict".to_string(),
                            "pre-existing unmanaged directory".to_string(),
                        )
                    }
                }
                Ok(_) => (
                    "unmanaged_conflict".to_string(),
                    "pre-existing unmanaged file".to_string(),
                ),
            };

            entries.push(CheckEntry {
                tools: target.tool_ids.clone(),
                skill: skill.name.clone(),
                target_path: dst,
                status,
                detail,
            });
        }
    }

    Ok(CheckReport {
        manifest_path,
        entries,
    })
}

pub fn run_uninstall(req: &UninstallRequest) -> Result<UninstallReport, String> {
    let home_dir = targets::resolve_home(req.home.as_deref())?;
    let proj_dir = resolve_project_dir(req.project_dir.as_deref())?;
    let dsh_home = dsh_home_from_env();
    let targets = targets::resolve_targets(
        &req.tools,
        req.scope,
        &home_dir,
        &proj_dir,
        dsh_home.as_deref(),
    )?;
    let manifest_path = manifest_location(req.scope, &home_dir, &proj_dir);
    let mut manifest = Manifest::load(&manifest_path)?;

    let skill_filter: Option<BTreeSet<String>> = if req.skills.trim() == "all" {
        None
    } else {
        let mut set = BTreeSet::new();
        for tok in req.skills.split(',') {
            let trimmed = tok.trim();
            if !trimmed.is_empty() {
                validate_safe_skill_name(trimmed)?;
                set.insert(trimmed.to_string());
            }
        }
        Some(set)
    };

    let mut entries = Vec::new();
    let mut to_remove: Vec<(String, PathBuf)> = Vec::new();

    for target in &targets {
        let keys_matching: Vec<(String, PathBuf, String)> = manifest
            .entries
            .keys()
            .filter_map(|k| {
                let p = PathBuf::from(k);
                if p.parent() == Some(target.skills_dir.as_path()) {
                    let skill = p.file_name()?.to_str()?.to_string();
                    if skill_filter
                        .as_ref()
                        .is_none_or(|filter| filter.contains(&skill))
                    {
                        return Some((k.clone(), p, skill));
                    }
                }
                None
            })
            .collect();

        for (key, dst, skill) in keys_matching {
            let owned = &manifest.entries[&key];
            match fs::symlink_metadata(&dst) {
                Err(_) => {
                    entries.push(PlannedEntry {
                        tools: target.tool_ids.clone(),
                        skill,
                        target_path: dst,
                        action: "prune_manifest".to_string(),
                        detail: "already absent on disk".to_string(),
                    });
                    if req.apply {
                        to_remove.push((key, PathBuf::new()));
                    }
                }
                Ok(meta) if meta.file_type().is_symlink() && owned.kind == "symlink" => {
                    entries.push(PlannedEntry {
                        tools: target.tool_ids.clone(),
                        skill,
                        target_path: dst.clone(),
                        action: "remove".to_string(),
                        detail: "owned symlink".to_string(),
                    });
                    if req.apply {
                        to_remove.push((key, dst));
                    }
                }
                Ok(meta) if meta.is_dir() && owned.kind == "copy" => {
                    let actual = collect_dir_hashes(&dst)?;
                    if actual == owned.files {
                        entries.push(PlannedEntry {
                            tools: target.tool_ids.clone(),
                            skill,
                            target_path: dst.clone(),
                            action: "remove".to_string(),
                            detail: "unmodified owned copy".to_string(),
                        });
                        if req.apply {
                            to_remove.push((key, dst));
                        }
                    } else {
                        entries.push(PlannedEntry {
                            tools: target.tool_ids.clone(),
                            skill,
                            target_path: dst,
                            action: "preserve".to_string(),
                            detail: "locally modified copy preserved".to_string(),
                        });
                    }
                }
                Ok(_) => {
                    entries.push(PlannedEntry {
                        tools: target.tool_ids.clone(),
                        skill,
                        target_path: dst,
                        action: "preserve".to_string(),
                        detail: "entry no longer matches owned manifest type".to_string(),
                    });
                }
            }
        }
    }

    if req.apply {
        for (key, path) in to_remove {
            if !path.as_os_str().is_empty() {
                remove_existing_path(&path)?;
            }
            manifest.entries.remove(&key);
        }
        if manifest.entries.is_empty() {
            if manifest_path.exists() {
                let _ = fs::remove_file(&manifest_path);
            }
            if req.scope == Scope::Global {
                let store = canonical_store_root(&home_dir);
                if store.exists() {
                    let _ = fs::remove_dir_all(&store);
                }
            }
        } else {
            manifest.save(&manifest_path)?;
        }
    }

    Ok(UninstallReport {
        apply: req.apply,
        manifest_path,
        entries,
    })
}

fn classify_install_target(
    dst: &Path,
    mode: InstallMode,
    link_src: &Path,
    expected_hashes: &BTreeMap<String, String>,
    owned: Option<&ManifestEntry>,
    force: bool,
) -> Result<(String, String), String> {
    match fs::symlink_metadata(dst) {
        Err(_) => Ok(("install".to_string(), "new target".to_string())),
        Ok(meta) if meta.file_type().is_symlink() => {
            let raw_target = fs::read_link(dst)
                .map_err(|e| format!("failed to read link {}: {e}", dst.display()))?;
            let resolved = resolve_link_target(dst, &raw_target);
            let owned_match = owned
                .is_some_and(|m| m.kind == "symlink" && Path::new(&m.source_or_target) == resolved);
            if mode == InstallMode::Symlink && resolved == link_src && owned_match {
                Ok(("skip".to_string(), "already up to date".to_string()))
            } else if owned_match || force {
                Ok((
                    "update".to_string(),
                    if force && !owned_match {
                        "forced overwrite".to_string()
                    } else {
                        "update owned symlink".to_string()
                    },
                ))
            } else {
                Ok((
                    "conflict".to_string(),
                    format!("unmanaged symlink exists (-> {})", resolved.display()),
                ))
            }
        }
        Ok(meta) if meta.is_dir() => {
            if let Some(m) = owned
                && m.kind == "copy"
            {
                let actual = collect_dir_hashes(dst)?;
                if actual == m.files {
                    if mode == InstallMode::Copy && actual == *expected_hashes {
                        return Ok(("skip".to_string(), "already up to date".to_string()));
                    }
                    return Ok(("update".to_string(), "update owned copy".to_string()));
                }
                if force {
                    return Ok((
                        "update".to_string(),
                        "forced overwrite of edited copy".to_string(),
                    ));
                }
                return Ok((
                    "conflict".to_string(),
                    "installed copy has local modifications (pass --force to overwrite)"
                        .to_string(),
                ));
            }
            if force {
                Ok((
                    "update".to_string(),
                    "forced overwrite of unmanaged directory".to_string(),
                ))
            } else {
                Ok((
                    "conflict".to_string(),
                    "unmanaged directory exists (pass --force to overwrite)".to_string(),
                ))
            }
        }
        Ok(_) => {
            if force {
                Ok((
                    "update".to_string(),
                    "forced overwrite of existing file".to_string(),
                ))
            } else {
                Ok((
                    "conflict".to_string(),
                    "unmanaged file exists (pass --force to overwrite)".to_string(),
                ))
            }
        }
    }
}

fn resolve_link_target(link_path: &Path, raw: &Path) -> PathBuf {
    if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        link_path.parent().unwrap_or(Path::new(".")).join(raw)
    }
}

fn remove_existing_path(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Err(_) => Ok(()),
        Ok(meta) if meta.file_type().is_symlink() || meta.is_file() => {
            fs::remove_file(path).map_err(|e| format!("failed to remove {}: {e}", path.display()))
        }
        Ok(_) => fs::remove_dir_all(path)
            .map_err(|e| format!("failed to remove directory {}: {e}", path.display())),
    }
}

fn sync_copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    remove_existing_path(dst)?;
    copy_dir_recursive(src, dst)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("failed to create {}: {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("failed to read {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("dir read error: {e}"))?;
        let name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let meta = fs::symlink_metadata(&src_path)
            .map_err(|e| format!("stat failed for {}: {e}", src_path.display()))?;
        if meta.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if meta.is_file() {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "failed to copy {} -> {}: {e}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn collect_dir_hashes(dir: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    collect_dir_hashes_inner(dir, dir, &mut out)?;
    Ok(out)
}

fn collect_dir_hashes_inner(
    root: &Path,
    cur: &Path,
    out: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(cur)
        .map_err(|e| format!("failed to read {}: {e}", cur.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        let meta = fs::symlink_metadata(&path)
            .map_err(|e| format!("stat failed for {}: {e}", path.display()))?;
        if meta.is_dir() {
            collect_dir_hashes_inner(root, &path, out)?;
        } else if meta.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| format!("strip_prefix error: {e}"))?
                .to_string_lossy()
                .replace('\\', "/");
            let hash = sha256::sha256_file(&path)
                .map_err(|e| format!("failed to hash {}: {e}", path.display()))?;
            out.insert(rel, hash);
        }
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(src, dst).map_err(|e| {
        format!(
            "failed to symlink {} -> {}: {e}",
            dst.display(),
            src.display()
        )
    })
}

#[cfg(not(unix))]
fn create_symlink(_src: &Path, _dst: &Path) -> Result<(), String> {
    Err("symlink mode is supported only on Unix targets; pass --mode copy".to_string())
}
