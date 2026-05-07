//! Golden-set freeze + restore for replay-driven contract tests.
//!
//! Per ADR-020 §"What we build ourselves" and `RUF-LEARN-INV-006`, the
//! on-disk layout is:
//!
//! ```text
//! <root>/
//!   <surface>/
//!     <surface_event_id>/
//!       manifest.json          # serialised GoldenSet (signed)
//!       inputs/
//!         <input_id>.json      # one fixture per GoldenInput
//! ```
//!
//! Freeze is a one-shot operation invoked at acceptance; the resulting
//! manifest is immutable post-write. Restore reads the manifest from
//! disk, then verifies the ed25519 signature via
//! [`crate::self_learning::signing::verify_manifest`] before returning
//! the [`GoldenSet`].

use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

use crate::error::CorpFinanceError;
use crate::self_learning::signing::{sign_manifest, verify_manifest};
use crate::self_learning::types::{GoldenInput, GoldenSet};
use crate::surface::Surface;
use crate::CorpFinanceResult;

/// Compute the manifest content hash from the input digests.
///
/// Format: `sha256(<digest_1><digest_2>...<digest_n>)` where each
/// `<digest_i>` is the lowercase-hex SHA-256 from
/// [`GoldenInput::expected_digest`]. Concatenation order follows the
/// `inputs` slice as supplied by the caller.
fn content_hash_from_inputs(inputs: &[GoldenInput]) -> String {
    let mut hasher = Sha256::new();
    for i in inputs {
        hasher.update(i.expected_digest.as_bytes());
    }
    let h = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in h {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Build the on-disk directory for a single (`surface`, `surface_event_id`)
/// golden set.
fn surface_dir(root: &Path, surface: Surface, surface_event_id: &str) -> PathBuf {
    root.join(surface.as_str()).join(surface_event_id)
}

/// Freeze a golden set to disk.
///
/// 1. Computes the manifest content hash from `inputs`.
/// 2. Signs the content hash via [`sign_manifest`] with `signing_key`.
/// 3. Writes `<output_dir>/<surface>/<surface_event_id>/manifest.json`.
/// 4. Writes one fixture per input under `inputs/<input_id>.json`.
///
/// Returns the freshly-built [`GoldenSet`].
pub fn freeze_golden_set(
    surface: Surface,
    surface_event_id: &str,
    inputs: Vec<GoldenInput>,
    output_dir: &Path,
    signing_key: &SigningKey,
) -> CorpFinanceResult<GoldenSet> {
    if surface_event_id.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "surface_event_id".into(),
            reason: "non-empty".into(),
        });
    }
    let content_hash = content_hash_from_inputs(&inputs);
    let signed = sign_manifest(&content_hash, signing_key);

    let dir = surface_dir(output_dir, surface, surface_event_id);
    let inputs_dir = dir.join("inputs");
    fs::create_dir_all(&inputs_dir).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "failed to create {}: {e}",
            inputs_dir.display()
        ))
    })?;
    for i in &inputs {
        let p = inputs_dir.join(format!("{}.json", i.input_id));
        let body = serde_json::to_vec_pretty(&i)?;
        fs::write(&p, body).map_err(|e| {
            CorpFinanceError::SerializationError(format!("failed to write {}: {e}", p.display()))
        })?;
    }

    let golden_set = GoldenSet {
        surface,
        surface_event_id: surface_event_id.to_string(),
        inputs,
        expected_output_digest: content_hash,
        signed_manifest: signed,
    };
    let manifest_path = dir.join("manifest.json");
    let body = serde_json::to_vec_pretty(&golden_set)?;
    fs::write(&manifest_path, body).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "failed to write {}: {e}",
            manifest_path.display()
        ))
    })?;
    Ok(golden_set)
}

