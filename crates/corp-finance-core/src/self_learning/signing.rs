//! ed25519 manifest signing for [`crate::self_learning::types::GoldenSet`]
//! manifests.
//!
//! Uses the `ed25519-dalek` crate for signing and verification per
//! ADR-020 §"Replay-driven contract tests" and `RUF-LEARN-INV-007`.
//!
//! ## Key persistence
//!
//! Keys are stored as raw bytes:
//!
//! - `signing.key` — 32-byte secret key
//! - `verifying.key` — 32-byte public key
//!
//! On first run, [`ensure_keypair`] generates a fresh keypair via
//! `getrandom` and persists it to disk; subsequent runs read it back.
//! The expected installation path is `<repo>/var/golden-sets/keys/`.

use std::fs;
use std::path::Path;

use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::error::CorpFinanceError;
use crate::self_learning::types::SignedManifest;
use crate::CorpFinanceResult;

/// Encode a byte slice as lowercase hex.
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Decode a lowercase or uppercase hex string into a byte vector.
fn hex_decode(s: &str) -> CorpFinanceResult<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return Err(CorpFinanceError::InvalidInput {
            field: "hex".into(),
            reason: "odd-length hex string".into(),
        });
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let chunk = &s[i..i + 2];
        let b = u8::from_str_radix(chunk, 16).map_err(|_| CorpFinanceError::InvalidInput {
            field: "hex".into(),
            reason: format!("non-hex digit in {chunk:?}"),
        })?;
        out.push(b);
    }
    Ok(out)
}

/// Generate a fresh ed25519 keypair.
///
/// Sources 32 bytes of entropy from the OS via `getrandom`. This is the
/// only place randomness is consumed; downstream APIs accept and persist
/// the resulting [`SigningKey`].
pub fn generate_keypair() -> CorpFinanceResult<(SigningKey, VerifyingKey)> {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "failed to source entropy for ed25519 keypair: {e}"
        ))
    })?;
    let sk = SigningKey::from_bytes(&seed);
    let vk = sk.verifying_key();
    Ok((sk, vk))
}

/// Sign `content_hash` (already-hashed manifest body) and produce a
/// [`SignedManifest`] envelope.
///
/// The function signs the raw bytes of `content_hash` (interpreted as
/// UTF-8); verification re-derives those bytes the same way.
pub fn sign_manifest(content_hash: &str, signing_key: &SigningKey) -> SignedManifest {
    let sig: Signature = signing_key.sign(content_hash.as_bytes());
    let vk = signing_key.verifying_key();
    SignedManifest {
        content_hash: content_hash.to_string(),
        signature: hex_encode(&sig.to_bytes()),
        public_key: hex_encode(vk.as_bytes()),
        signed_at: Utc::now(),
    }
}

/// Verify the ed25519 signature on a [`SignedManifest`].
///
/// Returns `Ok(true)` for a valid signature, `Ok(false)` for a clean
/// failure (the verifying key parses but the signature does not match),
/// and an error for malformed hex / bad key bytes / wrong signature
/// length.
pub fn verify_manifest(manifest: &SignedManifest) -> CorpFinanceResult<bool> {
    let pk_bytes = hex_decode(&manifest.public_key)?;
    if pk_bytes.len() != 32 {
        return Err(CorpFinanceError::InvalidInput {
            field: "public_key".into(),
            reason: format!("expected 32 bytes, got {}", pk_bytes.len()),
        });
    }
    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&pk_bytes);
    let vk = VerifyingKey::from_bytes(&pk_arr).map_err(|e| CorpFinanceError::InvalidInput {
        field: "public_key".into(),
        reason: format!("invalid ed25519 public key: {e}"),
    })?;

    let sig_bytes = hex_decode(&manifest.signature)?;
    if sig_bytes.len() != 64 {
        return Err(CorpFinanceError::InvalidInput {
            field: "signature".into(),
            reason: format!("expected 64 bytes, got {}", sig_bytes.len()),
        });
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let sig = Signature::from_bytes(&sig_arr);

    Ok(vk.verify(manifest.content_hash.as_bytes(), &sig).is_ok())
}

