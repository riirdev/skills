use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::json::{self, JsonValue};

#[derive(Debug, Default, Clone)]
pub struct AuditThresholds {
    pub forbid_static_mut: bool,
    pub require_safety_docs: bool,
    pub forbid_file_allows: bool,
    pub max_unsafe_blocks: Option<usize>,
    pub max_unsafe_fns: Option<usize>,
    pub min_forbid_files: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct FileAudit {
    pub path: PathBuf,
    pub forbid_unsafe_code: bool,
    pub file_allows: usize,
    pub unsafe_blocks: usize,
    pub documented_unsafe_blocks: usize,
    pub undocumented_unsafe_lines: Vec<usize>,
    pub unsafe_fns: usize,
    pub unsafe_impls_or_traits: usize,
    pub static_muts: usize,
}

#[derive(Debug, Clone)]
pub struct AuditReport {
    pub files_scanned: usize,
    pub forbid_files: usize,
    pub total_file_allows: usize,
    pub total_unsafe_blocks: usize,
    pub total_documented_unsafe_blocks: usize,
    pub total_unsafe_fns: usize,
    pub total_unsafe_impls_or_traits: usize,
    pub total_static_muts: usize,
    pub files: Vec<FileAudit>,
    pub failures: Vec<String>,
}

impl AuditReport {
    pub fn print_human(&self) {
        println!(
            "Unsafe Audit: {} file(s) scanned | forbid(unsafe_code)={} | file_allows={} | unsafe_blocks={} (documented={}) | unsafe_fn={} | unsafe_impl/trait={} | static_mut={}",
            self.files_scanned,
            self.forbid_files,
            self.total_file_allows,
            self.total_unsafe_blocks,
            self.total_documented_unsafe_blocks,
            self.total_unsafe_fns,
            self.total_unsafe_impls_or_traits,
            self.total_static_muts
        );
        for f in &self.files {
            if f.unsafe_blocks > 0
                || f.unsafe_fns > 0
                || f.unsafe_impls_or_traits > 0
                || f.static_muts > 0
                || f.file_allows > 0
            {
                println!(
                    "  - {}: blocks={} (undoc={:?}) fn={} impl/trait={} static_mut={} allows={}",
                    f.path.display(),
                    f.unsafe_blocks,
                    f.undocumented_unsafe_lines,
                    f.unsafe_fns,
                    f.unsafe_impls_or_traits,
                    f.static_muts,
                    f.file_allows
                );
            }
        }
        if !self.failures.is_empty() {
            println!("Gate failures:");
            for fail in &self.failures {
                println!("  * {fail}");
            }
        }
    }

