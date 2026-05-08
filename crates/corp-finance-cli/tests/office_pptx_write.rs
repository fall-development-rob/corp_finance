//! Integration test for `cfa office pptx write` (Phase 29 Wave 8).
//!
//! Exercises the happy path end-to-end with the real `cfa` binary:
//! 1. Writes a minimal SlideDeckSpec JSON to a tempfile.
//! 2. Invokes `cfa office pptx write --spec <spec> --out <out>`.
//! 3. Asserts the .pptx output exists, has nonzero size, and starts with the
//!    ZIP local-file-header magic bytes `PK\x03\x04`.
//! 4. Asserts the stdout JSON is parseable and `sha256` is 64 hex chars.

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

static TINY_SPEC: &str = r#"{
  "slides": [
    {
      "kind": "title",
      "title": "Q1 2026 Investor Presentation",
      "subtitle": "Confidential"
    }
  ]
}"#;

#[test]
fn office_pptx_write_happy_path() {
    let dir = TempDir::new().expect("tempdir");
    let spec_path = dir.path().join("spec.json");
    let out_path = dir.path().join("output.pptx");

    std::fs::write(&spec_path, TINY_SPEC).expect("write spec");

    let bin = cfa_binary();
    let output = Command::new(&bin)
        .args([
            "office",
            "pptx",
            "write",
            "--spec",
            spec_path.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "cfa office pptx write failed ({}): {}",
        output.status,
        stderr
    );

    // Assert .pptx file exists and has nonzero size.
    assert!(out_path.exists(), ".pptx output file must exist");
    let metadata = std::fs::metadata(&out_path).expect("metadata");
    assert!(metadata.len() > 0, ".pptx file must have nonzero size");

    // Assert ZIP magic `PK\x03\x04` (pptx is a zip container).
    let bytes = std::fs::read(&out_path).expect("read pptx");
    assert!(
        bytes.starts_with(b"PK\x03\x04"),
        ".pptx must start with ZIP magic PK\\x03\\x04"
    );

    // Assert stdout is valid JSON with a 64-char lowercase hex sha256.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    let sha256 = json["sha256"]
        .as_str()
        .expect("sha256 field must be a string");
    assert_eq!(sha256.len(), 64, "sha256 must be 64 chars");
    assert!(
        sha256.chars().all(|c| c.is_ascii_hexdigit()),
        "sha256 must be lowercase hex"
    );

    // Assert slide_count is present and correct.
    let slide_count = json["slide_count"]
        .as_u64()
        .expect("slide_count field must be a number");
    assert_eq!(slide_count, 1, "slide_count should match spec");
}
