//! Trust attestation issuance, verification, persistence, and revocation
//! (Phase 29 Wave 4, ADR-019 v2, RUF-FED-008).
//!
//! Every row read from the `attestations` table is crypto-re-verified before
//! being trusted. No row is trusted purely by database presence.
//!
//! ## Schema
//!
//! - `attestations` — one row per issued attestation
//! - `attestation_revocations` — revocation tombstones keyed by `attestation_id`
//!
//! ## Canonical payload format
//!
//! ```text
//! v1|<issuer>|<subject>|<caps_sorted_csv>|<issued_at_rfc3339>|<expires_at_rfc3339>
//! ```

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rusqlite::{params, Connection};
use uuid::{Timestamp, Uuid};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

use super::types::{AttestationRevocation, AttestationStatus, TrustAttestation};

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

/// Initialise the SQLite schema for attestations. Idempotent.
pub fn init_attestation_schema(conn: &Connection) -> CorpFinanceResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS attestations (
           attestation_id    TEXT PRIMARY KEY,
           issuer_tenant     TEXT NOT NULL,
           subject_tenant    TEXT NOT NULL,
           capabilities_json TEXT NOT NULL,
           issued_at         TEXT NOT NULL,
           expires_at        TEXT NOT NULL,
           signature         TEXT NOT NULL,
           public_key        TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_attestations_subject
           ON attestations(subject_tenant);
         CREATE TABLE IF NOT EXISTS attestation_revocations (
           attestation_id TEXT PRIMARY KEY,
           revoked_at     TEXT NOT NULL,
           revoked_by     TEXT NOT NULL,
           reason         TEXT NOT NULL
         );",
    )
    .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))
}

// ---------------------------------------------------------------------------
// Canonical payload
// ---------------------------------------------------------------------------

fn canonical_payload(
    issuer: &str,
    subject: &str,
    capabilities: &[String],
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> String {
    let mut sorted = capabilities.to_vec();
    sorted.sort();
    let caps_csv = sorted.join(",");
    format!(
        "v1|{}|{}|{}|{}|{}",
        issuer,
        subject,
        caps_csv,
        issued_at.to_rfc3339(),
        expires_at.to_rfc3339(),
    )
}

// ---------------------------------------------------------------------------
// Hex helpers (no external `hex` crate)
// ---------------------------------------------------------------------------

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(s: &str) -> CorpFinanceResult<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return Err(CorpFinanceError::InvalidInput {
            field: "hex_string".into(),
            reason: "odd-length hex string".into(),
        });
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| CorpFinanceError::InvalidInput {
                field: "hex_string".into(),
                reason: format!("invalid hex chars at position {}", i),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Issue
// ---------------------------------------------------------------------------

/// Issue a fresh attestation. Validates inputs, computes the canonical
/// payload, signs with `signing_key`, returns the populated aggregate.
///
/// Errors: empty issuer / subject, empty capabilities, non-positive `valid_for`.
pub fn issue_attestation(
    issuer_tenant: &str,
    subject_tenant: &str,
    capabilities: Vec<String>,
    valid_for: chrono::Duration,
    signing_key: &SigningKey,
) -> CorpFinanceResult<TrustAttestation> {
    if issuer_tenant.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "issuer_tenant".into(),
            reason: "must be non-empty".into(),
        });
    }
    if subject_tenant.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "subject_tenant".into(),
            reason: "must be non-empty".into(),
        });
    }
    if capabilities.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "capabilities".into(),
            reason: "must contain at least one capability".into(),
        });
    }
    if valid_for <= chrono::Duration::zero() {
        return Err(CorpFinanceError::InvalidInput {
            field: "valid_for".into(),
            reason: "must be a positive duration".into(),
        });
    }

    let issued_at = Utc::now();
    let expires_at = issued_at + valid_for;
    let payload = canonical_payload(
        issuer_tenant,
        subject_tenant,
        &capabilities,
        issued_at,
        expires_at,
    );
    let sig: Signature = signing_key.sign(payload.as_bytes());
    let verifying_key = signing_key.verifying_key();

    Ok(TrustAttestation {
        attestation_id: Uuid::new_v7(Timestamp::now(uuid::NoContext)),
        issuer_tenant: issuer_tenant.to_string(),
        subject_tenant: subject_tenant.to_string(),
        capabilities,
        issued_at,
        expires_at,
        signature: bytes_to_hex(sig.to_bytes().as_ref()),
        public_key: bytes_to_hex(verifying_key.as_bytes()),
    })
}

