//! CLI subcommands for `cfa golden-set <verb>` (Phase 28 Wave 2).
//!
//! Three verbs:
//! - `cfa golden-set freeze --surface <s> --surface-event-id <id>
//!   --inputs-file <path> --output-dir <path>` — read a JSONL of golden
//!   inputs and freeze them into a signed [`GoldenSet`] manifest on disk.
//!   Default signing key path is `<repo>/var/golden-sets/keys/`; the
//!   keypair is auto-generated on first run via [`ensure_keypair`].
//! - `cfa golden-set restore --path <PATH>` — restore a manifest from
//!   disk and verify its ed25519 signature.
//! - `cfa golden-set list [--root <PATH>]` — walk the on-disk root and
//!   list every verified golden set.
//!
//! Per ADR-020 §"Replay-driven contract tests", manifests are immutable
//! post-acceptance and the signature is the load-bearing CI gate.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{json, Value};
use uuid::Uuid;

use corp_finance_core::self_learning::{
    ensure_keypair, freeze_golden_set, list_golden_sets, restore_golden_set, types::GoldenInput,
};
use corp_finance_core::surface::Surface;

/// Top-level arg group for the `golden-set` subcommand group.
#[derive(Args)]
pub struct GoldenSetArgs {
    #[command(subcommand)]
    pub command: GoldenSetCommands,
}

#[derive(Subcommand)]
pub enum GoldenSetCommands {
    /// Freeze a JSONL of inputs into a signed manifest on disk.
    Freeze(GoldenSetFreezeArgs),
    /// Restore a manifest from disk and verify the signature.
    Restore(GoldenSetRestoreArgs),
    /// List every verified golden set under a root directory.
    List(GoldenSetListArgs),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_surface(s: &str) -> Result<Surface, Box<dyn std::error::Error>> {
    Surface::parse(s).ok_or_else(|| {
        format!(
            "unknown surface '{}'; expected cli | mcp | skill | plugin",
            s
        )
        .into()
    })
}

fn default_root() -> PathBuf {
    let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    p.push("var");
    p.push("golden-sets");
    p
}

fn default_keys_dir() -> PathBuf {
    let mut p = default_root();
    p.push("keys");
    p
}

// ---------------------------------------------------------------------------
// GoldenSetFreezeArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct GoldenSetFreezeArgs {
    /// Surface that produced the golden inputs.
    #[arg(long)]
    pub surface: String,

    /// Surface-event identifier (CLI subcommand, MCP tool name, etc.).
    #[arg(long = "surface-event-id")]
    pub surface_event_id: String,

    /// Path to a JSONL file. Each line is one of:
    /// - a full `GoldenInput` (with `input_id`, `input_json`, `expected_digest`).
    /// - an object with `input_json` + `expected_digest` (and optional
    ///   `input_id`); a fresh UUIDv7 is allocated when missing.
    /// - a bare object (treated as `input_json`); `expected_digest` defaults
    ///   to the empty string and must be filled in by the caller before
    ///   freezing in production.
    #[arg(long = "inputs-file")]
    pub inputs_file: PathBuf,

    /// Output directory under which `<surface>/<surface_event_id>/` is
    /// created. Defaults to `<repo>/var/golden-sets`.
    #[arg(long = "output-dir")]
    pub output_dir: Option<PathBuf>,

    /// Directory containing `signing.key` + `verifying.key`. The keypair
    /// is auto-generated on first run if missing. Defaults to
    /// `<repo>/var/golden-sets/keys`.
    #[arg(long = "keys-dir")]
    pub keys_dir: Option<PathBuf>,
}

