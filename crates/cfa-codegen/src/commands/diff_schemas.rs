//! `cfa-codegen diff-schemas <a> <b>` — port of `scripts/diff-schemas.py`.
//!
//! Compares two `schemas.json` snapshots (loaded from a filesystem path or
//! a git ref via `git show`) and classifies the differences by semver
//! severity per the cfa-core schema versioning policy.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

const SCHEMAS_PATH: &str = "plugins/cfa-core/mcp/src/schemas.json";

/// Load a schemas.json snapshot from a filesystem path or git ref.
pub fn load(repo_root: &Path, reference: &str) -> Result<Value> {
    let p = PathBuf::from(reference);
    if p.exists() {
        let text = std::fs::read_to_string(&p)
            .with_context(|| format!("read {}", p.display()))?;
        return serde_json::from_str(&text)
            .with_context(|| format!("parse {}", p.display()));
    }
    // Treat as git ref.
    let output = Command::new("git")
        .arg("show")
        .arg(format!("{}:{}", reference, SCHEMAS_PATH))
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("run git show {}:{}", reference, SCHEMAS_PATH))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "could not read {} from {}: {}",
            SCHEMAS_PATH,
            reference,
            stderr
        ));
    }
    let text = String::from_utf8(output.stdout)?;
    serde_json::from_str(&text).with_context(|| format!("parse {} from {}", SCHEMAS_PATH, reference))
}

#[derive(Default)]
pub struct Classified {
    pub major: Vec<String>,
    pub minor: Vec<String>,
    pub patch: Vec<String>,
}

impl Classified {
    pub fn is_empty(&self) -> bool {
        self.major.is_empty() && self.minor.is_empty() && self.patch.is_empty()
    }
}

pub fn classify(a: &Value, b: &Value) -> Classified {
    let mut out = Classified::default();

    let a_tools: std::collections::BTreeSet<String> = a
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let b_tools: std::collections::BTreeSet<String> = b
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let added: Vec<&String> = b_tools.difference(&a_tools).collect();
    let removed: Vec<&String> = a_tools.difference(&b_tools).collect();
    if !added.is_empty() {
        let sample = sample(&added);
        out.minor
            .push(format!("+{} tools (e.g. {})", added.len(), sample));
    }
    if !removed.is_empty() {
        let sample = sample(&removed);
        out.major.push(format!(
            "−{} tools removed (e.g. {})",
            removed.len(),
            sample
        ));
    }

    let a_missing = missing_tool_set(a);
    let b_missing = missing_tool_set(b);
    let newly_passthrough: Vec<&String> = b_missing.difference(&a_missing).collect();
    let newly_typed: Vec<&String> = a_missing.difference(&b_missing).collect();
    if !newly_passthrough.is_empty() {
        let names: Vec<String> = newly_passthrough.iter().map(|s| (*s).clone()).collect();
        out.major.push(format!(
            "became passthrough: {} — regression: a previously-typed schema lost its types",
            names.join(", ")
        ));
    }
    if !newly_typed.is_empty() {
        let names: Vec<String> = newly_typed.iter().map(|s| (*s).clone()).collect();
        out.minor.push(format!("became typed: {}", names.join(", ")));
    }

    let a_ver = a.get("schema_version").and_then(|v| v.as_str()).unwrap_or("?");
    let b_ver = b.get("schema_version").and_then(|v| v.as_str()).unwrap_or("?");
    if a_ver != b_ver {
        out.patch
            .push(format!("schema_version field: {} → {}", a_ver, b_ver));
    }

    out
}

fn missing_tool_set(snapshot: &Value) -> std::collections::BTreeSet<String> {
    snapshot
        .get("missing")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| entry.get("tool").and_then(|t| t.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn sample(items: &[&String]) -> String {
    let head: Vec<&str> = items.iter().take(5).map(|s| s.as_str()).collect();
    let mut s = head.join(", ");
    if items.len() > 5 {
        s.push_str(" ...");
    }
    s
}

pub fn run(repo_root: &Path, ref_a: &str, ref_b: &str) -> Result<i32> {
    let a = load(repo_root, ref_a)?;
    let b = load(repo_root, ref_b)?;

    println!("Comparing {} → {}", ref_a, ref_b);
    println!();

    let classified = classify(&a, &b);
    if classified.is_empty() {
        println!("No structural changes detected.");
        return Ok(0);
    }

    for (label, reasons) in [
        ("MAJOR", &classified.major),
        ("MINOR", &classified.minor),
        ("PATCH", &classified.patch),
    ] {
        if reasons.is_empty() {
            continue;
        }
        println!("## {}", label);
        for r in reasons {
            println!("  - {}", r);
        }
        println!();
    }

    if !classified.major.is_empty() {
        println!("Recommended schema_version bump: MAJOR");
    } else if !classified.minor.is_empty() {
        println!("Recommended schema_version bump: MINOR");
    } else {
        println!("Recommended schema_version bump: PATCH");
    }
    Ok(0)
}