/// Read the 32-byte secret key from `path` and reconstruct a
/// [`SigningKey`].
///
/// Errors propagate as [`CorpFinanceError::SerializationError`] for I/O
/// failures and [`CorpFinanceError::InvalidInput`] for length mismatches.
pub fn load_signing_key(path: &Path) -> CorpFinanceResult<SigningKey> {
    let bytes = fs::read(path).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "failed to read signing key {}: {e}",
            path.display()
        ))
    })?;
    if bytes.len() != 32 {
        return Err(CorpFinanceError::InvalidInput {
            field: "signing.key".into(),
            reason: format!("expected 32 bytes, got {}", bytes.len()),
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(SigningKey::from_bytes(&arr))
}

/// Persist the 32-byte secret-key bytes of `key` to `path`.
///
/// Creates intermediate directories. Existing files are overwritten —
/// callers are expected to gate this behind an `ensure_keypair`
/// existence check.
pub fn save_signing_key(path: &Path, key: &SigningKey) -> CorpFinanceResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            CorpFinanceError::SerializationError(format!(
                "failed to create {}: {e}",
                parent.display()
            ))
        })?;
    }
    let bytes = key.to_bytes();
    fs::write(path, bytes).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "failed to write signing key {}: {e}",
            path.display()
        ))
    })?;
    Ok(())
}

/// Persist the 32-byte public-key bytes of `key` to `path`.
pub fn save_verifying_key(path: &Path, key: &VerifyingKey) -> CorpFinanceResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            CorpFinanceError::SerializationError(format!(
                "failed to create {}: {e}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, key.as_bytes()).map_err(|e| {
        CorpFinanceError::SerializationError(format!(
            "failed to write verifying key {}: {e}",
            path.display()
        ))
    })?;
    Ok(())
}

/// First-run convenience: load `(signing.key, verifying.key)` from
/// `keys_dir` if present; otherwise generate a fresh keypair and write
/// it to disk.
///
/// The directory layout is fixed to:
/// - `<keys_dir>/signing.key`
/// - `<keys_dir>/verifying.key`
pub fn ensure_keypair(keys_dir: &Path) -> CorpFinanceResult<(SigningKey, VerifyingKey)> {
    let sk_path = keys_dir.join("signing.key");
    let vk_path = keys_dir.join("verifying.key");
    if sk_path.exists() && vk_path.exists() {
        let sk = load_signing_key(&sk_path)?;
        let vk = sk.verifying_key();
        return Ok((sk, vk));
    }
    let (sk, vk) = generate_keypair()?;
    save_signing_key(&sk_path, &sk)?;
    save_verifying_key(&vk_path, &vk)?;
    Ok((sk, vk))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn keypair_round_trip_via_disk() {
        let dir = TempDir::new().unwrap();
        let sk_path = dir.path().join("signing.key");
        let (sk1, _) = generate_keypair().unwrap();
        save_signing_key(&sk_path, &sk1).unwrap();
        let sk2 = load_signing_key(&sk_path).unwrap();
        assert_eq!(sk1.to_bytes(), sk2.to_bytes());
    }

    #[test]
    fn sign_then_verify_succeeds() {
        let (sk, _) = generate_keypair().unwrap();
        let m = sign_manifest("deadbeef", &sk);
        assert!(verify_manifest(&m).unwrap());
    }

    #[test]
    fn tampered_content_hash_fails_verification() {
        let (sk, _) = generate_keypair().unwrap();
        let mut m = sign_manifest("deadbeef", &sk);
        m.content_hash = "feedface".into();
        assert!(!verify_manifest(&m).unwrap());
    }

    #[test]
    fn ensure_keypair_persists_across_calls() {
        let dir = TempDir::new().unwrap();
        let (sk1, _) = ensure_keypair(dir.path()).unwrap();
        let (sk2, _) = ensure_keypair(dir.path()).unwrap();
        assert_eq!(sk1.to_bytes(), sk2.to_bytes());
    }

    #[test]
    fn hex_round_trip() {
        let bytes = vec![0x00, 0xff, 0xab, 0x12];
        let s = hex_encode(&bytes);
        assert_eq!(s, "00ffab12");
        let back = hex_decode(&s).unwrap();
        assert_eq!(back, bytes);
    }
}
