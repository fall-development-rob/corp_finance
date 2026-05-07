//! Federated session lifecycle aggregate (v2-relevant; v1 stub).
//!
//! The DDD declares the `FederatedSession` aggregate with a state machine
//! `Initiated -> Authenticated -> Authorised -> Active` plus `Hold` /
//! `Failed` transitions and a both-peer-signed handshake gate for
//! `Trusted` (PaidVendor) federation. v1 ships the type and three
//! lifecycle helpers (`open_session`, `close_session`, `record_payload`)
//! so callers can declare sessions; the actual handshake stack lands in
//! v2 with `rustls` + `ed25519-dalek`.

use chrono::Utc;
use uuid::{Timestamp, Uuid};

use super::types::FederatedSession;

// TODO(v2): mTLS + ed25519 challenge-response handshake. Today we
// generate a fresh session id and timestamp; v2 will accept a peer
// certificate, run the rustls handshake, and verify a signed nonce
// before the session is allowed to record any payloads.

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
