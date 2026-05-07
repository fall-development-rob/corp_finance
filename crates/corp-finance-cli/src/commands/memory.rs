//! CLI subcommands for `cfa memory <verb>` (Phase 26).
//!
//! Two verbs:
//! - `cfa memory find` — query the persisted HNSW + side-store index by
//!   embedding (vector) or by text (BM25 over the loaded summaries).
//! - `cfa memory ingest` — append a `RunSummary` JSON file into the persisted
//!   index.
//!
//! The on-disk index is the gzip+JSON envelope produced by
//! `HnswMemoryIndex::save_to`. The default `--index-dir` is
//! `<repo>/var/observability/`; the active envelope lives at
//! `<index-dir>/memory-index.cfa-mem`.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::memory::types::{RunSummary, Surface};
use corp_finance_core::memory::{BM25MemoryIndex, HnswMemoryIndex};

/// Top-level arg group for the `memory` subcommand group.
#[derive(Args)]
pub struct MemoryArgs {
    #[command(subcommand)]
    pub command: MemoryCommands,
}

#[derive(Subcommand)]
pub enum MemoryCommands {
    /// Find similar past `RunSummary` records.
    Find(MemoryFindArgs),
    /// Append a `RunSummary` JSON file into the persisted index.
    Ingest(MemoryIngestArgs),
}

// ---------------------------------------------------------------------------
// Defaults / helpers
// ---------------------------------------------------------------------------

const INDEX_FILENAME: &str = "memory-index.cfa-mem";

fn default_index_dir() -> PathBuf {
    let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    p.push("var");
    p.push("observability");
    p
}

fn resolve_index_path(opt: Option<&str>) -> PathBuf {
    let mut dir = match opt {
        Some(s) => PathBuf::from(s),
        None => default_index_dir(),
    };
    dir.push(INDEX_FILENAME);
    dir
}

