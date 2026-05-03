//! `cfa-codegen lint-skills [--check]` — port of `scripts/link-skills-tools.py`.
//!
//! Walks `plugins/cfa-core/skills/*/SKILL.md` and
//! `plugins/cfa-core/commands/cfa/*.md`, extracts backtick tool references,
//! cross-references against the schemas.json manifest, and (in non-check
//! mode) updates each file's frontmatter `requires_tools:` array.
//!
//! `--check` mode exits non-zero if any reference is unresolved — used by CI.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;
use walkdir::WalkDir;

use crate::data::ext_prefixes::{EXTERNAL_TOOL_PREFIXES, KNOWN_NON_TOOLS, LEGACY_ALIASES};

/// Lightweight YAML frontmatter (only what link-skills-tools.py supports):
/// scalar `key: value` and list `key:` followed by indented `  - item` lines.
/// Preserves insertion order via Vec.
#[derive(Debug, Clone, Default)]
pub struct Frontmatter {
    pub entries: Vec<(String, FrontmatterValue)>,
}

#[derive(Debug, Clone)]
pub enum FrontmatterValue {
    Scalar(String),
    List(Vec<String>),
}

impl Frontmatter {
    pub fn get(&self, key: &str) -> Option<&FrontmatterValue> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    pub fn set(&mut self, key: &str, value: FrontmatterValue) {
        if let Some(slot) = self
            .entries
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
        {
            *slot = value;
            return;
        }
        self.entries.push((key.to_string(), value));
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }
}

pub fn parse_frontmatter(text: &str) -> (Frontmatter, String) {
    // Match Python: re.match(r"^---\s*\n(.*?)\n---\s*\n(.*)$", DOTALL)
    let re = Regex::new(r"(?s)^---\s*\n(.*?)\n---\s*\n(.*)$").unwrap();
    let caps = match re.captures(text) {
        Some(c) => c,
        None => return (Frontmatter::default(), text.to_string()),
    };
    let body = caps.get(2).unwrap().as_str().to_string();
    let raw = caps.get(1).unwrap().as_str();

    let mut fm = Frontmatter::default();
    let mut cur_key: Option<String> = None;
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            // Continuation / list item — only handled if current value is a list.
            if let Some(key) = &cur_key {
                if let Some(FrontmatterValue::List(items)) = fm
                    .entries
                    .iter_mut()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v)
                {
                    let trimmed = line.trim();
                    let item = trimmed.strip_prefix('-').unwrap_or(trimmed).trim();
                    if !item.is_empty() {
                        items.push(item.to_string());
                    }
                }
            }
            continue;
        }
        if let Some(idx) = line.find(':') {
            let k = line[..idx].trim().to_string();
            let v_raw = line[idx + 1..].trim();
            let v = strip_yaml_quotes(v_raw);
            if v.is_empty() {
                fm.entries
                    .push((k.clone(), FrontmatterValue::List(Vec::new())));
            } else {
                fm.entries
                    .push((k.clone(), FrontmatterValue::Scalar(v)));
            }
            cur_key = Some(k);
        }
    }
    (fm, body)
}

fn strip_yaml_quotes(s: &str) -> String {
    // Python: v.strip().strip('"').strip("'")
    // strip removes any chars in set, repeatedly. Mimic by trimming any
    // outer matched quote pair.
    let mut t = s.to_string();
    while t.starts_with('"') {
        t = t[1..].to_string();
    }
    while t.ends_with('"') {
        t.pop();
    }
    while t.starts_with('\'') {
        t = t[1..].to_string();
    }
    while t.ends_with('\'') {
        t.pop();
    }
    t
}

pub fn render_frontmatter(fm: &Frontmatter, body: &str) -> String {
    let mut lines: Vec<String> = vec!["---".to_string()];
    for (k, v) in &fm.entries {
        match v {
            FrontmatterValue::List(items) => {
                if items.is_empty() {
                    lines.push(format!("{}: []", k));
                } else {
                    lines.push(format!("{}:", k));
                    for item in items {
                        lines.push(format!("  - {}", item));
                    }
                }
            }
            FrontmatterValue::Scalar(s) => {
                if s.contains(':') {
                    lines.push(format!("{}: \"{}\"", k, s));
                } else {
                    lines.push(format!("{}: {}", k, s));
                }
            }
        }
    }
    lines.push("---".to_string());
    let mut out = lines.join("\n");
    out.push('\n');
    out.push_str(body);
    out
}

