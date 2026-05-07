//! CLI subcommands for `cfa replay <verb>` (Phase 28 Wave 2).
//!
//! Two verbs:
//! - `cfa replay run --golden-set <PATH> --dispatcher cli --target <subcommand>`
//!   — load a [`GoldenSet`] from disk and replay every input through the
//!   spawned `./target/debug/cfa <target>` binary, capturing stdout and
//!   comparing the SHA-256 digest of the canonical JSON output against
//!   the manifest's `expected_digest`.
//! - `cfa replay verify --golden-set <PATH>` — restore the manifest and
//!   verify only the ed25519 signature; emits a compact verdict.
//!
//! Per ADR-020 / `RUF-LEARN-006`, replay is the load-bearing CI gate;
//! `cfa replay run` exits non-zero whenever any input fails so it can be
//! invoked directly from CI scripts.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::error::CorpFinanceError;
use corp_finance_core::self_learning::{restore_golden_set, run_replay};

/// Top-level arg group for the `replay` subcommand group.
#[derive(Args)]
pub struct ReplayArgs {
    #[command(subcommand)]
    pub command: ReplayCommands,
}

#[derive(Subcommand)]
pub enum ReplayCommands {
    /// Run a replay against a frozen [`GoldenSet`] using the supplied
    /// dispatcher.
    Run(ReplayRunArgs),
    /// Verify the ed25519 signature on a golden-set manifest without
    /// dispatching any inputs.
    Verify(ReplayVerifyArgs),
}

// ---------------------------------------------------------------------------
// ReplayRunArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ReplayRunArgs {
    /// Path to the golden-set manifest (`manifest.json`).
    #[arg(long = "golden-set")]
    pub golden_set: PathBuf,

    /// Dispatcher kind. v1 supports only `cli` — the input JSON is fed to
    /// the spawned `cfa <target>` binary on stdin and the captured stdout
    /// is the dispatcher output.
    #[arg(long, default_value = "cli")]
    pub dispatcher: String,

    /// Target CLI subcommand to invoke (e.g. `dcf`, `wacc`). Required for
    /// the `cli` dispatcher.
    #[arg(long)]
    pub target: Option<String>,

    /// Path to the `cfa` binary used by the dispatcher. Defaults to
    /// `./target/debug/cfa`.
    #[arg(long = "cfa-binary", default_value = "./target/debug/cfa")]
    pub cfa_binary: String,
}

pub fn run_replay_cmd(args: ReplayRunArgs) -> Result<Value, Box<dyn std::error::Error>> {
    if args.dispatcher != "cli" {
        return Err(format!(
            "unknown dispatcher '{}'; v1 supports only 'cli'",
            args.dispatcher
        )
        .into());
    }
    let target = args
        .target
        .as_ref()
        .ok_or("--target is required when --dispatcher cli")?
        .clone();
    if target.is_empty() {
        return Err("--target must be a non-empty CLI subcommand".into());
    }

    let golden_set = restore_golden_set(&args.golden_set).map_err(|e| {
        format!(
            "could not restore golden set '{}': {}",
            args.golden_set.display(),
            e
        )
    })?;

    let cfa_binary = args.cfa_binary.clone();
    let target_clone = target.clone();
    let result = run_replay(&golden_set, |input_json: &Value| {
        spawn_cli_dispatcher(&cfa_binary, &target_clone, input_json)
    })
    .map_err(|e| format!("run_replay error: {}", e))?;

    let payload = json!({
        "golden_set": args.golden_set.display().to_string(),
        "dispatcher": "cli",
        "target": target,
        "passed": result.passed,
        "failed": result.failed,
        "is_clean": result.is_clean(),
        "failures": result.failures.iter().map(|f| json!({
            "input_id": f.input_id.to_string(),
            "expected_digest": f.expected_digest,
            "actual_digest": f.actual_digest,
            "structural_delta": f.structural_delta,
        })).collect::<Vec<_>>(),
    });

    if !result.is_clean() {
        // Print the structured payload to stdout, then exit non-zero so CI
        // gates can rely on the exit code without parsing JSON.
        println!("{}", serde_json::to_string_pretty(&payload)?);
        std::process::exit(1);
    }

    Ok(payload)
}

/// Spawn `<cfa_binary> <target>` with `input_json` on stdin and return the
/// captured stdout parsed as JSON. The default CFA CLI emits JSON on
/// stdout for every subcommand (see `output::format_output`).
fn spawn_cli_dispatcher(
    cfa_binary: &str,
    target: &str,
    input_json: &Value,
) -> Result<Value, CorpFinanceError> {
    let payload = serde_json::to_vec(input_json).map_err(|e| {
        CorpFinanceError::SerializationError(format!("failed to serialise replay input: {e}"))
    })?;

    // Some CLI subcommands take JSON on stdin; some take flag args. The
    // common case for first-rev replay is stdin-fed JSON, which the
    // existing input parser already supports for several subcommands.
    // Future work: per-target argv expansion when stdin is not accepted.
    let mut child = Command::new(cfa_binary)
        .arg(target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            CorpFinanceError::SerializationError(format!(
                "failed to spawn '{} {}': {}",
                cfa_binary, target, e
            ))
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&payload).map_err(|e| {
            CorpFinanceError::SerializationError(format!(
                "failed to write stdin to dispatcher: {e}"
            ))
        })?;
    }

    let output = child.wait_with_output().map_err(|e| {
        CorpFinanceError::SerializationError(format!("dispatcher subprocess failed: {e}"))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(CorpFinanceError::FinancialImpossibility(format!(
            "dispatcher '{} {}' exited with status {}: {}",
            cfa_binary, target, output.status, stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str::<Value>(trimmed).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "dispatcher stdout is not valid JSON: {e}; raw='{}'",
            trimmed.chars().take(200).collect::<String>()
        ))
    })
}

// ---------------------------------------------------------------------------
// ReplayVerifyArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ReplayVerifyArgs {
    /// Path to the golden-set manifest (`manifest.json`).
    #[arg(long = "golden-set")]
    pub golden_set: PathBuf,
}

pub fn run_verify(args: ReplayVerifyArgs) -> Result<Value, Box<dyn std::error::Error>> {
    match restore_golden_set(&args.golden_set) {
        Ok(gs) => Ok(json!({
            "golden_set": args.golden_set.display().to_string(),
            "verdict": "ok",
            "surface": gs.surface.as_str(),
            "surface_event_id": gs.surface_event_id,
            "input_count": gs.inputs.len(),
            "expected_output_digest": gs.expected_output_digest,
            "signed_at": gs.signed_manifest.signed_at,
        })),
        Err(e) => Ok(json!({
            "golden_set": args.golden_set.display().to_string(),
            "verdict": "failed",
            "error": e.to_string(),
        })),
    }
}
