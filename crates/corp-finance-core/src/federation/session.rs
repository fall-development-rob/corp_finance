//! Federated session lifecycle aggregate (Phase 29 Wave 4 — authenticated).
//!
//! The DDD declares the `FederatedSession` aggregate with a state machine
//! `Initiated -> Authenticated -> Authorised -> Active` plus `Hold` /
//! `Failed` transitions and a both-peer-signed handshake gate for
//! `Trusted` (PaidVendor) federation.
//!
//! - `open_session` — unauthenticated entry point (v1); used by tests and
//!   callers that have already authenticated out-of-band.
//! - `open_authenticated_session` — ed25519 challenge-response handshake
//!   (Phase 29 Wave 4); verifies the peer's signature over a caller-supplied
//!   nonce before returning a session.

use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use uuid::{Timestamp, Uuid};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

use super::types::FederatedSession;

/// Open a new federated session targeting `peer_id`.
///
/// v1: returns the struct without doing any network I/O. The session id
/// is a fresh UUID v7 (time-ordered, the workspace's chosen UUID
/// variant); `opened_at` is the wall clock; `payload_count` is zero. v2
/// will gate the return on a successful mTLS + ed25519 challenge-
/// response.
pub fn open_session(peer_id: &str) -> FederatedSession {
    FederatedSession {
        session_id: Uuid::new_v7(Timestamp::now(uuid::NoContext)),
        peer_id: peer_id.to_string(),
        opened_at: Utc::now(),
        closed_at: None,
        payload_count: 0,
    }
}

/// Close a federated session by recording `closed_at`. Idempotent: a
/// second call is a no-op (the original close timestamp wins).
pub fn close_session(session: &mut FederatedSession) {
    if session.closed_at.is_none() {
        session.closed_at = Some(Utc::now());
    }
}

/// Record one payload exchanged on `session`. Increments `payload_count`
/// with a saturating add so the aggregate cannot panic on overflow.
pub fn record_payload(session: &mut FederatedSession) {
    session.payload_count = session.payload_count.saturating_add(1);
}

/// Open a session only after verifying that `peer_id`'s signature over
/// `nonce` matches `peer_public_key`. The signature is the peer's
/// challenge response; the nonce is caller-generated and one-time-use
/// (caller is responsible for freshness).
///
/// Returns `Err(InvalidInput)` when the signature doesn't verify; `Ok` with
/// a populated `FederatedSession` otherwise.
pub fn open_authenticated_session(
    peer_id: &str,
    nonce: &[u8],
    challenge_signature: &[u8; 64],
    peer_public_key: &VerifyingKey,
) -> CorpFinanceResult<FederatedSession> {
    let sig = Signature::from_bytes(challenge_signature);
    peer_public_key
        .verify(nonce, &sig)
        .map_err(|_| CorpFinanceError::InvalidInput {
            field: "challenge_signature".into(),
            reason: format!("ed25519 verify failed for peer '{}'", peer_id),
        })?;
    Ok(open_session(peer_id))
}