// ---------------------------------------------------------------------------
// Verify (pure, offline)
// ---------------------------------------------------------------------------

/// Pure offline verification. Does NOT consult the SQLite store.
pub fn verify_attestation(
    attestation: &TrustAttestation,
    expected_issuer: &str,
) -> AttestationStatus {
    if attestation.issuer_tenant != expected_issuer {
        return AttestationStatus::IssuerMismatch;
    }

    let pk_bytes = match hex_to_bytes(&attestation.public_key) {
        Ok(b) if b.len() == 32 => b,
        _ => return AttestationStatus::BadSignature,
    };
    let pk_arr: [u8; 32] = match pk_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return AttestationStatus::BadSignature,
    };
    let vk = match VerifyingKey::from_bytes(&pk_arr) {
        Ok(k) => k,
        Err(_) => return AttestationStatus::BadSignature,
    };

    let sig_bytes = match hex_to_bytes(&attestation.signature) {
        Ok(b) if b.len() == 64 => b,
        _ => return AttestationStatus::BadSignature,
    };
    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return AttestationStatus::BadSignature,
    };
    let sig = Signature::from_bytes(&sig_arr);

    let payload = canonical_payload(
        &attestation.issuer_tenant,
        &attestation.subject_tenant,
        &attestation.capabilities,
        attestation.issued_at,
        attestation.expires_at,
    );
    if vk.verify(payload.as_bytes(), &sig).is_err() {
        return AttestationStatus::BadSignature;
    }

    if Utc::now() > attestation.expires_at {
        return AttestationStatus::Expired;
    }

    AttestationStatus::Valid
}

// ---------------------------------------------------------------------------
// Status with revocation check
// ---------------------------------------------------------------------------

