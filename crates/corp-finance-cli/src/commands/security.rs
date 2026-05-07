//! CLI subcommands for `cfa security <verb>` (Phase 26).
//!
//! Single verb in this slice:
//! - `cfa security scan --for <path>` — run the native PII scanner +
//!   prompt-injection detector over a file, print findings as JSON.
//!
//! Three optional `--action` modes:
//! - `alert-only` (default) — print findings, exit 0 regardless.
//! - `block-on-injection` — exit 2 on any Critical injection finding (matches
//!   the plugin hook convention).
//! - `redact` — write a sidecar redacted file at `<input>.redacted.<ext>`
//!   using the per-finding `redaction_proposal`. Exit 0.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::security::{detect_injection, scan_for_pii, Finding, FindingKind, Severity};

/// Top-level arg group for the `security` subcommand group.
#[derive(Args)]
pub struct SecurityArgs {
    #[command(subcommand)]
    pub command: SecurityCommands,
}

#[derive(Subcommand)]
pub enum SecurityCommands {
    /// Scan a file for PII and prompt-injection findings.
    Scan(SecurityScanArgs),
}

// ---------------------------------------------------------------------------
// SecurityScanArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct SecurityScanArgs {
    /// File to scan.
    #[arg(long = "for")]
    pub for_path: String,

    /// Action to take on findings: alert-only | block-on-injection | redact.
    #[arg(long, default_value = "alert-only")]
    pub action: String,

    /// Hook point label (informational; surfaces in the JSON output).
    #[arg(long)]
    pub hook_point: Option<String>,
}

fn redacted_sidecar_path(input: &std::path::Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("input");
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");
    let parent = input.parent().unwrap_or_else(|| std::path::Path::new("."));
    let new_name = if ext.is_empty() {
        format!("{}.redacted", stem)
    } else {
        format!("{}.redacted.{}", stem, ext)
    };
    parent.join(new_name)
}

fn build_redacted_text(text: &str, findings: &[Finding]) -> String {
    // Apply PII redactions in reverse byte-offset order so earlier offsets
    // remain valid after later splices.
    let mut out = text.to_string();
    let mut pii: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.kind == FindingKind::Pii && f.redaction_proposal.is_some())
        .collect();
    pii.sort_by(|a, b| b.span_start.cmp(&a.span_start));
    for f in pii {
        if f.span_start > out.len() || f.span_end > out.len() || f.span_start > f.span_end {
            continue;
        }
        if let Some(replacement) = &f.redaction_proposal {
            out.replace_range(f.span_start..f.span_end, replacement);
        }
    }
    out
}

pub fn run_scan(args: SecurityScanArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let path = PathBuf::from(&args.for_path);
    let bytes = std::fs::read(&path)?;
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(scan_for_pii(&text));
    findings.extend(detect_injection(&text));
    findings.sort_by_key(|f| f.span_start);

    let critical_injection = findings
        .iter()
        .any(|f| f.kind == FindingKind::PromptInjection && f.severity == Severity::Critical);

    let action = args.action.to_ascii_lowercase();
    match action.as_str() {
        "alert-only" => {}
        "block-on-injection" => {
            if critical_injection {
                eprintln!(
                    "[cfa] security scan blocked: critical prompt-injection finding(s) in {}",
                    path.display()
                );
                let payload = json!({
                    "status": "blocked",
                    "reason": "critical_prompt_injection",
                    "hook_point": args.hook_point,
                    "findings": findings,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                std::process::exit(2);
            }
        }
        "redact" => {
            let redacted = build_redacted_text(&text, &findings);
            let dest = redacted_sidecar_path(&path);
            std::fs::write(&dest, redacted)?;
            return Ok(json!({
                "status": "redacted",
                "redacted_path": dest.display().to_string(),
                "hook_point": args.hook_point,
                "findings": findings,
            }));
        }
        other => {
            return Err(format!(
                "unknown --action '{}'; expected alert-only | block-on-injection | redact",
                other
            )
            .into())
        }
    }

    Ok(json!({
        "status": "ok",
        "hook_point": args.hook_point,
        "findings": findings,
    }))
}
