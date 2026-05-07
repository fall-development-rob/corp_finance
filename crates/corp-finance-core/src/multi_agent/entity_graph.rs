//! `petgraph`-backed entity graph for the multi-agent coordination
//! bounded context. Supersedes the Phase 26 stub at
//! [`crate::memory::entity_graph`]; the stub is left in place for
//! backwards compatibility with existing memory tests and will be cleaned
//! up in a follow-up pass.
//!
//! The graph holds [`EntityRef`] nodes and directed [`RelationKind`]
//! edges. It is per-session in v1; cross-session entity views are served
//! by the Phase 26 memory layer reading these adjacencies into BM25/HNSW
//! retrieval.
//!
//! ## Entity extraction
//!
//! [`extract_entities_from_text`] is a hand-rolled tag-and-token extractor
//! seeded with the CFA identifier set. It mirrors the Phase 26 stub's
//! ticker / CUSIP heuristics and extends with sector and fund hints:
//!
//! - **Ticker** — runs of 1..=5 ASCII upper-case letters bounded by
//!   non-alphanumerics, but only if the token is not a recognised
//!   stop-word (e.g. `THE`, `AND`).
//! - **Cusip** — runs of exactly 9 ASCII alphanumerics with at least one
//!   digit.
//! - **Sector** — exact match against a small whitelist of common
//!   sector names (case-insensitive; canonicalised to the whitelist
//!   spelling).
//! - **Fund** — heuristic for `<word>-<digits>` patterns or names ending
//!   in `Fund` / `LP` / `LLC`. v1 keeps this small; v2 will plug in the
//!   FIGI / OpenCorporates lookups described in ADR-018.
//! - **Issuer** — multi-token capitalised phrase ending in a corporate
//!   suffix (`Inc`, `Corp`, `Ltd`, `PLC`, `AG`, `SA`). Conservative to
//!   keep the false-positive rate low.

use std::collections::HashMap;

use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Directed;

use crate::multi_agent::types::{EntityKind, EntityRef, RelationKind};