/// `verify_attestation` plus a revocation-table check via `conn`.
pub fn status_with_revocations(
    conn: &Connection,
    attestation: &TrustAttestation,
    expected_issuer: &str,
) -> CorpFinanceResult<AttestationStatus> {
    let base = verify_attestation(attestation, expected_issuer);
    if base == AttestationStatus::BadSignature || base == AttestationStatus::IssuerMismatch {
        return Ok(base);
    }
    if is_revoked(conn, attestation.attestation_id)? {
        return Ok(AttestationStatus::Revoked);
    }
    Ok(base)
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Persist (upsert) an attestation row.
pub fn save_attestation(
    conn: &Connection,
    attestation: &TrustAttestation,
) -> CorpFinanceResult<()> {
    init_attestation_schema(conn)?;
    let caps_json = serde_json::to_string(&attestation.capabilities)?;
    conn.execute(
        "INSERT INTO attestations
           (attestation_id, issuer_tenant, subject_tenant, capabilities_json,
            issued_at, expires_at, signature, public_key)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(attestation_id) DO UPDATE SET
           issuer_tenant = excluded.issuer_tenant,
           subject_tenant = excluded.subject_tenant,
           capabilities_json = excluded.capabilities_json,
           issued_at = excluded.issued_at,
           expires_at = excluded.expires_at,
           signature = excluded.signature,
           public_key = excluded.public_key",
        params![
            attestation.attestation_id.to_string(),
            attestation.issuer_tenant,
            attestation.subject_tenant,
            caps_json,
            attestation.issued_at.to_rfc3339(),
            attestation.expires_at.to_rfc3339(),
            attestation.signature,
            attestation.public_key,
        ],
    )
    .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    Ok(())
}

/// Load an attestation by ID. Returns `Ok(None)` if not found.
pub fn load_attestation(
    conn: &Connection,
    attestation_id: Uuid,
) -> CorpFinanceResult<Option<TrustAttestation>> {
    init_attestation_schema(conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT attestation_id, issuer_tenant, subject_tenant, capabilities_json,
                    issued_at, expires_at, signature, public_key
             FROM attestations WHERE attestation_id = ?1",
        )
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let mut rows = stmt
        .query(params![attestation_id.to_string()])
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    if let Some(row) = rows
        .next()
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?
    {
        Ok(Some(row_to_attestation(row)?))
    } else {
        Ok(None)
    }
}

/// List all attestations where `subject_tenant` matches.
pub fn list_attestations_by_subject(
    conn: &Connection,
    subject_tenant: &str,
) -> CorpFinanceResult<Vec<TrustAttestation>> {
    init_attestation_schema(conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT attestation_id, issuer_tenant, subject_tenant, capabilities_json,
                    issued_at, expires_at, signature, public_key
             FROM attestations WHERE subject_tenant = ?1",
        )
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let mut rows = stmt
        .query(params![subject_tenant])
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?
    {
        out.push(row_to_attestation(row)?);
    }
    Ok(out)
}

fn row_to_attestation(row: &rusqlite::Row<'_>) -> CorpFinanceResult<TrustAttestation> {
    let id_str: String = row
        .get(0)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let caps_json: String = row
        .get(3)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let issued_str: String = row
        .get(4)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let expires_str: String = row
        .get(5)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;

    let attestation_id = Uuid::parse_str(&id_str)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let capabilities: Vec<String> = serde_json::from_str(&caps_json)?;
    let issued_at = DateTime::parse_from_rfc3339(&issued_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let expires_at = DateTime::parse_from_rfc3339(&expires_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;

    Ok(TrustAttestation {
        attestation_id,
        issuer_tenant: row
            .get(1)
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?,
        subject_tenant: row
            .get(2)
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?,
        capabilities,
        issued_at,
        expires_at,
        signature: row
            .get(6)
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?,
        public_key: row
            .get(7)
            .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?,
    })
}

// ---------------------------------------------------------------------------
// Revocation
// ---------------------------------------------------------------------------

/// Record a revocation tombstone for the given attestation.
pub fn revoke_attestation(
    conn: &Connection,
    attestation_id: Uuid,
    revoked_by: &str,
    reason: &str,
) -> CorpFinanceResult<()> {
    init_attestation_schema(conn)?;
    let rev = AttestationRevocation {
        attestation_id,
        revoked_at: Utc::now(),
        revoked_by: revoked_by.to_string(),
        reason: reason.to_string(),
    };
    conn.execute(
        "INSERT INTO attestation_revocations (attestation_id, revoked_at, revoked_by, reason)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(attestation_id) DO NOTHING",
        params![
            rev.attestation_id.to_string(),
            rev.revoked_at.to_rfc3339(),
            rev.revoked_by,
            rev.reason,
        ],
    )
    .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    Ok(())
}

/// Returns true iff the attestation is listed in `attestation_revocations`.
pub fn is_revoked(conn: &Connection, attestation_id: Uuid) -> CorpFinanceResult<bool> {
    init_attestation_schema(conn)?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM attestation_revocations WHERE attestation_id = ?1",
            params![attestation_id.to_string()],
            |row| row.get(0),
        )
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    Ok(count > 0)
}

// ---------------------------------------------------------------------------
// Capability check
// ---------------------------------------------------------------------------

/// Returns true iff `subject_tenant` has a non-revoked, non-expired attestation
/// that grants `capability` from any issuer. The verifying key is read from
/// the attestation row; crypto is re-verified at every call (RUF-FED-008).
pub fn check_capability(
    conn: &Connection,
    subject_tenant: &str,
    capability: &str,
) -> CorpFinanceResult<bool> {
    let attestations = list_attestations_by_subject(conn, subject_tenant)?;
    for att in &attestations {
        let status = status_with_revocations(conn, att, &att.issuer_tenant.clone())?;
        if status == AttestationStatus::Valid && att.capabilities.iter().any(|c| c == capability) {
            return Ok(true);
        }
    }
    Ok(false)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[1u8; 32])
    }

    fn test_key_b() -> SigningKey {
        SigningKey::from_bytes(&[2u8; 32])
    }

    #[test]
    fn issue_then_verify_returns_valid() {
        let key = test_key();
        let att = issue_attestation(
            "alice",
            "bob",
            vec!["read:portfolio".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        assert_eq!(verify_attestation(&att, "alice"), AttestationStatus::Valid);
    }

    #[test]
    fn verify_rejects_tampered_signature() {
        let key = test_key();
        let mut att = issue_attestation(
            "alice",
            "bob",
            vec!["read:portfolio".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        // Flip first byte of hex signature
        let mut sig_bytes = hex_to_bytes(&att.signature).unwrap();
        sig_bytes[0] ^= 0xFF;
        att.signature = bytes_to_hex(&sig_bytes);
        assert_eq!(
            verify_attestation(&att, "alice"),
            AttestationStatus::BadSignature
        );
    }

    #[test]
    fn verify_rejects_tampered_capabilities() {
        let key = test_key();
        let mut att = issue_attestation(
            "alice",
            "bob",
            vec!["read:portfolio".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        att.capabilities.push("write:portfolio".into());
        assert_eq!(
            verify_attestation(&att, "alice"),
            AttestationStatus::BadSignature
        );
    }

    #[test]
    fn verify_returns_issuer_mismatch_when_caller_expects_wrong_issuer() {
        let key = test_key();
        let att = issue_attestation(
            "alice",
            "bob",
            vec!["read:portfolio".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        assert_eq!(
            verify_attestation(&att, "bob"),
            AttestationStatus::IssuerMismatch
        );
    }

    #[test]
    fn verify_returns_expired_after_window() {
        let key = test_key();
        // Issue with a negative duration: expires_at < issued_at, already expired
        let att = issue_attestation(
            "alice",
            "bob",
            vec!["read:portfolio".into()],
            chrono::Duration::seconds(-1),
            &key,
        );
        // Duration is non-positive, so issue should fail
        assert!(att.is_err());

        // Alternative: manually construct an already-expired attestation
        let now = Utc::now();
        let issued_at = now - chrono::Duration::hours(2);
        let expires_at = now - chrono::Duration::hours(1);
        let payload = canonical_payload(
            "alice",
            "bob",
            &["read:portfolio".to_string()],
            issued_at,
            expires_at,
        );
        let sig: Signature = key.sign(payload.as_bytes());
        let vk = key.verifying_key();
        let att = TrustAttestation {
            attestation_id: Uuid::new_v7(Timestamp::now(uuid::NoContext)),
            issuer_tenant: "alice".into(),
            subject_tenant: "bob".into(),
            capabilities: vec!["read:portfolio".into()],
            issued_at,
            expires_at,
            signature: bytes_to_hex(sig.to_bytes().as_ref()),
            public_key: bytes_to_hex(vk.as_bytes()),
        };
        assert_eq!(
            verify_attestation(&att, "alice"),
            AttestationStatus::Expired
        );
    }

    #[test]
    fn save_then_load_round_trips_attestation() {
        let conn = Connection::open_in_memory().unwrap();
        let key = test_key();
        let att = issue_attestation(
            "alice",
            "bob",
            vec!["read:portfolio".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        save_attestation(&conn, &att).unwrap();
        let loaded = load_attestation(&conn, att.attestation_id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded, att);
    }

    #[test]
    fn list_by_subject_returns_only_matching() {
        let conn = Connection::open_in_memory().unwrap();
        let key = test_key();
        for _ in 0..2 {
            let att = issue_attestation(
                "alice",
                "bob",
                vec!["cap1".into()],
                chrono::Duration::hours(1),
                &key,
            )
            .unwrap();
            save_attestation(&conn, &att).unwrap();
        }
        let att = issue_attestation(
            "alice",
            "carol",
            vec!["cap2".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        save_attestation(&conn, &att).unwrap();
        let bobs = list_attestations_by_subject(&conn, "bob").unwrap();
        assert_eq!(bobs.len(), 2);
        let carols = list_attestations_by_subject(&conn, "carol").unwrap();
        assert_eq!(carols.len(), 1);
    }

    #[test]
    fn revoke_then_status_with_revocations_returns_revoked() {
        let conn = Connection::open_in_memory().unwrap();
        let key = test_key();
        let att = issue_attestation(
            "alice",
            "bob",
            vec!["cap1".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        save_attestation(&conn, &att).unwrap();
        revoke_attestation(&conn, att.attestation_id, "admin", "policy violation").unwrap();
        let status = status_with_revocations(&conn, &att, "alice").unwrap();
        assert_eq!(status, AttestationStatus::Revoked);
    }

    #[test]
    fn is_revoked_false_for_unrevoked() {
        let conn = Connection::open_in_memory().unwrap();
        let key = test_key();
        let att = issue_attestation(
            "alice",
            "bob",
            vec!["cap1".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        save_attestation(&conn, &att).unwrap();
        assert!(!is_revoked(&conn, att.attestation_id).unwrap());
    }

    #[test]
    fn check_capability_true_when_unrevoked_unexpired_match() {
        let conn = Connection::open_in_memory().unwrap();
        let key = test_key();
        let att = issue_attestation(
            "alice",
            "bob",
            vec!["read:portfolio".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        save_attestation(&conn, &att).unwrap();
        assert!(check_capability(&conn, "bob", "read:portfolio").unwrap());
    }

    #[test]
    fn check_capability_false_after_revocation() {
        let conn = Connection::open_in_memory().unwrap();
        let key = test_key();
        let att = issue_attestation(
            "alice",
            "bob",
            vec!["read:portfolio".into()],
            chrono::Duration::hours(1),
            &key,
        )
        .unwrap();
        save_attestation(&conn, &att).unwrap();
        revoke_attestation(&conn, att.attestation_id, "admin", "revoked").unwrap();
        assert!(!check_capability(&conn, "bob", "read:portfolio").unwrap());
    }

    #[test]
    fn check_capability_false_for_unknown_subject() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(!check_capability(&conn, "nobody", "read:portfolio").unwrap());
    }

    #[test]
    fn canonical_payload_is_deterministic_across_capability_order() {
        let now = Utc::now();
        let expires = now + chrono::Duration::hours(1);
        let p1 = canonical_payload(
            "alice",
            "bob",
            &["a".to_string(), "b".to_string()],
            now,
            expires,
        );
        let p2 = canonical_payload(
            "alice",
            "bob",
            &["b".to_string(), "a".to_string()],
            now,
            expires,
        );
        assert_eq!(p1, p2);
        assert!(p1.starts_with("v1|"));
    }

    #[test]
    fn init_attestation_schema_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_attestation_schema(&conn).unwrap();
        init_attestation_schema(&conn).unwrap(); // second call must not error
    }

    #[test]
    fn open_authenticated_session_succeeds_with_valid_signature() {
        use super::super::session::open_authenticated_session;
        use ed25519_dalek::Signer;
        let key = test_key();
        let nonce = b"test-nonce-12345";
        let sig: Signature = key.sign(nonce);
        let vk = key.verifying_key();
        let sig_bytes: [u8; 64] = sig.to_bytes();
        let result = open_authenticated_session("peer-a", nonce, &sig_bytes, &vk);
        assert!(result.is_ok());
    }

    #[test]
    fn open_authenticated_session_rejects_bad_signature() {
        use super::super::session::open_authenticated_session;
        let key = test_key();
        let vk = key.verifying_key();
        let bad_sig = [0u8; 64];
        let result = open_authenticated_session("peer-a", b"nonce", &bad_sig, &vk);
        assert!(result.is_err());
    }

    #[test]
    fn open_authenticated_session_rejects_wrong_public_key() {
        use super::super::session::open_authenticated_session;
        use ed25519_dalek::Signer;
        let key_a = test_key();
        let key_b = test_key_b();
        let nonce = b"test-nonce-abc";
        let sig: Signature = key_a.sign(nonce);
        let vk_b = key_b.verifying_key();
        let sig_bytes: [u8; 64] = sig.to_bytes();
        let result = open_authenticated_session("peer-a", nonce, &sig_bytes, &vk_b);
        assert!(result.is_err());
    }
}
