//! Coverage / freshness audit between cookbooks and skills.
//!
//! Skills are resolved at deploy time in this repo (see `deploy::build_deploy_payload`),
//! so `sync_skills` does NOT copy any files. It walks `cookbooks_root`, reads each
//! `agent.json`, collects the `(cookbook_slug, skill_slug)` pairs, then walks
//! `skills_root` to find every `<slug>/SKILL.md` on disk. Output flags:
//!
//! - `skill_usage` — every skill referenced by any cookbook with the list of
//!   cookbooks referencing it.
//! - `missing_skills` — `(cookbook, skill)` pairs where the referenced skill
//!   does not resolve to `<skills_root>/<slug>/SKILL.md`.
//! - `orphan_skills` — skills present on disk that no cookbook references.
//!
//! Pure function — only filesystem reads, no network I/O.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use crate::managed_agent::types::{AgentManifest, SkillUsage, SyncInput, SyncOutput};
use crate::CorpFinanceResult;

/// Audit cookbook→skill references vs. skills present on disk.
pub fn sync_skills(input: &SyncInput) -> CorpFinanceResult<SyncOutput> {
    let cookbooks_root = Path::new(&input.cookbooks_root);
    if !cookbooks_root.is_dir() {
        return Err(crate::error::CorpFinanceError::InvalidInput {
            field: "cookbooks_root".to_string(),
            reason: format!("not a directory: {}", cookbooks_root.display()),
        });
    }

    let skills_root = Path::new(&input.skills_root);
    if !skills_root.is_dir() {
        return Err(crate::error::CorpFinanceError::InvalidInput {
            field: "skills_root".to_string(),
            reason: format!("not a directory: {}", skills_root.display()),
        });
    }

    // 1. Discover cookbook slugs (subdirs containing agent.json).
    let cookbook_slugs = discover_cookbook_slugs(cookbooks_root)?;

    // 2. Discover skill slugs (subdirs containing SKILL.md).
    let on_disk_skills = discover_skill_slugs(skills_root)?;

    // 3. For each cookbook, parse agent.json and collect skill references.
    //    Use BTreeMap so output is deterministic (sorted by skill slug).
    let mut usage: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut missing: Vec<SkillUsage> = Vec::new();

    for cookbook_slug in &cookbook_slugs {
        let manifest_path = cookbooks_root.join(cookbook_slug).join("agent.json");
        let contents = match std::fs::read_to_string(&manifest_path) {
            Ok(c) => c,
            Err(_) => continue, // discovered earlier, but tolerate read failures
        };
        let manifest: AgentManifest = match serde_json::from_str(&contents) {
            Ok(m) => m,
            Err(_) => continue, // malformed manifests are validate's problem, skip here
        };
        for skill_ref in &manifest.skills {
            let skill_slug = skill_ref.from_skill.clone();
            usage
                .entry(skill_slug.clone())
                .or_default()
                .insert(cookbook_slug.clone());
            if !on_disk_skills.contains(&skill_slug) {
                missing.push(SkillUsage {
                    cookbook: cookbook_slug.clone(),
                    skill: skill_slug,
                });
            }
        }
    }

    // 4. Orphan skills: on disk but not referenced.
    let referenced: BTreeSet<&String> = usage.keys().collect();
    let orphans: Vec<String> = on_disk_skills
        .iter()
        .filter(|s| !referenced.contains(s))
        .cloned()
        .collect();

    // 5. Convert BTreeMap<_, BTreeSet<_>> → HashMap<String, Vec<String>> for serde output.
    let skill_usage: HashMap<String, Vec<String>> = usage
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect();

    Ok(SyncOutput {
        cookbooks_root: input.cookbooks_root.clone(),
        skills_root: input.skills_root.clone(),
        total_cookbooks: cookbook_slugs.len(),
        total_skills: on_disk_skills.len(),
        skill_usage,
        missing_skills: missing,
        orphan_skills: orphans,
    })
}

// ---------------------------------------------------------------------------
// Discovery helpers
// ---------------------------------------------------------------------------

fn discover_cookbook_slugs(root: &Path) -> CorpFinanceResult<Vec<String>> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("Cannot read cookbooks root {}: {}", root.display(), e))?;
    let mut slugs: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("agent.json").exists() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            slugs.push(name.to_string());
        }
    }
    slugs.sort();
    Ok(slugs)
}