    pub fn to_json(&self) -> String {
        let mut root = BTreeMap::new();
        root.insert("ok".to_string(), JsonValue::Bool(self.failures.is_empty()));
        root.insert(
            "files_scanned".to_string(),
            JsonValue::Number(self.files_scanned.to_string()),
        );
        root.insert(
            "forbid_files".to_string(),
            JsonValue::Number(self.forbid_files.to_string()),
        );
        root.insert(
            "total_file_allows".to_string(),
            JsonValue::Number(self.total_file_allows.to_string()),
        );
        root.insert(
            "total_unsafe_blocks".to_string(),
            JsonValue::Number(self.total_unsafe_blocks.to_string()),
        );
        root.insert(
            "total_documented_unsafe_blocks".to_string(),
            JsonValue::Number(self.total_documented_unsafe_blocks.to_string()),
        );
        root.insert(
            "total_unsafe_fns".to_string(),
            JsonValue::Number(self.total_unsafe_fns.to_string()),
        );
        root.insert(
            "total_unsafe_impls_or_traits".to_string(),
            JsonValue::Number(self.total_unsafe_impls_or_traits.to_string()),
        );
        root.insert(
            "total_static_muts".to_string(),
            JsonValue::Number(self.total_static_muts.to_string()),
        );
        root.insert(
            "failures".to_string(),
            JsonValue::Array(
                self.failures
                    .iter()
                    .map(|f| JsonValue::String(f.clone()))
                    .collect(),
            ),
        );
        json::to_string_pretty(&JsonValue::Object(root))
    }
}

pub fn run_audit(paths: &[PathBuf], thresholds: &AuditThresholds) -> Result<AuditReport, String> {
    let mut rs_files = Vec::new();
    for p in paths {
        collect_rs_files(p, &mut rs_files)?;
    }
    rs_files.sort();
    rs_files.dedup();

    if rs_files.is_empty() {
        return Err("no .rs files found under specified path(s)".to_string());
    }

    let mut files = Vec::with_capacity(rs_files.len());
    for file in rs_files {
        let src = fs::read_to_string(&file)
            .map_err(|e| format!("failed to read {}: {e}", file.display()))?;
        files.push(audit_source(&file, &src));
    }

    let forbid_files = files.iter().filter(|f| f.forbid_unsafe_code).count();
    let total_file_allows: usize = files.iter().map(|f| f.file_allows).sum();
    let total_unsafe_blocks: usize = files.iter().map(|f| f.unsafe_blocks).sum();
    let total_documented_unsafe_blocks: usize =
        files.iter().map(|f| f.documented_unsafe_blocks).sum();
    let total_unsafe_fns: usize = files.iter().map(|f| f.unsafe_fns).sum();
    let total_unsafe_impls_or_traits: usize = files.iter().map(|f| f.unsafe_impls_or_traits).sum();
    let total_static_muts: usize = files.iter().map(|f| f.static_muts).sum();

    let mut failures = Vec::new();
    if thresholds.forbid_static_mut && total_static_muts > 0 {
        failures.push(format!(
            "found {total_static_muts} static mut declaration(s) (--forbid-static-mut)"
        ));
    }
    if thresholds.forbid_file_allows && total_file_allows > 0 {
        failures.push(format!(
            "found {total_file_allows} file-level unsafe allow attribute(s) (--forbid-file-allows)"
        ));
    }
    if thresholds.require_safety_docs && total_documented_unsafe_blocks < total_unsafe_blocks {
        let missing = total_unsafe_blocks - total_documented_unsafe_blocks;
        failures.push(format!(
            "{missing} unsafe block(s) lack a preceding '// SAFETY:' comment (--require-safety-docs)"
        ));
    }
    if let Some(max) = thresholds.max_unsafe_blocks
        && total_unsafe_blocks > max
    {
        failures.push(format!(
            "unsafe block count {total_unsafe_blocks} exceeds max budget {max}"
        ));
    }
    if let Some(max) = thresholds.max_unsafe_fns
        && total_unsafe_fns > max
    {
        failures.push(format!(
            "unsafe fn count {total_unsafe_fns} exceeds max budget {max}"
        ));
    }
    if let Some(min) = thresholds.min_forbid_files
        && forbid_files < min
    {
        failures.push(format!(
            "#![forbid(unsafe_code)] file count {forbid_files} is below minimum {min}"
        ));
    }

    Ok(AuditReport {
        files_scanned: files.len(),
        forbid_files,
        total_file_allows,
        total_unsafe_blocks,
        total_documented_unsafe_blocks,
        total_unsafe_fns,
        total_unsafe_impls_or_traits,
        total_static_muts,
        files,
        failures,
    })
}

fn collect_rs_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for entry in entries {
        let name = entry.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if entry.is_dir() {
            if matches!(name, "target" | ".git" | "node_modules") {
                continue;
            }
            collect_rs_files(&entry, out)?;
        } else if entry.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(entry);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokKind {
    Ident(String),
    Punct(char),
}

#[derive(Debug, Clone)]
struct Tok {
    kind: TokKind,
    line: usize,
}

pub fn audit_source(path: &Path, src: &str) -> FileAudit {
    let (tokens, safety_lines) = lex_rust(src);

    let mut forbid_unsafe_code = false;
    let mut file_allows = 0usize;
    let mut unsafe_blocks = 0usize;
    let mut documented_unsafe_blocks = 0usize;
    let mut undocumented_unsafe_lines = Vec::new();
    let mut unsafe_fns = 0usize;
    let mut unsafe_impls_or_traits = 0usize;
    let mut static_muts = 0usize;

    // Detect crate/file inner attributes `#![...]` from token stream.
    let mut idx = 0usize;
    while idx + 2 < tokens.len() {
        if tokens[idx].kind == TokKind::Punct('#')
            && tokens[idx + 1].kind == TokKind::Punct('!')
            && tokens[idx + 2].kind == TokKind::Punct('[')
        {
            let mut end = idx + 3;
            let mut depth = 1usize;
            while end < tokens.len() && depth > 0 {
                match tokens[end].kind {
                    TokKind::Punct('[') => depth += 1,
                    TokKind::Punct(']') => depth -= 1,
                    _ => {}
                }
                end += 1;
            }
            let inner = &tokens[idx + 3..end.saturating_sub(1)];
            if attr_contains(inner, "forbid", "unsafe_code") {
                forbid_unsafe_code = true;
            }
            if attr_contains(inner, "allow", "unsafe_op_in_unsafe_fn")
                || attr_contains(inner, "allow", "static_mut_refs")
            {
                file_allows += 1;
            }
            idx = end;
            continue;
        }
        idx += 1;
    }

    let mut i = 0usize;
    let mut last_stmt_end_line = 0usize;
    let mut used_safety_idx = 0usize;
    while i < tokens.len() {
        if matches!(tokens[i].kind, TokKind::Punct(';' | '}')) {
            last_stmt_end_line = tokens[i].line;
        }
        match &tokens[i].kind {
            TokKind::Ident(word) if word == "unsafe" => {
                if let Some(next) = tokens.get(i + 1) {
                    match &next.kind {
                        TokKind::Punct('{') => {
                            unsafe_blocks += 1;
                            let line = tokens[i].line;
                            if consume_safety_comment(
                                &safety_lines,
                                &mut used_safety_idx,
                                last_stmt_end_line,
                                line,
                            ) {
                                documented_unsafe_blocks += 1;
                            } else {
                                undocumented_unsafe_lines.push(line);
                            }
                            last_stmt_end_line = line;
                            i += 2;
                            continue;
                        }
                        TokKind::Ident(n) if n == "fn" => {
                            unsafe_fns += 1;
                            i += 2;
                            continue;
                        }
                        TokKind::Ident(n) if n == "extern" => {
                            if is_extern_fn(&tokens[i + 2..]) {
                                unsafe_fns += 1;
                            }
                            i += 2;
                            continue;
                        }
                        TokKind::Ident(n) if n == "impl" || n == "trait" => {
                            unsafe_impls_or_traits += 1;
                            i += 2;
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            TokKind::Ident(word) if word == "static" => {
                if matches!(tokens.get(i + 1).map(|t| &t.kind), Some(TokKind::Ident(m)) if m == "mut")
                {
                    static_muts += 1;
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }

    FileAudit {
        path: path.to_path_buf(),
        forbid_unsafe_code,
        file_allows,
        unsafe_blocks,
        documented_unsafe_blocks,
        undocumented_unsafe_lines,
        unsafe_fns,
        unsafe_impls_or_traits,
        static_muts,
    }
}

fn attr_contains(inner: &[Tok], head: &str, target: &str) -> bool {
    matches!(inner.first().map(|t| &t.kind), Some(TokKind::Ident(h)) if h == head)
        && inner
            .iter()
            .any(|t| matches!(&t.kind, TokKind::Ident(id) if id == target))
}

fn is_extern_fn(after_extern: &[Tok]) -> bool {
    for tok in after_extern.iter().take(4) {
        match &tok.kind {
            TokKind::Ident(w) if w == "fn" => return true,
            TokKind::Punct(';' | '{' | '(') => return false,
            _ => {}
        }
    }
    false
}

fn consume_safety_comment(
    safety_lines: &[usize],
    used_idx: &mut usize,
    last_stmt_end_line: usize,
    unsafe_line: usize,
) -> bool {
    while *used_idx < safety_lines.len() && safety_lines[*used_idx] < last_stmt_end_line {
        *used_idx += 1;
    }
    let mut matched = false;
    while *used_idx < safety_lines.len() && safety_lines[*used_idx] <= unsafe_line {
        let s = safety_lines[*used_idx];
        *used_idx += 1;
        if unsafe_line - s <= 4 {
            matched = true;
        }
    }
    matched
}

fn lex_rust(src: &str) -> (Vec<Tok>, Vec<usize>) {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut tokens = Vec::new();
    let mut safety_lines = Vec::new();

    while i < len {
        let b = bytes[i];
        if b == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Line comment
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            let comment = String::from_utf8_lossy(&bytes[start..i]);
            if comment.contains("SAFETY:") {
                safety_lines.push(line);
            }
            continue;
        }

        // Nested block comment
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            let start_line = line;
            let start = i;
            i += 2;
            let mut depth = 1usize;
            while i < len && depth > 0 {
                if bytes[i] == b'\n' {
                    line += 1;
                    i += 1;
                } else if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let comment = String::from_utf8_lossy(&bytes[start..i]);
            if comment.contains("SAFETY:") {
                safety_lines.push(start_line);
                if line != start_line {
                    safety_lines.push(line);
                }
            }
            continue;
        }

        // Raw string literal: r#"..."#, br#"..."#, cr#"..."#
        if let Some(after_raw) = match_raw_string_prefix(bytes, i) {
            let hashes = after_raw.0;
            i = after_raw.1;
            while i < len {
                if bytes[i] == b'\n' {
                    line += 1;
                    i += 1;
                } else if bytes[i] == b'"'
                    && bytes[i + 1..]
                        .iter()
                        .take(hashes)
                        .eq(std::iter::repeat_n(&b'#', hashes))
                    && bytes[i + 1..].len() >= hashes
                {
                    i += 1 + hashes;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Normal or byte/c string literal
        if b == b'"' || ((b == b'b' || b == b'c') && bytes.get(i + 1) == Some(&b'"')) {
            if b != b'"' {
                i += 1;
            }
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(len);
                } else if bytes[i] == b'\n' {
                    line += 1;
                    i += 1;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Char / byte-char literal vs lifetime `'a`
        if b == b'\'' || (b == b'b' && bytes.get(i + 1) == Some(&b'\'')) {
            let quote_pos = if b == b'\'' { i } else { i + 1 };
            if let Some(end_pos) = scan_char_literal_end(bytes, quote_pos) {
                i = end_pos;
                continue;
            }
        }

        // Identifier
        if is_ident_start(b) {
            let start = i;
            i += 1;
            while i < len && is_ident_continue(bytes[i]) {
                i += 1;
            }
            let ident = String::from_utf8_lossy(&bytes[start..i]).to_string();
            tokens.push(Tok {
                kind: TokKind::Ident(ident),
                line,
            });
            continue;
        }

        // Punctuation
        tokens.push(Tok {
            kind: TokKind::Punct(b as char),
            line,
        });
        i += 1;
    }

    (tokens, safety_lines)
}

fn match_raw_string_prefix(bytes: &[u8], i: usize) -> Option<(usize, usize)> {
    let mut pos = i;
    if matches!(bytes.get(pos), Some(b'b' | b'c')) {
        pos += 1;
    }
    if bytes.get(pos) != Some(&b'r') {
        return None;
    }
    pos += 1;
    let mut hashes = 0usize;
    while bytes.get(pos) == Some(&b'#') {
        hashes += 1;
        pos += 1;
    }
    if bytes.get(pos) == Some(&b'"') {
        Some((hashes, pos + 1))
    } else {
        None
    }
}

fn scan_char_literal_end(bytes: &[u8], quote_pos: usize) -> Option<usize> {
    let next = *bytes.get(quote_pos + 1)?;
    if next == b'\\' {
        let mut p = quote_pos + 2;
        while p < bytes.len() && bytes[p] != b'\n' && p - quote_pos <= 12 {
            if bytes[p] == b'\'' {
                return Some(p + 1);
            }
            p += 1;
        }
        None
    } else if next != b'\n' && bytes.get(quote_pos + 2) == Some(&b'\'') {
        Some(quote_pos + 3)
    } else {
        None
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_nested_comments_raw_strings_and_lifetimes() {
        let src = r####"
#![forbid(unsafe_code)]
fn borrow<'a>(x: &'a str) -> &'a str {
    let _ch = '\'';
    let _raw = r###"unsafe { static mut BAD: u8 = 0; }"###;
    /* outer /* inner unsafe {} */ still comment unsafe fn f() {} */
    x
}
"####;
        let audit = audit_source(Path::new("clean.rs"), src);
        assert!(audit.forbid_unsafe_code);
        assert_eq!(audit.unsafe_blocks, 0);
        assert_eq!(audit.unsafe_fns, 0);
        assert_eq!(audit.static_muts, 0);
        assert_eq!(audit.file_allows, 0);
    }

    #[test]
    fn safety_comment_does_not_leak_across_statements() {
        let src = r#"
fn demo(p: *mut u8) {
    // SAFETY: p is valid and aligned
    // and exclusively borrowed.
    unsafe { *p = 1; }
    unsafe { *p = 2; }
}
"#;
        let audit = audit_source(Path::new("demo.rs"), src);
        assert_eq!(audit.unsafe_blocks, 2);
        assert_eq!(audit.documented_unsafe_blocks, 1);
        assert_eq!(audit.undocumented_unsafe_lines, vec![6]);
    }
}
