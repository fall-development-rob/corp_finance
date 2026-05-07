//! Integration test for `cfa office docx write` (Phase 29 Wave 7).
//!
//! Exercises the happy path end-to-end with the real `cfa` binary:
//! 1. Writes a minimal WordDocSpec JSON to a tempfile.
//! 2. Invokes `cfa office docx write --spec <spec> --out <out>`.
//! 3. Asserts the .docx output exists, has nonzero size, and starts with the
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
  "sections": [
    {
      "blocks": [
        {
          "kind": "heading",
          "level": 1,
          "text": "Investment Committee Memo"
        },
        {
          "kind": "paragraph",
          "runs": [
            {
              "text": "This is the executive summary of the deal.",
              "bold": false,
              "italic": false
            }
          ]
        }
      ]
    }
  ]
}"#;

#[test]
fn office_docx_write_happy_path() {
    let dir = TempDir::new().expect("tempdir");
    let spec_path = dir.path().join("spec.json");
    let out_path = dir.path().join("output.docx");

    std::fs::write(&spec_path, TINY_SPEC).expect("write spec");

    let bin = cfa_binary();
    let output = Command::new(&bin)
        .args([
            "office",
            "docx",
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
        "cfa office docx write failed ({}): {}",
        output.status,
        stderr
    );

    // Assert .docx file exists and has nonzero size.
    assert!(out_path.exists(), ".docx output file must exist");
    let metadata = std::fs::metadata(&out_path).expect("metadata");
    assert!(metadata.len() > 0, ".docx file must have nonzero size");

    // Assert ZIP magic `PK\x03\x04` (docx is a zip container).
    let bytes = std::fs::read(&out_path).expect("read docx");
    assert!(
        bytes.starts_with(b"PK\x03\x04"),
        ".docx must start with ZIP magic PK\\x03\\x04"
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

    // Assert section_count is present and correct.
    let section_count = json["section_count"]
        .as_u64()
        .expect("section_count field must be a number");
    assert_eq!(section_count, 1, "section_count should match spec");
}