fn discover_skill_slugs(root: &Path) -> CorpFinanceResult<BTreeSet<String>> {
    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("Cannot read skills root {}: {}", root.display(), e))?;
    let mut slugs: BTreeSet<String> = BTreeSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("SKILL.md").exists() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            slugs.insert(name.to_string());
        }
    }
    Ok(slugs)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic temp tree:
    /// - `<tmp>/cookbooks/<slug>/agent.json` (manifest with given skills)
    /// - `<tmp>/skills/<slug>/SKILL.md`
    struct TempTree {
        root: std::path::PathBuf,
    }

    impl TempTree {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!("cfa_sync_test_{}", name));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::create_dir_all(root.join("cookbooks")).unwrap();
            std::fs::create_dir_all(root.join("skills")).unwrap();
            TempTree { root }
        }

        fn cookbooks(&self) -> std::path::PathBuf {
            self.root.join("cookbooks")
        }

        fn skills(&self) -> std::path::PathBuf {
            self.root.join("skills")
        }

        fn add_cookbook(&self, slug: &str, skill_slugs: &[&str]) {
            let dir = self.cookbooks().join(slug);
            std::fs::create_dir_all(&dir).unwrap();
            let skills_json: Vec<String> = skill_slugs
                .iter()
                .map(|s| format!(r#"{{"from_skill":"{}"}}"#, s))
                .collect();
            let manifest = format!(
                r#"{{"name":"cfa-{slug}","model":"claude-opus-4-7","system":{{"text":"x"}},"tools":[],"mcp_servers":[],"skills":[{}],"callable_agents":[]}}"#,
                skills_json.join(","),
                slug = slug
            );
            std::fs::write(dir.join("agent.json"), manifest).unwrap();
        }

        fn add_skill(&self, slug: &str) {
            let dir = self.skills().join(slug);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("SKILL.md"), "# stub\n").unwrap();
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn make_input(t: &TempTree) -> SyncInput {
        SyncInput {
            cookbooks_root: t.cookbooks().display().to_string(),
            skills_root: t.skills().display().to_string(),
        }
    }

    #[test]
    fn errors_on_missing_cookbooks_root() {
        let input = SyncInput {
            cookbooks_root: "/tmp/cfa_sync_no_such_cookbooks_dir".to_string(),
            skills_root: "/tmp".to_string(),
        };
        assert!(sync_skills(&input).is_err());
    }

    #[test]
    fn empty_dirs_return_zero_counts() {
        let t = TempTree::new("empty");
        let out = sync_skills(&make_input(&t)).unwrap();
        assert_eq!(out.total_cookbooks, 0);
        assert_eq!(out.total_skills, 0);
        assert!(out.skill_usage.is_empty());
        assert!(out.missing_skills.is_empty());
        assert!(out.orphan_skills.is_empty());
    }

    #[test]
    fn detects_missing_skill_referenced_by_cookbook() {
        let t = TempTree::new("missing");
        t.add_cookbook(
            "equity-analyst",
            &["corp-finance-analyst-core", "ghost-skill"],
        );
        t.add_skill("corp-finance-analyst-core");
        // intentionally do NOT add "ghost-skill"

        let out = sync_skills(&make_input(&t)).unwrap();
        assert_eq!(out.total_cookbooks, 1);
        assert_eq!(out.total_skills, 1);
        assert_eq!(out.missing_skills.len(), 1);
        assert_eq!(out.missing_skills[0].cookbook, "equity-analyst");
        assert_eq!(out.missing_skills[0].skill, "ghost-skill");
        assert!(out.orphan_skills.is_empty());
    }

    #[test]
    fn detects_orphan_skill_not_referenced_by_any_cookbook() {
        let t = TempTree::new("orphan");
        t.add_cookbook("equity-analyst", &["corp-finance-analyst-core"]);
        t.add_skill("corp-finance-analyst-core");
        t.add_skill("orphan-skill"); // not referenced

        let out = sync_skills(&make_input(&t)).unwrap();
        assert_eq!(out.total_skills, 2);
        assert!(out.missing_skills.is_empty());
        assert_eq!(out.orphan_skills, vec!["orphan-skill".to_string()]);
    }

    #[test]
    fn happy_path_multiple_cookbooks_share_skills() {
        let t = TempTree::new("happy");
        t.add_cookbook(
            "equity-analyst",
            &["corp-finance-analyst-core", "data-fred"],
        );
        t.add_cookbook("credit-analyst", &["corp-finance-analyst-core"]);
        t.add_skill("corp-finance-analyst-core");
        t.add_skill("data-fred");

        let out = sync_skills(&make_input(&t)).unwrap();
        assert_eq!(out.total_cookbooks, 2);
        assert_eq!(out.total_skills, 2);
        assert!(out.missing_skills.is_empty());
        assert!(out.orphan_skills.is_empty());

        let core_users = out
            .skill_usage
            .get("corp-finance-analyst-core")
            .expect("core skill should appear in usage map");
        assert_eq!(core_users.len(), 2);
        assert!(core_users.contains(&"credit-analyst".to_string()));
        assert!(core_users.contains(&"equity-analyst".to_string()));

        let fred_users = out.skill_usage.get("data-fred").unwrap();
        assert_eq!(fred_users, &vec!["equity-analyst".to_string()]);
    }

    #[test]
    fn ignores_subdirs_without_agent_json_or_skill_md() {
        let t = TempTree::new("ignored");
        // cookbook subdir without agent.json
        std::fs::create_dir_all(t.cookbooks().join("not-a-cookbook")).unwrap();
        // skill subdir without SKILL.md
        std::fs::create_dir_all(t.skills().join("not-a-skill")).unwrap();
        t.add_cookbook("equity-analyst", &["corp-finance-analyst-core"]);
        t.add_skill("corp-finance-analyst-core");

        let out = sync_skills(&make_input(&t)).unwrap();
        assert_eq!(out.total_cookbooks, 1);
        assert_eq!(out.total_skills, 1);
        assert!(out.missing_skills.is_empty());
        assert!(out.orphan_skills.is_empty());
    }
}
