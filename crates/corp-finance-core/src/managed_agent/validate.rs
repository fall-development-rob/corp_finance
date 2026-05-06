//! Schema validation for managed-agent cookbooks.
//!
//! Pure function: reads files from disk (the cookbook directory), performs
//! structural and file-existence checks, and returns a `ValidateOutput`.
//! No network I/O.
//!
//! Pattern mirrors `crates/corp-finance-core/src/workflows/types.rs` validation helpers.

use std::path::Path;

use crate::managed_agent::types::{
    AgentManifest, CheckAllInput, CheckAllOutput, CheckResult, CookbookOutcome, ValidateInput,
    ValidateOutput, ALLOWED_SLUGS,
};
use crate::CorpFinanceResult;

/// Validate a managed-agent cookbook for a given slug.
///
/// Checks performed:
/// 1. Slug is in the allowlist.
/// 2. `agent.json` exists and parses correctly.
/// 3. Required top-level fields are present (`name`, `model`, `system`).
/// 4. `system.file` path resolves to an existing `.md` file.
/// 5. Each `skills[].from_skill` path resolves to `.claude/skills/<slug>/SKILL.md`.
/// 6. Each `callable_agents[].manifest` path resolves to an existing file.
/// 7. `steering-examples.json` exists.
pub fn validate_manifest(input: &ValidateInput) -> CorpFinanceResult<ValidateOutput> {
    let mut checks: Vec<CheckResult> = Vec::new();
    let slug = &input.slug;

    // 1. Slug allowlist
    checks.push(check_slug_allowlist(slug));

    let cookbook_dir = Path::new(&input.cookbooks_root).join(slug);

    // 2 & 3. agent.json parse + required fields
    let manifest_path = cookbook_dir.join("agent.json");
    let (manifest_opt, manifest_check) = check_manifest_parse(&manifest_path);
    checks.push(manifest_check);

    if let Some(ref manifest) = manifest_opt {
        checks.push(check_required_fields(manifest));

        // 4. system.file resolution
        if let Some(ref file_ref) = manifest.system.file {
            checks.push(check_system_prompt(
                file_ref,
                &cookbook_dir,
                Path::new(&input.agents_root),
            ));
        } else {
            checks.push(CheckResult {
                name: "system_prompt_file".to_string(),
                passed: true,
                detail: "system uses inline text (no file reference)".to_string(),
            });
        }

        // 5. Skills
        for skill in &manifest.skills {
            checks.push(check_skill_path(
                &skill.from_skill,
                Path::new(&input.skills_root),
            ));
        }

        // 6. Callable agent manifests
        for ca in &manifest.callable_agents {
            checks.push(check_subagent_manifest(&ca.manifest, &cookbook_dir));
        }
    }

    // 7. steering-examples.json
    checks.push(check_steering_examples(&cookbook_dir));

    let ok = checks.iter().all(|c| c.passed);
    Ok(ValidateOutput {
        slug: slug.clone(),
        ok,
        checks,
    })
}

/// Validate every cookbook directory found under `cookbooks_root`.
///
/// Walks `cookbooks_root` for subdirectories containing an `agent.json`,
/// runs `validate_manifest` against each, and returns an aggregate summary.
/// Slugs not in `ALLOWED_SLUGS` will produce a failed check inside their
/// own `ValidateOutput` — they are still discovered and reported, not skipped.
pub fn validate_all(input: &CheckAllInput) -> CorpFinanceResult<CheckAllOutput> {
    let root = Path::new(&input.cookbooks_root);
    if !root.is_dir() {
        return Err(crate::error::CorpFinanceError::InvalidInput {
            field: "cookbooks_root".to_string(),
            reason: format!("not a directory: {}", root.display()),
        });
    }

    let mut outcomes: Vec<CookbookOutcome> = Vec::new();

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

    for slug in &slugs {
        let v = validate_manifest(&ValidateInput {
            slug: slug.clone(),
            cookbooks_root: input.cookbooks_root.clone(),
            skills_root: input.skills_root.clone(),
            agents_root: input.agents_root.clone(),
        })?;

        let failed_checks: Vec<String> = v
            .checks
            .iter()
            .filter(|c| !c.passed)
            .map(|c| format!("{}: {}", c.name, c.detail))
            .collect();

        outcomes.push(CookbookOutcome {
            slug: v.slug,
            ok: v.ok,
            failed_checks,
        });
    }

    let passed = outcomes.iter().filter(|o| o.ok).count();
    let failed = outcomes.len() - passed;

    Ok(CheckAllOutput {
        cookbooks_root: input.cookbooks_root.clone(),
        total: outcomes.len(),
        passed,
        failed,
        outcomes,
    })
}

