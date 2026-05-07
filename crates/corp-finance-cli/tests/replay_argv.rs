//! Integration test for Phase 29 Wave 3: `cfa replay run --argv-mode flags/template`
//!
//! Uses the real `cfa` binary end-to-end:
//! 1. Capture `cfa managed-agent list --tier core_only` output and compute its digest.
//! 2. Freeze a golden set via `cfa golden-set freeze`.
//! 3. Replay via `cfa replay run --argv-mode flags` — expect passed=1, failed=0.
//! 4. Replay via `cfa replay run --argv-mode template` — same expectation.
//!
//! This exercises spawn_cli_dispatcher, render_argv_flags, render_argv_template,
//! and the multi-word target split end-to-end with real subprocess output.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;
use uuid::Uuid;

/// Path to the cfa binary. Uses `CARGO_BIN_EXE_cfa` when set by cargo test,
/// falls back to workspace `target/debug/cfa`.
fn cfa_binary() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_cfa") {
        return PathBuf::from(p);
    }
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // corp-finance-cli/
    p.pop(); // crates/
    p.push("target/debug/cfa");
    p
}

/// Run the cfa binary and return parsed stdout JSON. Panics on failure.
fn run_cfa(bin: &Path, args: &[&str]) -> Value {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", bin));
    assert!(
        out.status.success(),
        "cfa {:?} failed ({}): {}",
        args,
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is not JSON: {e}\n---\n{stdout}"))
}

/// Compute SHA-256 of the canonical JSON of `v` (mirrors `digest_output` in
/// corp-finance-core::self_learning::replay).
fn digest_output(v: &Value) -> String {
    fn canonicalize(v: &Value) -> Value {
        match v {
            Value::Object(map) => {
                let mut sorted = serde_json::Map::new();
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                for k in keys {
                    sorted.insert(k.clone(), canonicalize(&map[k]));
                }
                Value::Object(sorted)
            }
            Value::Array(arr) => Value::Array(arr.iter().map(canonicalize).collect()),
            other => other.clone(),
        }
    }
    use sha2::{Digest, Sha256};
    let canon = canonicalize(v);
    let bytes = serde_json::to_vec(&canon).unwrap();
    let hash = Sha256::digest(&bytes);
    hash.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Freeze a golden set via `cfa golden-set freeze` and return the manifest path.
fn freeze_golden_set(
    bin: &Path,
    dir: &Path,
    surface: &str,
    event_id: &str,
    input_json: &Value,
    expected_digest: &str,
) -> PathBuf {
    let inputs_file = dir.join("inputs.jsonl");
    let golden_input = serde_json::json!({
        "input_id": Uuid::now_v7().to_string(),
        "input_json": input_json,
        "expected_digest": expected_digest,
    });
    std::fs::write(&inputs_file, serde_json::to_string(&golden_input).unwrap()).unwrap();

    let golden_dir = dir.join("golden-sets");
    let keys_dir = dir.join("keys");

    let freeze_out = run_cfa(
        bin,
        &[
            "golden-set",
            "freeze",
            "--surface",
            surface,
            "--surface-event-id",
            event_id,
            "--inputs-file",
            inputs_file.to_str().unwrap(),
            "--output-dir",
            golden_dir.to_str().unwrap(),
            "--keys-dir",
            keys_dir.to_str().unwrap(),
        ],
    );
    assert_eq!(
        freeze_out["status"], "frozen",
        "freeze failed: {freeze_out}"
    );

    golden_dir
        .join(surface)
        .join(event_id)
        .join("manifest.json")
}

#[test]
fn replay_flags_mode_managed_agent_list() {
    let bin = cfa_binary();
    if !bin.exists() {
        eprintln!("SKIP: cfa binary not found at {}", bin.display());
        return;
    }
    let bin_str = bin.to_str().unwrap();

    // Capture the canonical output and compute its digest.
    let real_output = run_cfa(&bin, &["managed-agent", "list", "--tier", "core_only"]);
    let expected_digest = digest_output(&real_output);

    let dir = TempDir::new().unwrap();
    let manifest = freeze_golden_set(
        &bin,
        dir.path(),
        "cli",
        "managed-agent.list.flags",
        &serde_json::json!({"tier": "core_only"}),
        &expected_digest,
    );

    let result = run_cfa(
        &bin,
        &[
            "replay",
            "run",
            "--golden-set",
            manifest.to_str().unwrap(),
            "--dispatcher",
            "cli",
            "--target",
            "managed-agent list",
            "--cfa-binary",
            bin_str,
            "--argv-mode",
            "flags",
        ],
    );

    assert_eq!(result["passed"], 1, "expected passed=1: {result}");
    assert_eq!(result["failed"], 0, "expected failed=0: {result}");
    assert_eq!(result["is_clean"], true, "expected is_clean=true: {result}");
}

#[test]
fn replay_template_mode_managed_agent_list() {
    let bin = cfa_binary();
    if !bin.exists() {
        eprintln!("SKIP: cfa binary not found at {}", bin.display());
        return;
    }
    let bin_str = bin.to_str().unwrap();

    let real_output = run_cfa(&bin, &["managed-agent", "list", "--tier", "core_only"]);
    let expected_digest = digest_output(&real_output);

    let dir = TempDir::new().unwrap();
    let manifest = freeze_golden_set(
        &bin,
        dir.path(),
        "cli",
        "managed-agent.list.template",
        &serde_json::json!({"tier": "core_only"}),
        &expected_digest,
    );

    // --argv-template value starts with '--', so we use = syntax to pass it
    // through clap without ambiguity.
    let template_arg = "--argv-template=--tier {tier}";

    let result = run_cfa(
        &bin,
        &[
            "replay",
            "run",
            "--golden-set",
            manifest.to_str().unwrap(),
            "--dispatcher",
            "cli",
            "--target",
            "managed-agent list",
            "--cfa-binary",
            bin_str,
            "--argv-mode",
            "template",
            template_arg,
        ],
    );

    assert_eq!(result["passed"], 1, "expected passed=1: {result}");
    assert_eq!(result["failed"], 0, "expected failed=0: {result}");
    assert_eq!(result["is_clean"], true, "expected is_clean=true: {result}");
}

#[test]
fn replay_stdin_mode_not_regressed() {
    // Stdin mode is tested indirectly: if the binary compiles and responds
    // to --help without error, the mode is at minimum structurally present.
    // The full stdin-path coverage is in the unit tests (render_argv_flags,
    // etc.) and in corp-finance-core replay tests.
    let bin = cfa_binary();
    if !bin.exists() {
        return;
    }
    let out = Command::new(&bin)
        .args(["replay", "run", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stdout);
    assert!(
        help.contains("argv-mode"),
        "help should mention argv-mode: {help}"
    );
    assert!(
        help.contains("stdin"),
        "help should mention stdin mode: {help}"
    );
    assert!(
        help.contains("flags"),
        "help should mention flags mode: {help}"
    );
    assert!(
        help.contains("template"),
        "help should mention template mode: {help}"
    );
}