/// Restore a golden set from disk and verify its manifest signature.
///
/// `path` is the path to the `manifest.json` file (not the parent
/// directory). Returns [`CorpFinanceError::InvalidInput`] when the
/// signature does not verify.
pub fn restore_golden_set(path: &Path) -> CorpFinanceResult<GoldenSet> {
    let body = fs::read(path).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "failed to read manifest {}: {e}",
            path.display()
        ))
    })?;
    let manifest: GoldenSet = serde_json::from_slice(&body)?;
    let ok = verify_manifest(&manifest.signed_manifest)?;
    if !ok {
        return Err(CorpFinanceError::InvalidInput {
            field: "signature".into(),
            reason: format!(
                "ed25519 signature verification failed for {}",
                path.display()
            ),
        });
    }
    Ok(manifest)
}

/// Walk `<root>/<surface>/<surface_event_id>/manifest.json` paths under
/// `root` and return a [`GoldenSet`] for each verifiable manifest.
///
/// Skips directories whose manifest does not exist or fails to parse;
/// returns an error for any signature-verification failure (this is
/// the load-bearing CI invariant).
pub fn list_golden_sets(root: &Path) -> CorpFinanceResult<Vec<GoldenSet>> {
    let mut out: Vec<GoldenSet> = Vec::new();
    let surfaces = match fs::read_dir(root) {
        Ok(rd) => rd,
        Err(_) => return Ok(out),
    };
    for surface_entry in surfaces.flatten() {
        let surface_path = surface_entry.path();
        if !surface_path.is_dir() {
            continue;
        }
        let events = match fs::read_dir(&surface_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for ev in events.flatten() {
            let ev_path = ev.path();
            if !ev_path.is_dir() {
                continue;
            }
            let manifest_path = ev_path.join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            let gs = restore_golden_set(&manifest_path)?;
            out.push(gs);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_learning::signing::generate_keypair;
    use serde_json::json;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn sample_input() -> GoldenInput {
        GoldenInput {
            input_id: Uuid::now_v7(),
            input_json: json!({"ticker": "AAPL"}),
            expected_digest: "abc123".into(),
        }
    }

    #[test]
    fn freeze_then_restore_round_trip() {
        let dir = TempDir::new().unwrap();
        let (sk, _) = generate_keypair().unwrap();
        let inputs = vec![sample_input(), sample_input()];
        let gs =
            freeze_golden_set(Surface::Mcp, "test_tool", inputs.clone(), dir.path(), &sk).unwrap();
        let manifest_path = dir.path().join("mcp/test_tool/manifest.json");
        let restored = restore_golden_set(&manifest_path).unwrap();
        assert_eq!(restored.surface, Surface::Mcp);
        assert_eq!(restored.surface_event_id, "test_tool");
        assert_eq!(restored.inputs.len(), 2);
        assert_eq!(restored.expected_output_digest, gs.expected_output_digest);
    }

    #[test]
    fn restore_rejects_tampered_manifest() {
        let dir = TempDir::new().unwrap();
        let (sk, _) = generate_keypair().unwrap();
        freeze_golden_set(Surface::Cli, "evt", vec![sample_input()], dir.path(), &sk).unwrap();
        let manifest_path = dir.path().join("cli/evt/manifest.json");
        let body = fs::read_to_string(&manifest_path).unwrap();
        // Mutate the content_hash so signature no longer matches.
        let tampered = body.replace(
            "\"content_hash\":",
            "\"content_hash\":\"deadbeef\",\"_was\":",
        );
        fs::write(&manifest_path, tampered).unwrap();
        // Re-read into JSON and inject a real-shape tampered hash.
        let mut v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        v["signed_manifest"]["content_hash"] = serde_json::Value::String("ff".into());
        fs::write(&manifest_path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
        let err = restore_golden_set(&manifest_path).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn list_finds_multiple_golden_sets() {
        let dir = TempDir::new().unwrap();
        let (sk, _) = generate_keypair().unwrap();
        freeze_golden_set(Surface::Cli, "a", vec![sample_input()], dir.path(), &sk).unwrap();
        freeze_golden_set(Surface::Mcp, "b", vec![sample_input()], dir.path(), &sk).unwrap();
        let all = list_golden_sets(dir.path()).unwrap();
        assert_eq!(all.len(), 2);
    }
}
