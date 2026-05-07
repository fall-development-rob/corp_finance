//! CLI subcommands for `cfa entities <verb>` (Phase 27).
//!
//! Two verbs:
//! - `cfa entities list` — extract typed `EntityRef` records from a free-
//!   form text input (either `--from-text` or `--from-file`).
//! - `cfa entities graph` — read a JSONL file of `RunSummary` records,
//!   build a session-scoped `EntityGraph`, and report entities whose
//!   adjacency-degree meets the supplied threshold (`pattern_detected`).
//!
//! See `corp_finance_core::multi_agent::entity_graph` for the extractor
//! and graph implementation.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{json, Value};

use corp_finance_core::memory::types::RunSummary;
use corp_finance_core::multi_agent::entity_graph::{extract_entities_from_text, EntityGraph};
use corp_finance_core::multi_agent::types::{EntityRef, RelationKind};

/// Top-level arg group for the `entities` subcommand group.
#[derive(Args)]
pub struct EntitiesArgs {
    #[command(subcommand)]
    pub command: EntitiesCommands,
}

#[derive(Subcommand)]
pub enum EntitiesCommands {
    /// Extract typed entities from free-form text.
    List(EntitiesListArgs),
    /// Build an entity graph from a JSONL of run summaries and report
    /// entities meeting a co-occurrence threshold.
    Graph(EntitiesGraphArgs),
}

// ---------------------------------------------------------------------------
// EntitiesListArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct EntitiesListArgs {
    /// Inline text to scan.
    #[arg(long, conflicts_with = "from_file")]
    pub from_text: Option<String>,

    /// Path to a UTF-8 file whose contents are scanned.
    #[arg(long)]
    pub from_file: Option<String>,
}

pub fn run_list(args: EntitiesListArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let text = match (args.from_text.as_deref(), args.from_file.as_deref()) {
        (Some(t), _) => t.to_string(),
        (None, Some(p)) => {
            let bytes = std::fs::read(p)
                .map_err(|e| format!("could not read --from-file '{}': {}", p, e))?;
            String::from_utf8_lossy(&bytes).into_owned()
        }
        (None, None) => {
            return Err("provide either --from-text or --from-file".into());
        }
    };

    let entities: Vec<EntityRef> = extract_entities_from_text(&text);
    Ok(json!({
        "count": entities.len(),
        "entities": entities,
    }))
}

// ---------------------------------------------------------------------------
// EntitiesGraphArgs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct EntitiesGraphArgs {
    /// Path to a JSONL file. Each line is one JSON object with at least
    /// a `summary_text` field; `RunSummary` records are accepted natively.
    /// Malformed lines are skipped with a stderr warning.
    #[arg(long)]
    pub build_from_runs: String,

    /// Adjacency-degree threshold for `pattern_detected`. Default: 2.
    #[arg(long, default_value_t = 2)]
    pub threshold: usize,
}

pub fn run_graph(args: EntitiesGraphArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let path = PathBuf::from(&args.build_from_runs);
    let bytes = std::fs::read(&path).map_err(|e| {
        format!(
            "could not read --build-from-runs '{}': {}",
            path.display(),
            e
        )
    })?;
    let text = String::from_utf8_lossy(&bytes);

    let mut graph = EntityGraph::new();
    let mut lines_total: usize = 0;
    let mut lines_ok: usize = 0;
    let mut lines_skipped: usize = 0;
    let mut entities_added: usize = 0;
    let mut relations_added: usize = 0;

    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        lines_total += 1;

        let summary_text = match parse_summary_text(line) {
            Some(t) => t,
            None => {
                eprintln!(
                    "[cfa] entities graph: skipping malformed line {} (no summary_text)",
                    lineno + 1
                );
                lines_skipped += 1;
                continue;
            }
        };

        let extracted = extract_entities_from_text(&summary_text);
        // Add each entity, then connect every pair as MentionedTogether
        // so adjacency-degree reflects per-line co-occurrence.
        for e in &extracted {
            graph.add_entity(e.clone());
            entities_added += 1;
        }
        for i in 0..extracted.len() {
            for j in (i + 1)..extracted.len() {
                graph.add_relation(
                    extracted[i].clone(),
                    extracted[j].clone(),
                    RelationKind::MentionedTogether,
                );
                relations_added += 1;
            }
        }
        lines_ok += 1;
    }

    let patterns = graph.pattern_detected(args.threshold);

    Ok(json!({
        "input": path.display().to_string(),
        "threshold": args.threshold,
        "lines_total": lines_total,
        "lines_ingested": lines_ok,
        "lines_skipped": lines_skipped,
        "entities_added": entities_added,
        "relations_added": relations_added,
        "node_count": graph.node_count(),
        "edge_count": graph.edge_count(),
        "patterns_detected": patterns,
    }))
}

/// Pull `summary_text` out of a single JSONL line. Accepts either a full
/// `RunSummary` envelope or any object that has a `summary_text` string.
fn parse_summary_text(line: &str) -> Option<String> {
    // Fast path: try to parse as a `RunSummary`.
    if let Ok(s) = serde_json::from_str::<RunSummary>(line) {
        return Some(s.summary_text);
    }
    // Loose path: any JSON object with a `summary_text` string field.
    let v: Value = serde_json::from_str(line).ok()?;
    v.get("summary_text")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}
