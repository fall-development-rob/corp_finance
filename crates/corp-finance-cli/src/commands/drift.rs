//! CLI subcommands for `cfa drift <verb>` (Phase 28 Wave 2).
//!
//! One verb:
//! - `cfa drift check --baseline <PATH> --current <PATH> [--tolerance-pct 5.0]`
//!   — load two JSON files (baseline and current surface-event outputs)
//!   and call [`self_learning::detect_drift`]. Emits the resulting
//!   [`DriftReport`] as JSON. Exits with status 2 (non-zero, distinct from
//!   the generic error path) when the verdict is `BeyondTolerance` or
//!   `Critical` so plugin hooks and CI gates can branch on it; matches the
//!   convention used by `plugins/cfa-core/hooks/hooks.json` for the
//!   `pre-deploy-drift-check` hook.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::Value;

use corp_finance_core::self_learning::{detect_drift, types::DriftVerdict};

/// Top-level arg group for the `drift` subcommand group.
#[derive(Args)]
pub struct DriftArgs {
    #[command(subcommand)]
    pub command: DriftCommands,
}

#[derive(Subcommand)]
pub enum DriftCommands {
    /// Compare two JSON outputs and emit a drift report.
    Check(DriftCheckArgs),
}

// ---------------------------------------------------------------------------
// DriftCheckArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct DriftCheckArgs {
    /// Path to the baseline JSON file.
    #[arg(long)]
    pub baseline: PathBuf,

    /// Path to the current JSON file.
    #[arg(long)]
    pub current: PathBuf,

    /// Drift tolerance as a percentage (e.g. `5.0` means 5%). Drift
    /// strictly below this fraction is `WithinTolerance`; at or above is
    /// `BeyondTolerance`. Default per `RUF-LEARN-INV-008` is 5%.
    #[arg(long = "tolerance-pct", default_value_t = 5.0)]
    pub tolerance_pct: f64,
}

pub fn run_check(args: DriftCheckArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let baseline_bytes = std::fs::read(&args.baseline).map_err(|e| {
        format!(
            "could not read --baseline '{}': {}",
            args.baseline.display(),
            e
        )
    })?;
    let current_bytes = std::fs::read(&args.current).map_err(|e| {
        format!(
            "could not read --current '{}': {}",
            args.current.display(),
            e
        )
    })?;

    let baseline: Value = serde_json::from_slice(&baseline_bytes).map_err(|e| {
        format!(
            "could not parse baseline JSON '{}': {}",
            args.baseline.display(),
            e
        )
    })?;
    let current: Value = serde_json::from_slice(&current_bytes).map_err(|e| {
        format!(
            "could not parse current JSON '{}': {}",
            args.current.display(),
            e
        )
    })?;

    // The CLI argument is in percent (e.g. 5.0 == 5%); the core function
    // takes a fractional tolerance.
    let tolerance_fraction = args.tolerance_pct / 100.0;
    let report = detect_drift(&baseline, &current, tolerance_fraction);

    let payload = serde_json::to_value(&report)?;

    let blocking = matches!(
        report.verdict,
        DriftVerdict::BeyondTolerance { .. } | DriftVerdict::Critical { .. }
    );

    if blocking {
        // Emit the structured report on stdout, then exit with status 2 so
        // CI / plugin hooks can branch on the exit code without re-parsing.
        println!("{}", serde_json::to_string_pretty(&payload)?);
        std::process::exit(2);
    }

    Ok(payload)
}
