//! Payload assembly for managed-agent deployment.
//!
//! Pure function: reads the cookbook directory, performs environment-variable
//! substitution on `${VAR}` placeholders, inlines system prompts and skill
//! contents, and returns a structured JSON payload ready to be POSTed to
//! the Anthropic Managed Agents API by the caller's shell.
//!
//! No HTTP I/O — the consumer pipes the output to `curl`:
//! ```text
//! cfa managed-agent deploy equity-analyst \
//!   | jq '.orchestrator' \
//!   | curl -X POST https://api.anthropic.com/v1/agents \
//!       -H "x-api-key: $ANTHROPIC_API_KEY" \
//!       -H "anthropic-version: 2023-06-01" \
//!       -H "anthropic-beta: managed-agents-2026-05-06" \
//!       -H "Content-Type: application/json" -d @-
//! ```
//!
//! Pattern mirrors `crates/corp-finance-core/src/workflows/` module structure.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use crate::managed_agent::types::{
    AgentManifest, DeployInput, DeployOutput, SkillPayload, SubagentPayload,
};
use crate::CorpFinanceResult;

/// Build the full deployment payload for a managed agent slug.
///
/// Returned `DeployOutput` contains:
/// - `skill_payloads`: one entry per `skills[].from_skill`, with inlined SKILL.md content.
/// - `subagent_payloads`: one entry per `callable_agents[].manifest`, fully resolved JSON.
/// - `orchestrator_payload`: the orchestrator agent JSON with system prompt inlined.
pub fn build_deploy_payload(input: &DeployInput) -> CorpFinanceResult<DeployOutput> {
    let slug = &input.slug;
    let cookbook_dir = Path::new(&input.cookbooks_root).join(slug);
    let manifest_path = cookbook_dir.join("agent.json");

    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Cannot read {}: {}", manifest_path.display(), e))?;
    let substituted = env_substitute(&raw, &input.env_vars);
    let mut manifest: AgentManifest = serde_json::from_str(&substituted)
        .map_err(|e| format!("JSON parse error in {}: {}", manifest_path.display(), e))?;

    // Collect skill payloads
    let mut skill_payloads: Vec<SkillPayload> = Vec::new();
    for skill_ref in &manifest.skills {
        let skill_path = Path::new(&input.skills_root)
            .join(&skill_ref.from_skill)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&skill_path).unwrap_or_default();
        skill_payloads.push(SkillPayload {
            from_skill: skill_ref.from_skill.clone(),
            skill_path: skill_path.display().to_string(),
            content_length: content.len(),
        });
    }

    // Collect subagent payloads
    let mut subagent_payloads: Vec<SubagentPayload> = Vec::new();
    for ca in &manifest.callable_agents {
        let sub_path = cookbook_dir.join(&ca.manifest);
        let sub_raw = std::fs::read_to_string(&sub_path)
            .map_err(|e| format!("Cannot read subagent {}: {}", sub_path.display(), e))?;
        let sub_substituted = env_substitute(&sub_raw, &input.env_vars);
        let mut sub_value: Value = serde_json::from_str(&sub_substituted)
            .map_err(|e| format!("JSON parse error in {}: {}", sub_path.display(), e))?;
        inline_system_prompt(&mut sub_value, &cookbook_dir, Path::new(&input.agents_root))?;
        subagent_payloads.push(SubagentPayload {
            manifest_path: sub_path.display().to_string(),
            payload: sub_value,
        });
    }

    // Inline orchestrator system prompt
    let mut orchestrator_value =
        serde_json::to_value(&manifest).map_err(|e| format!("Serialisation error: {}", e))?;
    inline_system_prompt(
        &mut orchestrator_value,
        &cookbook_dir,
        Path::new(&input.agents_root),
    )?;

    // Patch callable_agents in orchestrator with placeholder agent_ids
    // (the shell / caller substitutes real IDs after POSTing each subagent)
    if let Value::Array(ref subs) = orchestrator_value["callable_agents"].clone() {
        let patched: Vec<Value> = subs
            .iter()
            .enumerate()
            .map(|(i, ca)| {
                let mut ca = ca.clone();
                if ca.get("agent_id").is_none() {
                    ca["agent_id"] = json!(format!("{{subagent_{}_id}}", i));
                }
                ca
            })
            .collect();
        orchestrator_value["callable_agents"] = Value::Array(patched);
    }

    // Normalise manifest.callable_agents with same placeholder agent_ids
    for (i, ca) in manifest.callable_agents.iter_mut().enumerate() {
        if ca.agent_id.is_none() {
            ca.agent_id = Some(format!("{{subagent_{}_id}}", i));
        }
    }

    Ok(DeployOutput {
        slug: slug.clone(),
        orchestrator_payload: orchestrator_value,
        subagent_payloads,
        skill_payloads,
        dry_run: true,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Substitute `${VAR}` placeholders using the provided map.
/// Unset variables are left as-is (not blank) so the caller can inspect them.
pub(crate) fn env_substitute(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (key, value) in vars {
        let placeholder = format!("${{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

/// Resolve and inline `system.file` → `system.text` inside a JSON manifest value.
fn inline_system_prompt(
    value: &mut Value,
    cookbook_dir: &Path,
    agents_root: &Path,
) -> CorpFinanceResult<()> {
    let file_ref = match value.pointer("/system/file").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => return Ok(()),
    };

    // Try cookbook-relative first, then agents_root basename fallback
    let candidate1 = cookbook_dir.join(&file_ref);
    let candidate2 = agents_root.join(
        Path::new(&file_ref)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(&file_ref)),
    );

    let system_text = if candidate1.exists() {
        std::fs::read_to_string(&candidate1)
            .map_err(|e| format!("Cannot read system prompt {}: {}", candidate1.display(), e))?
    } else if candidate2.exists() {
        std::fs::read_to_string(&candidate2)
            .map_err(|e| format!("Cannot read system prompt {}: {}", candidate2.display(), e))?
    } else {
        // File missing — leave a placeholder so caller knows what to supply
        format!("{{system_prompt_file_missing: {}}}", file_ref)
    };

    let append = value
        .pointer("/system/append")
        .and_then(Value::as_str)
        .map(|s| format!("\n\n{}", s))
        .unwrap_or_default();
    let combined = format!("{}{}", system_text, append);

    value["system"] = json!({ "text": combined });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_substitute_replaces_placeholders() {
        let mut vars = HashMap::new();
        vars.insert("FOO".to_string(), "bar".to_string());
        vars.insert("BAZ".to_string(), "qux".to_string());
        let result = env_substitute("${FOO} and ${BAZ}", &vars);
        assert_eq!(result, "bar and qux");
    }

    #[test]
    fn env_substitute_leaves_unset_vars() {
        let vars: HashMap<String, String> = HashMap::new();
        let result = env_substitute("${UNSET_VAR}", &vars);
        assert_eq!(result, "${UNSET_VAR}");
    }

    #[test]
    fn env_substitute_partial_replacement() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "1".to_string());
        let result = env_substitute("${A} ${B}", &vars);
        assert_eq!(result, "1 ${B}");
    }

    #[test]
    fn inline_system_prompt_noop_when_no_file_ref() {
        let mut v = json!({"system": {"text": "Hello"}});
        inline_system_prompt(&mut v, Path::new("/tmp"), Path::new("/tmp")).unwrap();
        assert_eq!(v["system"]["text"], "Hello");
    }

    #[test]
    fn inline_system_prompt_uses_placeholder_for_missing_file() {
        let mut v = json!({"system": {"file": "../../../.claude/agents/cfa/ghost.md"}});
        inline_system_prompt(
            &mut v,
            Path::new("/tmp/no_dir"),
            Path::new("/tmp/no_agents"),
        )
        .unwrap();
        let text = v["system"]["text"].as_str().unwrap();
        assert!(text.contains("system_prompt_file_missing"));
    }

    #[test]
    fn build_deploy_payload_fails_gracefully_on_missing_cookbook() {
        let input = DeployInput {
            slug: "equity-analyst".to_string(),
            cookbooks_root: "/tmp/no_such_cookbooks_dir".to_string(),
            skills_root: "/tmp/skills".to_string(),
            agents_root: "/tmp/agents".to_string(),
            env_vars: HashMap::new(),
        };
        let result = build_deploy_payload(&input);
        assert!(result.is_err());
    }
}
