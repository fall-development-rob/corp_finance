//! Lightweight entity graph stub for the Memory bounded context.
//!
//! TODO(phase-27): expand into `multi_agent::entity_graph` and move the
//! richer relationship typing (entity -> sub-agent, entity -> MCP tool,
//! entity -> skill) over from the Memory context. Phase 26 ships only the
//! minimum needed for `RunSummary` enrichment: extract entities from
//! summary text and surface co-occurrence counts.

use std::collections::HashMap;

use petgraph::graph::{Graph, NodeIndex};
use petgraph::Directed;
use uuid::Uuid;

use crate::memory::types::{EntityKind, EntityRef};

/// Node payload — wraps an `EntityRef` for now; Phase 27 will widen this
/// to carry entity metadata (canonical name, identifier mapping, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityNode {
    pub entity: EntityRef,
}

/// Edge payload — `source_run` is the originating `RunSummary` id and
/// `weight` is a co-occurrence multiplicity (one increment per surface event).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityEdge {
    pub source_run: Uuid,
    pub weight: u32,
}

/// Directed graph over `EntityRef` nodes.
pub struct EntityGraph {
    graph: Graph<EntityNode, EntityEdge, Directed>,
    index: HashMap<EntityRef, NodeIndex>,
    /// Per-entity co-occurrence count (incremented every `add_entity`).
    co_occurrence: HashMap<EntityRef, usize>,
}

impl Default for EntityGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityGraph {
    /// Build an empty graph.
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            index: HashMap::new(),
            co_occurrence: HashMap::new(),
        }
    }

    /// Number of nodes currently in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Add an entity tied to a source run id; bumps co-occurrence count.
    ///
    /// Idempotent on the node: a repeated entity is a no-op for node
    /// creation but always bumps the co-occurrence counter.
    pub fn add_entity(&mut self, entity: EntityRef, _source_run: Uuid) {
        self.index.entry(entity.clone()).or_insert_with(|| {
            self.graph.add_node(EntityNode {
                entity: entity.clone(),
            })
        });
        *self.co_occurrence.entry(entity).or_insert(0) += 1;
    }

    /// Co-occurrence count for `entity`; `0` if never seen.
    pub fn co_occurrence_count(&self, entity: &EntityRef) -> usize {
        self.co_occurrence.get(entity).copied().unwrap_or(0)
    }
}

/// Extract entities from free-form text.
///
/// Phase 26 ships only ticker / CUSIP heuristics:
///
/// - Tickers — runs of 1..=5 ASCII upper-case letters bounded by non-alphanumerics.
/// - CUSIPs — runs of exactly 9 ASCII alphanumerics with at least one digit.
///
/// Both heuristics are deliberately simple; Phase 27 will swap in a
/// proper entity recogniser tied to FIGI / OpenCorporates lookups.
pub fn extract_entities_from_text(text: &str) -> Vec<EntityRef> {
    let mut out: Vec<EntityRef> = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if !is_token_start(bytes[i], i, bytes) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < len && (bytes[j].is_ascii_alphanumeric()) {
            j += 1;
        }
        let token = &bytes[i..j];
        let token_str = match std::str::from_utf8(token) {
            Ok(s) => s,
            Err(_) => {
                i = j.max(i + 1);
                continue;
            }
        };
        if is_ticker(token_str) {
            out.push(EntityRef {
                kind: EntityKind::Ticker,
                value: token_str.to_string(),
            });
        } else if is_cusip(token_str) {
            out.push(EntityRef {
                kind: EntityKind::Cusip,
                value: token_str.to_string(),
            });
        }
        i = j.max(i + 1);
    }
    // De-duplicate while preserving first-seen order.
    let mut seen = std::collections::HashSet::new();
    out.retain(|e| seen.insert(e.clone()));
    out
}

fn is_token_start(b: u8, i: usize, bytes: &[u8]) -> bool {
    if !b.is_ascii_alphanumeric() {
        return false;
    }
    if i == 0 {
        return true;
    }
    !bytes[i - 1].is_ascii_alphanumeric()
}

fn is_ticker(token: &str) -> bool {
    let len = token.len();
    if !(1..=5).contains(&len) {
        return false;
    }
    token.chars().all(|c| c.is_ascii_uppercase())
}

fn is_cusip(token: &str) -> bool {
    if token.len() != 9 {
        return false;
    }
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    let all_alnum = token.chars().all(|c| c.is_ascii_alphanumeric());
    has_digit && all_alnum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticker_extraction_picks_up_caps_words() {
        let ents = extract_entities_from_text("AAPL beat MSFT on revenue");
        let tickers: Vec<&str> = ents
            .iter()
            .filter(|e| e.kind == EntityKind::Ticker)
            .map(|e| e.value.as_str())
            .collect();
        assert!(tickers.contains(&"AAPL"));
        assert!(tickers.contains(&"MSFT"));
    }

    #[test]
    fn cusip_extraction_recognises_9char_alnum_with_digit() {
        let ents = extract_entities_from_text("Reference CUSIP 037833100 traded today");
        let cusips: Vec<&str> = ents
            .iter()
            .filter(|e| e.kind == EntityKind::Cusip)
            .map(|e| e.value.as_str())
            .collect();
        assert!(cusips.contains(&"037833100"));
    }

    #[test]
    fn add_entity_increments_co_occurrence() {
        let mut g = EntityGraph::new();
        let e = EntityRef {
            kind: EntityKind::Ticker,
            value: "AAPL".into(),
        };
        let run = Uuid::now_v7();
        g.add_entity(e.clone(), run);
        g.add_entity(e.clone(), run);
        assert_eq!(g.co_occurrence_count(&e), 2);
        assert_eq!(g.node_count(), 1);
    }
}