fn ensure_parent_dir(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn parse_surface(s: &str) -> Result<Surface, Box<dyn std::error::Error>> {
    match s.to_ascii_lowercase().as_str() {
        "cli" => Ok(Surface::Cli),
        "mcp" => Ok(Surface::Mcp),
        "skill" => Ok(Surface::Skill),
        "plugin" => Ok(Surface::Plugin),
        other => Err(format!(
            "unknown surface '{}'; expected cli | mcp | skill | plugin",
            other
        )
        .into()),
    }
}

fn read_embedding_file(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let v: Value = serde_json::from_slice(&bytes)?;
    let arr = v
        .as_array()
        .ok_or("embedding file must contain a JSON array of f32 values")?;
    let mut out = Vec::with_capacity(arr.len());
    for x in arr {
        let n = x.as_f64().ok_or("embedding values must be numeric")?;
        out.push(n as f32);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// MemoryFindArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct MemoryFindArgs {
    /// Free-form text query (BM25 path).
    #[arg(long, conflicts_with = "embedding_file")]
    pub text: Option<String>,

    /// Path to a JSON file containing the query embedding (HNSW path).
    #[arg(long)]
    pub embedding_file: Option<String>,

    /// Optional surface filter: cli | mcp | skill | plugin.
    #[arg(long)]
    pub surface: Option<String>,

    /// Maximum number of results to return.
    #[arg(long, default_value_t = 5)]
    pub limit: usize,

    /// Optional tenant filter.
    #[arg(long)]
    pub tenant: Option<String>,

    /// Directory containing the persisted memory index.
    #[arg(long)]
    pub index_dir: Option<String>,
}

pub fn run_find(args: MemoryFindArgs) -> Result<Value, Box<dyn std::error::Error>> {
    if args.text.is_none() && args.embedding_file.is_none() {
        return Err("provide either --text or --embedding-file".into());
    }

    let path = resolve_index_path(args.index_dir.as_deref());
    if !path.exists() {
        return Ok(json!({
            "results": [],
            "note": format!("no memory index found at {}", path.display()),
        }));
    }

    let surface_filter = args.surface.as_deref().map(parse_surface).transpose()?;
    let tenant = args.tenant.clone();

    if let Some(emb_path) = args.embedding_file.as_deref() {
        let embedding = read_embedding_file(emb_path)?;
        let idx = HnswMemoryIndex::load_from(&path)?;
        let results = idx.query(&embedding, args.limit, |s: &RunSummary| {
            if let Some(want) = surface_filter {
                if s.surface != want {
                    return false;
                }
            }
            if let Some(ref want) = tenant {
                if s.tenant_id.as_deref() != Some(want.as_str()) {
                    return false;
                }
            }
            true
        });
        let payload: Vec<Value> = results
            .into_iter()
            .map(|(s, dist)| {
                json!({
                    "summary": s,
                    "distance": dist,
                })
            })
            .collect();
        Ok(json!({ "mode": "embedding", "results": payload }))
    } else {
        // Text path — rebuild a BM25 index from the persisted side store.
        let hnsw = HnswMemoryIndex::load_from(&path)?;
        let mut bm25 = BM25MemoryIndex::new()?;
        // Re-ingest every persisted summary into the BM25 index.  We re-read
        // the side store via a small replay: HnswMemoryIndex doesn't expose
        // an iter, so we save+round-trip via JSON.
        let summaries: Vec<RunSummary> = hnsw_envelope_summaries(&hnsw);
        for s in &summaries {
            // Tenant is preserved on the RunSummary.  Filter at query time.
            bm25.ingest(s)?;
        }
        let text = args.text.as_deref().unwrap_or("");
        let raw = bm25.query(text, args.limit, tenant.as_deref())?;
        let filtered: Vec<RunSummary> = raw
            .into_iter()
            .filter(|s| match surface_filter {
                Some(want) => s.surface == want,
                None => true,
            })
            .collect();
        Ok(json!({ "mode": "text", "results": filtered }))
    }
}

/// Helper: extract the persisted summaries from an HNSW index by serialising
/// to a transient JSON value.  Avoids exposing private fields of the index.
fn hnsw_envelope_summaries(idx: &HnswMemoryIndex) -> Vec<RunSummary> {
    // The index doesn't expose iter(), so we round-trip through save/load to a
    // temp file. This is wasteful for the text path but keeps the public API
    // surface narrow; revisit when the bounded-context iterator API lands.
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("cfa-memory-find-{}.cfa-mem", uuid::Uuid::now_v7()));
    if idx.save_to(&tmp).is_err() {
        return Vec::new();
    }
    let bytes = match std::fs::read(&tmp) {
        Ok(b) => b,
        Err(_) => {
            let _ = std::fs::remove_file(&tmp);
            return Vec::new();
        }
    };
    let _ = std::fs::remove_file(&tmp);

    // Decode the gzip+JSON envelope to extract `summaries`.
    use std::io::Read;
    let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
    let mut buf = Vec::new();
    if decoder.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }
    let v: Value = match serde_json::from_slice(&buf) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = match v.get("summaries").and_then(|x| x.as_array()) {
        Some(a) => a.clone(),
        None => return Vec::new(),
    };
    let mut out = Vec::with_capacity(arr.len());
    for entry in arr {
        if let Ok(s) = serde_json::from_value::<RunSummary>(entry) {
            out.push(s);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// MemoryIngestArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct MemoryIngestArgs {
    /// Path to a JSON file containing one `RunSummary` (or a JSON array of
    /// summaries).
    #[arg(long)]
    pub run_summary_file: String,

    /// Directory containing the persisted memory index.
    #[arg(long)]
    pub index_dir: Option<String>,
}

pub fn run_ingest(args: MemoryIngestArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let path = resolve_index_path(args.index_dir.as_deref());
    ensure_parent_dir(&path)?;

    let bytes = std::fs::read(&args.run_summary_file)?;
    let v: Value = serde_json::from_slice(&bytes)?;
    let summaries: Vec<RunSummary> = if v.is_array() {
        serde_json::from_value(v)?
    } else {
        vec![serde_json::from_value(v)?]
    };

    if summaries.is_empty() {
        return Err("run-summary file contained no records".into());
    }

    let dim = summaries[0].embedding.len();
    let mut idx = if path.exists() {
        HnswMemoryIndex::load_from(&path)?
    } else {
        HnswMemoryIndex::new(dim)
    };

    let mut ingested = 0_usize;
    for s in &summaries {
        idx.ingest(s)?;
        ingested += 1;
    }
    idx.save_to(&path)?;

    Ok(json!({
        "status": "ok",
        "ingested": ingested,
        "index_path": path.display().to_string(),
    }))
}