// ---------------------------------------------------------------------------
// Individual check helpers (all return CheckResult)
// ---------------------------------------------------------------------------

fn check_slug_allowlist(slug: &str) -> CheckResult {
    let passed = ALLOWED_SLUGS.contains(&slug);
    CheckResult {
        name: "slug_allowlist".to_string(),
        passed,
        detail: if passed {
            format!("'{}' is an allowed slug", slug)
        } else {
            format!(
                "'{}' is not in the allowed slug list: {:?}",
                slug, ALLOWED_SLUGS
            )
        },
    }
}

fn check_manifest_parse(path: &Path) -> (Option<AgentManifest>, CheckResult) {
    if !path.exists() {
        return (
            None,
            CheckResult {
                name: "agent_json_exists".to_string(),
                passed: false,
                detail: format!("agent.json not found at {}", path.display()),
            },
        );
    }
    match std::fs::read_to_string(path) {
        Err(e) => (
            None,
            CheckResult {
                name: "agent_json_exists".to_string(),
                passed: false,
                detail: format!("Cannot read {}: {}", path.display(), e),
            },
        ),
        Ok(contents) => match serde_json::from_str::<AgentManifest>(&contents) {
            Err(e) => (
                None,
                CheckResult {
                    name: "agent_json_exists".to_string(),
                    passed: false,
                    detail: format!("JSON parse error in {}: {}", path.display(), e),
                },
            ),
            Ok(m) => (
                Some(m),
                CheckResult {
                    name: "agent_json_exists".to_string(),
                    passed: true,
                    detail: "agent.json exists and parses correctly".to_string(),
                },
            ),
        },
    }
}

fn check_required_fields(manifest: &AgentManifest) -> CheckResult {
    let mut missing: Vec<&str> = Vec::new();
    if manifest.name.is_empty() {
        missing.push("name");
    }
    if manifest.model.is_empty() {
        missing.push("model");
    }
    let has_system = manifest.system.file.is_some() || manifest.system.text.is_some();
    if !has_system {
        missing.push("system.file or system.text");
    }
    let passed = missing.is_empty();
    CheckResult {
        name: "required_fields".to_string(),
        passed,
        detail: if passed {
            "name, model, system all present".to_string()
        } else {
            format!("Missing required fields: {}", missing.join(", "))
        },
    }
}

fn check_system_prompt(file_ref: &str, cookbook_dir: &Path, agents_root: &Path) -> CheckResult {
    // Try resolving relative to cookbook dir first, then agents root
    let candidate1 = cookbook_dir.join(file_ref);
    let candidate2 = agents_root.join(
        Path::new(file_ref)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(file_ref)),
    );

    let found = candidate1.exists() || candidate2.exists();
    let resolved = if candidate1.exists() {
        candidate1.display().to_string()
    } else {
        candidate2.display().to_string()
    };

    CheckResult {
        name: "system_prompt_file".to_string(),
        passed: found,
        detail: if found {
            format!("System prompt resolved: {}", resolved)
        } else {
            format!(
                "System prompt file '{}' not found (tried {} and {})",
                file_ref,
                candidate1.display(),
                candidate2.display()
            )
        },
    }
}

fn check_skill_path(from_skill: &str, skills_root: &Path) -> CheckResult {
    let skill_md = skills_root.join(from_skill).join("SKILL.md");
    let passed = skill_md.exists();
    CheckResult {
        name: format!("skill:{}", from_skill),
        passed,
        detail: if passed {
            format!("SKILL.md found: {}", skill_md.display())
        } else {
            format!("SKILL.md not found: {}", skill_md.display())
        },
    }
}