/// Directed `petgraph` graph with `EntityRef` nodes and `RelationKind`
/// edges.
pub struct EntityGraph {
    graph: Graph<EntityRef, RelationKind, Directed>,
    index: HashMap<EntityRef, NodeIndex>,
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
        }
    }

    /// Number of nodes currently in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Number of edges currently in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Insert an entity, returning the node index. Idempotent on the
    /// `(kind, value)` pair; repeated inserts return the existing node.
    pub fn add_entity(&mut self, entity: EntityRef) -> NodeIndex {
        if let Some(idx) = self.index.get(&entity) {
            return *idx;
        }
        let idx = self.graph.add_node(entity.clone());
        self.index.insert(entity, idx);
        idx
    }

    /// Insert a directed relation from `from` to `to`. Both endpoints
    /// are added to the graph if not already present.
    pub fn add_relation(&mut self, from: EntityRef, to: EntityRef, kind: RelationKind) {
        let from_idx = self.add_entity(from);
        let to_idx = self.add_entity(to);
        self.graph.add_edge(from_idx, to_idx, kind);
    }

    /// Return the count of edges incident to `entity` (in or out). For
    /// nodes that don't exist in the graph, returns `0`.
    ///
    /// "Co-occurrence" here is the per-entity adjacency degree: every
    /// time an entity appears alongside another in a specialist output
    /// the orchestrator should call [`add_relation`] with kind
    /// `MentionedTogether`, which bumps both endpoints' degree counts.
    pub fn co_occurrence_count(&self, entity: &EntityRef) -> usize {
        let Some(&idx) = self.index.get(entity) else {
            return 0;
        };
        self.graph
            .edges_directed(idx, petgraph::Direction::Outgoing)
            .count()
            + self
                .graph
                .edges_directed(idx, petgraph::Direction::Incoming)
                .count()
    }

    /// Return the entities whose co-occurrence count is at least
    /// `threshold`.
    ///
    /// This is the underlying `pattern_detected` query per
    /// `RUF-ORC-005`. The orchestrator emits a `pattern_detected` event
    /// per matching entity when this is called over a sliding window.
    pub fn pattern_detected(&self, threshold: usize) -> Vec<EntityRef> {
        let mut out: Vec<EntityRef> = self
            .index
            .iter()
            .filter(|(entity, _)| self.co_occurrence_count(entity) >= threshold)
            .map(|(entity, _)| entity.clone())
            .collect();
        // Stable order for deterministic test assertions: by kind then value.
        out.sort_by(|a, b| (a.kind as u8, a.value.as_str()).cmp(&(b.kind as u8, b.value.as_str())));
        out
    }

    /// Return entities reachable from `entity` within `depth` hops via
    /// outgoing edges.
    ///
    /// `depth == 0` returns just `[entity]` if it exists in the graph.
    /// `depth == 1` returns immediate out-neighbours, and so on.
    /// Implemented as a bounded BFS over `petgraph` adjacencies.
    pub fn neighbors(&self, entity: &EntityRef, depth: u8) -> Vec<EntityRef> {
        let Some(&start) = self.index.get(entity) else {
            return Vec::new();
        };
        let mut visited: HashMap<NodeIndex, u8> = HashMap::new();
        visited.insert(start, 0);
        let mut frontier = vec![start];
        let mut out: Vec<EntityRef> = Vec::new();
        out.push(entity.clone());
        for d in 1..=depth {
            let mut next: Vec<NodeIndex> = Vec::new();
            for node in frontier.drain(..) {
                for edge in self.graph.edges(node) {
                    let target = edge.target();
                    if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(target) {
                        e.insert(d);
                        if let Some(weight) = self.graph.node_weight(target) {
                            out.push(weight.clone());
                        }
                        next.push(target);
                    }
                }
            }
            frontier = next;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Tag-and-token extractor
// ---------------------------------------------------------------------------

const TICKER_STOPWORDS: &[&str] = &[
    "A", "I", "AN", "AND", "AT", "BE", "BY", "DO", "FOR", "GO", "HE", "IF", "IN", "IS", "IT", "ME",
    "NO", "OF", "ON", "OR", "OS", "SO", "THE", "TO", "UP", "US", "WE", "PE", "VS", "EU", "UK",
    "USA", "GMT", "UTC", "EST", "CFA", "CEO", "CFO", "COO", "CTO", "GDP", "GAAP", "IFRS", "SEC",
    "ETF", "IPO", "FX", "FY", "IRR", "ROI", "ROE", "ROA", "EPS", "EBIT", "FCF", "P/E", "WACC",
    "DCF", "LBO", "MA", "AS", "BUT", "DCM", "ECM", "PIPE", "RFP", "RFI", "EBITDA", "OK", "WAS",
    "WERE", "ARE", "HAS", "HAD", "HAVE", "BEEN", "WITH", "FROM", "INTO", "OUT", "THAN", "THEN",
    "THIS", "THAT", "THEY", "ITS", "ALL", "ANY", "MAY", "NOT", "NOW", "PER", "VIA",
];

const SECTOR_NAMES: &[&str] = &[
    "Energy",
    "Materials",
    "Industrials",
    "Utilities",
    "Healthcare",
    "Financials",
    "Technology",
    "Telecommunications",
    "Consumer",
    "Staples",
    "Discretionary",
    "Real Estate",
];

const ISSUER_SUFFIXES: &[&str] = &[
    "Inc",
    "Inc.",
    "Corp",
    "Corp.",
    "Corporation",
    "Ltd",
    "Ltd.",
    "Limited",
    "PLC",
    "plc",
    "AG",
    "SA",
    "S.A.",
    "NV",
    "GmbH",
];

const FUND_SUFFIXES: &[&str] = &[
    "Fund", "Funds", "LP", "L.P.", "LLC", "LLP", "L.L.P.", "Trust", "SICAV", "OEIC", "ICAV",
];

/// Extract entity references from free-form text.
///
/// Output preserves first-seen order; duplicate `(kind, value)` pairs
/// are collapsed.
///
/// `source_invocation` is set to `None` on every returned entity; the
/// caller (chief-analyst or orchestrator) is responsible for stamping
/// the invocation id.
pub fn extract_entities_from_text(text: &str) -> Vec<EntityRef> {
    let mut out: Vec<EntityRef> = Vec::new();

    extract_tickers_and_cusips(text, &mut out);
    extract_sectors(text, &mut out);
    extract_funds(text, &mut out);
    extract_issuers(text, &mut out);

    let mut seen = std::collections::HashSet::new();
    out.retain(|e| seen.insert((e.kind, e.value.clone())));
    out
}

fn extract_tickers_and_cusips(text: &str, out: &mut Vec<EntityRef>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if !is_token_start(bytes[i], i, bytes) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < len && bytes[j].is_ascii_alphanumeric() {
            j += 1;
        }
        let token = match std::str::from_utf8(&bytes[i..j]) {
            Ok(s) => s,
            Err(_) => {
                i = j.max(i + 1);
                continue;
            }
        };
        if is_ticker(token) {
            out.push(EntityRef {
                kind: EntityKind::Ticker,
                value: token.to_string(),
                source_invocation: None,
            });
        } else if is_cusip(token) {
            out.push(EntityRef {
                kind: EntityKind::Cusip,
                value: token.to_string(),
                source_invocation: None,
            });
        }
        i = j.max(i + 1);
    }
}

fn extract_sectors(text: &str, out: &mut Vec<EntityRef>) {
    for sector in SECTOR_NAMES {
        if contains_word_ci(text, sector) {
            out.push(EntityRef {
                kind: EntityKind::Sector,
                value: (*sector).to_string(),
                source_invocation: None,
            });
        }
    }
}

fn extract_funds(text: &str, out: &mut Vec<EntityRef>) {
    // Walk tokens; when we hit a fund suffix, look back for up to 3
    // capitalised tokens and form the fund name.
    let tokens = tokenise_for_phrase(text);
    let n = tokens.len();
    let mut i = 0;
    while i < n {
        if FUND_SUFFIXES
            .iter()
            .any(|s| s.eq_ignore_ascii_case(tokens[i]))
            && i > 0
        {
            let start = i.saturating_sub(3);
            let phrase_start = (start..i)
                .rev()
                .take_while(|&k| starts_capital(tokens[k]))
                .last()
                .unwrap_or(i);
            if phrase_start < i {
                let value = tokens[phrase_start..=i].join(" ");
                out.push(EntityRef {
                    kind: EntityKind::Fund,
                    value,
                    source_invocation: None,
                });
            }
        }
        i += 1;
    }
}

fn extract_issuers(text: &str, out: &mut Vec<EntityRef>) {
    let tokens = tokenise_for_phrase(text);
    let n = tokens.len();
    let mut i = 0;
    while i < n {
        let raw = tokens[i].trim_end_matches(',').trim_end_matches('.');
        if ISSUER_SUFFIXES.iter().any(|s| s.eq_ignore_ascii_case(raw)) && i > 0 {
            let start = i.saturating_sub(3);
            let phrase_start = (start..i)
                .rev()
                .take_while(|&k| starts_capital(tokens[k]))
                .last()
                .unwrap_or(i);
            if phrase_start < i {
                let value = tokens[phrase_start..=i].join(" ");
                out.push(EntityRef {
                    kind: EntityKind::Issuer,
                    value,
                    source_invocation: None,
                });
            }
        }
        i += 1;
    }
}

fn tokenise_for_phrase(text: &str) -> Vec<&str> {
    text.split([' ', '\t', '\n', '\r'])
        .filter(|s| !s.is_empty())
        .collect()
}

fn starts_capital(token: &str) -> bool {
    token
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
}

fn contains_word_ci(text: &str, word: &str) -> bool {
    // Case-insensitive whole-word match: split text into space-delimited
    // tokens and look for the word (also potentially multi-word) as a
    // substring whose neighbours are non-alphanumeric.
    let lhs = text.to_ascii_lowercase();
    let rhs = word.to_ascii_lowercase();
    let mut search_start = 0;
    while let Some(idx) = lhs[search_start..].find(&rhs) {
        let abs = search_start + idx;
        let before_ok = abs == 0 || !lhs.as_bytes()[abs - 1].is_ascii_alphanumeric();
        let after = abs + rhs.len();
        let after_ok = after >= lhs.len() || !lhs.as_bytes()[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        search_start = abs + 1;
    }
    false
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
    if !token.chars().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    !TICKER_STOPWORDS.contains(&token)
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

    fn ticker(value: &str) -> EntityRef {
        EntityRef {
            kind: EntityKind::Ticker,
            value: value.into(),
            source_invocation: None,
        }
    }

    #[test]
    fn extracts_ticker_and_cusip() {
        let ents = extract_entities_from_text("AAPL traded with CUSIP 037833100 today");
        assert!(ents
            .iter()
            .any(|e| e.kind == EntityKind::Ticker && e.value == "AAPL"));
        assert!(ents
            .iter()
            .any(|e| e.kind == EntityKind::Cusip && e.value == "037833100"));
    }

    #[test]
    fn skips_ticker_stopwords() {
        let ents = extract_entities_from_text("THE AND OF on AAPL");
        let tickers: Vec<&str> = ents
            .iter()
            .filter(|e| e.kind == EntityKind::Ticker)
            .map(|e| e.value.as_str())
            .collect();
        assert!(tickers.contains(&"AAPL"));
        assert!(!tickers.contains(&"THE"));
        assert!(!tickers.contains(&"AND"));
    }

    #[test]
    fn extracts_sector() {
        let ents = extract_entities_from_text("Healthcare and Financials lagged");
        assert!(ents
            .iter()
            .any(|e| e.kind == EntityKind::Sector && e.value == "Healthcare"));
        assert!(ents
            .iter()
            .any(|e| e.kind == EntityKind::Sector && e.value == "Financials"));
    }

    #[test]
    fn extracts_issuer_with_suffix() {
        let ents = extract_entities_from_text("Acme Industrial Corp reported");
        assert!(ents.iter().any(|e| e.kind == EntityKind::Issuer
            && e.value.contains("Acme")
            && e.value.contains("Corp")));
    }

    #[test]
    fn extracts_fund_with_suffix() {
        let ents = extract_entities_from_text("Carlyle Buyout Fund closed Q3");
        assert!(ents
            .iter()
            .any(|e| e.kind == EntityKind::Fund && e.value.contains("Fund")));
    }

    #[test]
    fn add_entity_is_idempotent() {
        let mut g = EntityGraph::new();
        let a = ticker("AAPL");
        let i1 = g.add_entity(a.clone());
        let i2 = g.add_entity(a.clone());
        assert_eq!(i1, i2);
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn add_relation_creates_directed_edge() {
        let mut g = EntityGraph::new();
        let a = ticker("AAPL");
        let b = ticker("MSFT");
        g.add_relation(a.clone(), b.clone(), RelationKind::MentionedTogether);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.co_occurrence_count(&a), 1);
        assert_eq!(g.co_occurrence_count(&b), 1);
    }

    #[test]
    fn pattern_detected_returns_entities_meeting_threshold() {
        let mut g = EntityGraph::new();
        let a = ticker("AAPL");
        let b = ticker("MSFT");
        let c = ticker("GOOG");
        g.add_relation(a.clone(), b.clone(), RelationKind::MentionedTogether);
        g.add_relation(a.clone(), c.clone(), RelationKind::MentionedTogether);
        g.add_relation(b.clone(), c.clone(), RelationKind::MentionedTogether);
        // a has degree 2, b has degree 2, c has degree 2.
        let hits = g.pattern_detected(2);
        assert_eq!(hits.len(), 3);
        let hits = g.pattern_detected(3);
        assert!(hits.is_empty());
    }

    #[test]
    fn neighbors_bfs_respects_depth() {
        let mut g = EntityGraph::new();
        let a = ticker("AAPL");
        let b = ticker("MSFT");
        let c = ticker("GOOG");
        g.add_relation(a.clone(), b.clone(), RelationKind::MentionedTogether);
        g.add_relation(b.clone(), c.clone(), RelationKind::MentionedTogether);
        let d0 = g.neighbors(&a, 0);
        assert_eq!(d0, vec![a.clone()]);
        let d1 = g.neighbors(&a, 1);
        assert!(d1.contains(&a));
        assert!(d1.contains(&b));
        assert!(!d1.contains(&c));
        let d2 = g.neighbors(&a, 2);
        assert!(d2.contains(&c));
    }
}
