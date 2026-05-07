//! Shared kernel — `Surface` enum used by every Phase 26 module
//! and Phase 27 multi_agent / federation modules.
//!
//! The CFA runtime executes through four surfaces; capture and
//! attribution always happens at one of these boundaries.
//!
//! Per Phase-26 ADR-015 §"Runtime surfaces vs deployment artefacts".
//!
//! Prior to Phase-27 cleanup, each of `memory`, `audit`, `observability`,
//! and `cost` declared its own structurally identical `Surface` enum
//! locally. They have been collapsed into this single shared kernel
//! module so the four enums cannot drift. Each Phase-26 module now
//! re-exports `Surface` from here for backward-compatibility.

use serde::{Deserialize, Serialize};

/// One of the four CFA execution surfaces.
///
/// Every CFA runtime event lands on exactly one of these four boundaries.
/// The wire form is `snake_case` — for the current variant set this is
/// byte-identical to lowercase, but `snake_case` is the canonical choice
/// to match the rest of the Phase-26 enum vocabulary (see `cost::TierTag`,
/// `cost::BudgetPeriod`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// `cfa <subcommand>` — CLI invocation, dispatched from
    /// `crates/corp-finance-cli/src/main.rs`.
    Cli,
    /// MCP `server.tool(...)` handler — invoked by any of the
    /// `packages/*-mcp-server/` crates.
    Mcp,
    /// Slash command or `.claude/skills/*` invocation — captured via
    /// the MCP wrapper through which the LLM ultimately executes.
    Skill,
    /// Plugin hook fire — `plugins/cfa-core/hooks/hooks.json`
    /// (`PreToolUse`, `PostToolUse`, `Write`, `Edit`).
    Plugin,
}

impl Surface {
    /// Lower-case display token used in serialisation, BM25 index fields,
    /// span attribute values, and audit-hash canonical strings.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Mcp => "mcp",
            Self::Skill => "skill",
            Self::Plugin => "plugin",
        }
    }

    /// Inverse of [`as_str`]. Returns `None` for any unrecognised token.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cli" => Some(Self::Cli),
            "mcp" => Some(Self::Mcp),
            "skill" => Some(Self::Skill),
            "plugin" => Some(Self::Plugin),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `as_str` round-trips through `parse` for every variant.
    #[test]
    fn as_str_round_trips_via_parse() {
        for s in [Surface::Cli, Surface::Mcp, Surface::Skill, Surface::Plugin] {
            assert_eq!(Surface::parse(s.as_str()), Some(s));
        }
    }

    /// Each variant maps to its expected lower-case token.
    #[test]
    fn as_str_uses_lowercase_tokens() {
        assert_eq!(Surface::Cli.as_str(), "cli");
        assert_eq!(Surface::Mcp.as_str(), "mcp");
        assert_eq!(Surface::Skill.as_str(), "skill");
        assert_eq!(Surface::Plugin.as_str(), "plugin");
    }

    /// Serde round-trip: every variant serialises to a quoted lower-case
    /// JSON string and deserialises back to the same variant.
    #[test]
    fn serde_round_trips_for_all_variants() {
        for s in [Surface::Cli, Surface::Mcp, Surface::Skill, Surface::Plugin] {
            let j = serde_json::to_string(&s).expect("serialize");
            let back: Surface = serde_json::from_str(&j).expect("deserialize");
            assert_eq!(s, back);
        }
    }

    /// Wire-format contract: JSON string is the lowercase token (no
    /// underscores, no extra characters). Locks the wire format so any
    /// accidental rename rule change is caught at test time.
    #[test]
    fn serde_wire_format_is_lowercase_token() {
        assert_eq!(serde_json::to_string(&Surface::Cli).unwrap(), "\"cli\"");
        assert_eq!(serde_json::to_string(&Surface::Mcp).unwrap(), "\"mcp\"");
        assert_eq!(serde_json::to_string(&Surface::Skill).unwrap(), "\"skill\"");
        assert_eq!(
            serde_json::to_string(&Surface::Plugin).unwrap(),
            "\"plugin\""
        );
    }

    /// All four variants are enumerable and distinct. Guards against
    /// accidental future variant deletion or duplication.
    #[test]
    fn all_four_variants_distinct() {
        let all = [Surface::Cli, Surface::Mcp, Surface::Skill, Surface::Plugin];
        assert_eq!(all.len(), 4);
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    /// `parse` returns `None` for any unrecognised token (case-sensitive).
    #[test]
    fn parse_returns_none_for_unknown_token() {
        assert_eq!(Surface::parse(""), None);
        assert_eq!(Surface::parse("CLI"), None);
        assert_eq!(Surface::parse("cli "), None);
        assert_eq!(Surface::parse("agent"), None);
    }
}