pub fn run_freeze(args: GoldenSetFreezeArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let surface = parse_surface(&args.surface)?;
    if args.surface_event_id.is_empty() {
        return Err("--surface-event-id must be non-empty".into());
    }

    let bytes = std::fs::read(&args.inputs_file).map_err(|e| {
        format!(
            "could not read --inputs-file '{}': {}",
            args.inputs_file.display(),
            e
        )
    })?;
    let text = String::from_utf8_lossy(&bytes);

    let mut inputs: Vec<GoldenInput> = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|e| {
            format!(
                "could not parse JSONL line {} of '{}': {}",
                lineno + 1,
                args.inputs_file.display(),
                e
            )
        })?;
        inputs.push(coerce_input(v, lineno + 1)?);
    }
    if inputs.is_empty() {
        return Err(format!(
            "--inputs-file '{}' contained no inputs",
            args.inputs_file.display()
        )
        .into());
    }

    let output_dir = args.output_dir.unwrap_or_else(default_root);
    let keys_dir = args.keys_dir.unwrap_or_else(default_keys_dir);
    let (signing_key, _verifying_key) = ensure_keypair(&keys_dir).map_err(|e| {
        format!(
            "could not ensure keypair in '{}': {}",
            keys_dir.display(),
            e
        )
    })?;

    let golden_set = freeze_golden_set(
        surface,
        &args.surface_event_id,
        inputs,
        &output_dir,
        &signing_key,
    )
    .map_err(|e| format!("freeze_golden_set error: {}", e))?;

    let manifest_path = output_dir
        .join(surface.as_str())
        .join(&args.surface_event_id)
        .join("manifest.json");

    Ok(json!({
        "status": "frozen",
        "manifest_path": manifest_path.display().to_string(),
        "surface": golden_set.surface.as_str(),
        "surface_event_id": golden_set.surface_event_id,
        "input_count": golden_set.inputs.len(),
        "expected_output_digest": golden_set.expected_output_digest,
        "signed_at": golden_set.signed_manifest.signed_at,
        "public_key": golden_set.signed_manifest.public_key,
    }))
}

/// Coerce a JSONL line into a [`GoldenInput`]. Accepts the three documented
/// shapes; defaults `input_id` to a fresh UUIDv7 when missing and
/// `expected_digest` to the empty string (caller must fill in before
/// production replay).
fn coerce_input(v: Value, lineno: usize) -> Result<GoldenInput, Box<dyn std::error::Error>> {
    if let Ok(g) = serde_json::from_value::<GoldenInput>(v.clone()) {
        return Ok(g);
    }
    let obj = v
        .as_object()
        .ok_or_else(|| format!("line {} is not a JSON object", lineno))?;

    let input_id = match obj.get("input_id") {
        Some(Value::String(s)) => Uuid::parse_str(s)
            .map_err(|e| format!("line {} has invalid input_id '{}': {}", lineno, s, e))?,
        _ => Uuid::now_v7(),
    };
    let expected_digest = match obj.get("expected_digest") {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let input_json = obj
        .get("input_json")
        .cloned()
        .unwrap_or_else(|| Value::Object(obj.clone()));

    Ok(GoldenInput {
        input_id,
        input_json,
        expected_digest,
    })
}

// ---------------------------------------------------------------------------
// GoldenSetRestoreArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct GoldenSetRestoreArgs {
    /// Path to the `manifest.json` file to restore.
    #[arg(long)]
    pub path: PathBuf,
}

pub fn run_restore(args: GoldenSetRestoreArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let golden_set = restore_golden_set(&args.path).map_err(|e| {
        format!(
            "restore_golden_set error for '{}': {}",
            args.path.display(),
            e
        )
    })?;
    Ok(serde_json::to_value(&golden_set)?)
}

// ---------------------------------------------------------------------------
// GoldenSetListArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct GoldenSetListArgs {
    /// Root directory containing `<surface>/<surface_event_id>/manifest.json`
    /// trees. Defaults to `<repo>/var/golden-sets`.
    #[arg(long)]
    pub root: Option<PathBuf>,
}

pub fn run_list(args: GoldenSetListArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let root = args.root.unwrap_or_else(default_root);
    if !root.exists() {
        return Ok(json!({
            "root": root.display().to_string(),
            "count": 0,
            "golden_sets": [],
            "note": "root directory does not exist",
        }));
    }

    let golden_sets = list_golden_sets(&root)
        .map_err(|e| format!("list_golden_sets error for '{}': {}", root.display(), e))?;

    let summaries: Vec<Value> = golden_sets
        .iter()
        .map(|gs| {
            json!({
                "surface": gs.surface.as_str(),
                "surface_event_id": gs.surface_event_id,
                "input_count": gs.inputs.len(),
                "expected_output_digest": gs.expected_output_digest,
                "signed_at": gs.signed_manifest.signed_at,
                "public_key": gs.signed_manifest.public_key,
            })
        })
        .collect();

    Ok(json!({
        "root": root.display().to_string(),
        "count": summaries.len(),
        "golden_sets": summaries,
    }))
}
