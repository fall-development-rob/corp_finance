//! Integration test for `cfa office render-template` (Phase 29 Wave 6, Task B3).
//!
//! Exercises the comps path end-to-end with the real `cfa` binary:
//! 1. Writes a minimal CompsOutput JSON to a tempfile.
//! 2. Invokes `cfa office render-template --kind comps --result <path>`.
//! 3. Parses stdout as WorkbookSpec JSON.
//! 4. Asserts sheet count is 2 (Peers + Summary).

#![cfg(feature = "office")]

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

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

/// A minimal CompsOutput serialised as JSON.
/// MultipleType variants serialise as PascalCase (no rename_all on the enum).
static COMPS_OUTPUT_JSON: &str = r#"{
  "multiple_statistics": [
    {
      "multiple_type": "EvEbitda",
      "values": [["PeerA", "10.0"], ["PeerB", "12.0"]],
      "mean": "11.0",
      "median": "11.0",
      "high": "12.0",
      "low": "10.0",
      "std_dev": "1.0",
      "count": 2
    }
  ],
  "implied_valuations": [],
  "companies_included": 2,
  "companies_excluded": 0
}"#;

#[test]
fn office_render_template_comps_happy_path() {
    let dir = TempDir::new().expect("tempdir");
    let result_path = dir.path().join("comps_output.json");

    std::fs::write(&result_path, COMPS_OUTPUT_JSON).expect("write comps output JSON");

    let bin = cfa_binary();
    let output = Command::new(&bin)
        .args([
            "office",
            "render-template",
            "--kind",
            "comps",
            "--result",
            result_path.to_str().unwrap(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "cfa office render-template failed ({}): {}",
        output.status,
        stderr
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid WorkbookSpec JSON");

    let sheets = json["sheets"]
        .as_array()
        .expect("WorkbookSpec must have a sheets array");
    assert_eq!(sheets.len(), 2, "comps template must produce 2 sheets (Peers + Summary)");
}
