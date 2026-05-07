//! Portable `.cfa-session` archive: gzip-compressed JSON.
//!
//! See ADR-016 §"Cross-session persistence (portable session archive)".
//!
//! The format is intentionally portable and crate-version-agnostic:
//!
//! - `flate2` gzip envelope (magic `1f 8b`).
//! - `serde_json`-encoded `CfaSession` payload.
//! - Schema is the `CfaSession` struct itself; field order is irrelevant.
//!
//! Round-trip equality (RUF-MEM-INV-003) is enforced by
//! [`round_trip_test_helper`].

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::error::CorpFinanceError;
use crate::memory::types::CfaSession;
use crate::CorpFinanceResult;

/// Persist a `CfaSession` to a `.cfa-session` archive.
pub fn save(session: &CfaSession, path: &Path) -> CorpFinanceResult<()> {
    let json = serde_json::to_vec(session)?;
    let file = File::create(path).map_err(io_to_cf)?;
    let mut encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
    encoder.write_all(&json).map_err(io_to_cf)?;
    encoder.finish().map_err(io_to_cf)?;
    Ok(())
}

/// Restore a `CfaSession` from a `.cfa-session` archive.
pub fn restore(path: &Path) -> CorpFinanceResult<CfaSession> {
    let file = File::open(path).map_err(io_to_cf)?;
    let mut decoder = GzDecoder::new(BufReader::new(file));
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).map_err(io_to_cf)?;
    let session: CfaSession = serde_json::from_slice(&buf)?;
    Ok(session)
}

/// Round-trip helper for RUF-MEM-INV-003 — `restore(save(s)) == s`.
///
/// Writes to a uniquely-named temp file in `std::env::temp_dir()`, reads
/// it back, and removes the file. Returns the restored aggregate.
pub fn round_trip_test_helper(session: &CfaSession) -> CorpFinanceResult<CfaSession> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "cfa-session-roundtrip-{}.cfa-session",
        session.session_id
    ));
    save(session, &path)?;
    let restored = restore(&path)?;
    let _ = std::fs::remove_file(&path);
    Ok(restored)
}

fn io_to_cf(e: std::io::Error) -> CorpFinanceError {
    CorpFinanceError::SerializationError(format!("io: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{RunSummary, Surface};

    fn mk_session_with_runs(n: usize) -> CfaSession {
        let mut s = CfaSession::new(Surface::Cli);
        for i in 0..n {
            s.append(RunSummary::new(
                Surface::Mcp,
                format!("tool_{i}"),
                "djb2:0xaaaa",
                format!("summary {i}"),
                vec![0.1 * (i as f32), 0.2, 0.3],
            ));
        }
        s
    }

    #[test]
    fn round_trip_preserves_fields() {
        let s = mk_session_with_runs(3);
        let restored = round_trip_test_helper(&s).unwrap();
        assert_eq!(s, restored);
    }
}