fn check_subagent_manifest(manifest_ref: &str, cookbook_dir: &Path) -> CheckResult {
    let path = cookbook_dir.join(manifest_ref);
    let passed = path.exists();
    CheckResult {
        name: format!("subagent:{}", manifest_ref),
        passed,
        detail: if passed {
            format!("Subagent manifest found: {}", path.display())
        } else {
            format!("Subagent manifest not found: {}", path.display())
        },
    }
}

fn check_steering_examples(cookbook_dir: &Path) -> CheckResult {
    let path = cookbook_dir.join("steering-examples.json");
    let passed = path.exists();
    CheckResult {
        name: "steering_examples".to_string(),
        passed,
        detail: if passed {
            "steering-examples.json exists".to_string()
        } else {
            format!("steering-examples.json not found at {}", path.display())
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_allowlist_passes_for_known_slugs() {
        assert!(check_slug_allowlist("equity-analyst").passed);
        assert!(check_slug_allowlist("private-markets-analyst").passed);
        assert!(check_slug_allowlist("credit-analyst").passed);
    }

    #[test]
    fn slug_allowlist_fails_for_unknown_slug() {
        let r = check_slug_allowlist("rogue-agent");
        assert!(!r.passed);
        assert!(r.detail.contains("not in the allowed slug list"));
    }

    #[test]
    fn required_fields_passes_when_all_present() {
        let m: AgentManifest = serde_json::from_str(
            r#"{"name":"x","model":"claude-opus-4-7","system":{"text":"hi"},"tools":[],"mcp_servers":[],"skills":[],"callable_agents":[]}"#,
        )
        .unwrap();
        assert!(check_required_fields(&m).passed);
    }

    #[test]
    fn required_fields_fails_on_empty_name() {
        let m: AgentManifest = serde_json::from_str(
            r#"{"name":"","model":"claude-opus-4-7","system":{"text":"hi"},"tools":[],"mcp_servers":[],"skills":[],"callable_agents":[]}"#,
        )
        .unwrap();
        let r = check_required_fields(&m);
        assert!(!r.passed);
        assert!(r.detail.contains("name"));
    }

    #[test]
    fn required_fields_fails_on_missing_system() {
        let m: AgentManifest = serde_json::from_str(
            r#"{"name":"x","model":"claude-opus-4-7","system":{},"tools":[],"mcp_servers":[],"skills":[],"callable_agents":[]}"#,
        )
        .unwrap();
        let r = check_required_fields(&m);
        assert!(!r.passed);
        assert!(r.detail.contains("system"));
    }

    #[test]
    fn manifest_parse_fails_for_nonexistent_file() {
        let (m, r) = check_manifest_parse(Path::new("/tmp/does_not_exist_cfa_test/agent.json"));
        assert!(m.is_none());
        assert!(!r.passed);
    }

    #[test]
    fn skill_path_fails_when_missing() {
        let r = check_skill_path("nonexistent-skill", Path::new("/tmp"));
        assert!(!r.passed);
        assert!(r.detail.contains("not found"));
    }

    #[test]
    fn subagent_manifest_fails_when_missing() {
        let r = check_subagent_manifest("./subagents/ghost.json", Path::new("/tmp"));
        assert!(!r.passed);
    }

    #[test]
    fn validate_all_errors_on_missing_root() {
        let input = CheckAllInput {
            cookbooks_root: "/tmp/cfa_no_such_dir_check_all".to_string(),
            skills_root: "/tmp".to_string(),
            agents_root: "/tmp".to_string(),
        };
        assert!(validate_all(&input).is_err());
    }

    #[test]
    fn validate_all_returns_empty_for_empty_root() {
        let tmp = std::env::temp_dir().join("cfa_check_all_empty");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let input = CheckAllInput {
            cookbooks_root: tmp.display().to_string(),
            skills_root: "/tmp".to_string(),
            agents_root: "/tmp".to_string(),
        };
        let out = validate_all(&input).unwrap();
        assert_eq!(out.total, 0);
        assert_eq!(out.passed, 0);
        assert_eq!(out.failed, 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