/// Extract backtick tool references from a body.
///
/// Matches `[a-z][a-z0-9_]+`, requires at least one underscore, drops
/// known non-tool tokens. Returns sorted-deduped names.
pub fn extract_references(body: &str) -> BTreeSet<String> {
    let re = Regex::new(r"`([a-z][a-z0-9_]+)`").unwrap();
    let banned: std::collections::HashSet<&str> = KNOWN_NON_TOOLS.iter().copied().collect();
    let mut out = BTreeSet::new();
    for caps in re.captures_iter(body) {
        let token = caps.get(1).unwrap().as_str();
        if banned.contains(token) {
            continue;
        }
        if !token.contains('_') {
            continue;
        }
        out.insert(token.to_string());
    }
    out
}

pub fn is_external(name: &str) -> bool {
    EXTERNAL_TOOL_PREFIXES
        .iter()
        .any(|p| name.starts_with(p))
}

/// Python's `difflib.get_close_matches(word, possibilities, n=2, cutoff=0.6)`
/// uses SequenceMatcher.ratio(). We approximate with a normalised
/// Levenshtein-ish ratio: 1 - (edit_distance / max_len). Good enough for
/// suggesting "calculate_merton" when the user typed "merton_model".
fn similarity_ratio(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    let la = a.chars().count();
    let lb = b.chars().count();
    if la == 0 || lb == 0 {
        return 0.0;
    }
    let dist = levenshtein(a, b);
    let max = la.max(lb) as f32;
    1.0 - (dist as f32 / max)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let av: Vec<char> = a.chars().collect();
    let bv: Vec<char> = b.chars().collect();
    let m = av.len();
    let n = bv.len();
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut cur: Vec<usize> = vec![0; n + 1];
    for i in 1..=m {
        cur[0] = i;
        for j in 1..=n {
            let cost = if av[i - 1] == bv[j - 1] { 0 } else { 1 };
            cur[j] = (cur[j - 1] + 1)
                .min(prev[j] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[n]
}

pub fn close_matches(word: &str, candidates: &BTreeSet<String>, n: usize, cutoff: f32) -> Vec<String> {
    let mut scored: Vec<(f32, &String)> = candidates
        .iter()
        .map(|c| (similarity_ratio(word, c), c))
        .filter(|(s, _)| *s >= cutoff)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(n)
        .map(|(_, c)| c.clone())
        .collect()
}

pub struct LintResult {
    pub total_refs: usize,
    pub total_resolved: usize,
    pub total_external: usize,
    pub unresolved_per_file: BTreeMap<String, Vec<(String, Vec<String>)>>,
    pub updated_files: usize,
}

pub fn run(repo_root: &Path, check: bool) -> Result<i32> {
    let plugin = repo_root.join("plugins").join("cfa-core");
    let skills_dir = plugin.join("skills");
    let commands_dir = plugin.join("commands").join("cfa");
    let schemas_path = plugin.join("mcp").join("src").join("schemas.json");

    let manifest_text = std::fs::read_to_string(&schemas_path)
        .with_context(|| format!("read {}", schemas_path.display()))?;
    let manifest: Value = serde_json::from_str(&manifest_text)?;
    let mut known: BTreeSet<String> = manifest
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    for alias in LEGACY_ALIASES {
        known.insert((*alias).to_string());
    }
    println!("Known cfa-core tools : {}", known.len());

    let md_files = collect_md_files(&skills_dir, &commands_dir);
    println!("Markdown files       : {}", md_files.len());

    let mut total_refs = 0usize;
    let mut total_resolved = 0usize;
    let mut total_external = 0usize;
    let mut unresolved_per_file: BTreeMap<String, Vec<(String, Vec<String>)>> = BTreeMap::new();
    let mut updated_files = 0usize;

    for path in &md_files {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let (mut fm, body) = parse_frontmatter(&text);
        let refs = extract_references(&body);
        if refs.is_empty() {
            continue;
        }

        let resolved: Vec<String> = refs
            .iter()
            .filter(|r| known.contains(*r))
            .cloned()
            .collect();
        let external: Vec<String> = refs.iter().filter(|r| is_external(r)).cloned().collect();
        let unresolved: Vec<String> = refs
            .iter()
            .filter(|r| !known.contains(*r) && !is_external(r))
            .cloned()
            .collect();

        total_refs += refs.len();
        total_resolved += resolved.len();
        total_external += external.len();

        if !unresolved.is_empty() {
            let mut entries: Vec<(String, Vec<String>)> = Vec::new();
            for r in &unresolved {
                let close = close_matches(r, &known, 2, 0.6);
                entries.push((r.clone(), close));
            }
            let rel = path
                .strip_prefix(repo_root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            unresolved_per_file.insert(rel, entries);
        }

        if !check && !resolved.is_empty() {
            let existing: BTreeSet<String> = match fm.get("requires_tools") {
                Some(FrontmatterValue::List(items)) => items.iter().cloned().collect(),
                _ => BTreeSet::new(),
            };
            let mut new_set: BTreeSet<String> = existing.clone();
            for r in &resolved {
                new_set.insert(r.clone());
            }
            let new_list: Vec<String> = new_set.iter().cloned().collect();
            // Was the existing already a sorted list matching new? Then skip.
            let existing_list: Vec<String> = existing.iter().cloned().collect();
            if new_list != existing_list || !fm.contains("requires_tools") {
                fm.set(
                    "requires_tools",
                    FrontmatterValue::List(new_list),
                );
                if !external.is_empty() {
                    let mut ext_sorted = external.clone();
                    ext_sorted.sort();
                    ext_sorted.dedup();
                    fm.set(
                        "requires_external_tools",
                        FrontmatterValue::List(ext_sorted),
                    );
                }
                let rendered = render_frontmatter(&fm, &body);
                std::fs::write(path, rendered)
                    .with_context(|| format!("write {}", path.display()))?;
                updated_files += 1;
            }
        }
    }

    println!("References found     : {}", total_refs);
    println!("Resolved (cfa-core)  : {}", total_resolved);
    println!("External (other MCP) : {}", total_external);
    let unresolved_total: usize = unresolved_per_file.values().map(|v| v.len()).sum();
    println!("Unresolved           : {}", unresolved_total);

    if !check {
        println!("Files updated        : {}", updated_files);
    }

    if !unresolved_per_file.is_empty() {
        println!("\n## Unresolved references");
        for (fpath, items) in &unresolved_per_file {
            println!("\n  {}", fpath);
            for (r, suggestions) in items {
                if !suggestions.is_empty() {
                    println!(
                        "    - `{}` → did you mean: {}?",
                        r,
                        suggestions.join(", ")
                    );
                } else {
                    println!("    - `{}` (no close match)", r);
                }
            }
        }
        if check {
            return Ok(1);
        }
    }

    Ok(0)
}

fn collect_md_files(skills_dir: &Path, commands_dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    if skills_dir.exists() {
        for entry in WalkDir::new(skills_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file()
                && entry.file_name() == std::ffi::OsStr::new("SKILL.md")
            {
                files.push(entry.into_path());
            }
        }
    }
    if commands_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(commands_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_backtick_refs() {
        let body = "Use `calculate_wacc` and `build_dcf`. Skip `result` and `n`.";
        let refs = extract_references(body);
        assert!(refs.contains("calculate_wacc"));
        assert!(refs.contains("build_dcf"));
        assert!(!refs.contains("result"));
        assert!(!refs.contains("n"));
    }

    #[test]
    fn parses_simple_frontmatter() {
        let text = "---\nname: foo\ndescription: bar\n---\nbody here\n";
        let (fm, body) = parse_frontmatter(text);
        assert_eq!(body, "body here\n");
        assert_eq!(fm.entries.len(), 2);
        match fm.get("name").unwrap() {
            FrontmatterValue::Scalar(s) => assert_eq!(s, "foo"),
            _ => panic!("expected scalar"),
        }
    }

    #[test]
    fn parses_list_frontmatter() {
        let text = "---\nrequires_tools:\n  - calculate_wacc\n  - build_dcf\n---\nbody\n";
        let (fm, _) = parse_frontmatter(text);
        match fm.get("requires_tools").unwrap() {
            FrontmatterValue::List(items) => assert_eq!(items, &vec!["calculate_wacc", "build_dcf"]),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn close_match_finds_typo() {
        let mut known = BTreeSet::new();
        known.insert("calculate_merton".to_string());
        known.insert("calculate_wacc".to_string());
        let matches = close_matches("calculate_marton", &known, 2, 0.6);
        assert!(matches.first().map(String::as_str) == Some("calculate_merton"));
    }
}
