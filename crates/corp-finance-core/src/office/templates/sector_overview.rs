//! Sector Overview template — converts a [`SectorOverviewInput`] into a
//! [`WordDocSpec`] in a single call.
//!
//! Gated on the `office` feature. Operates solely on caller-supplied,
//! pre-formatted strings; no compute-result dependency.

// ---------------------------------------------------------------------------
// Input struct
// ---------------------------------------------------------------------------

/// All data required to render an equity-research Sector Overview note.
#[cfg(feature = "office")]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SectorOverviewInput {
    pub sector_name: String,
    pub date: String,
    pub author: String,
    /// Multi-paragraph executive summary (split on blank lines).
    pub executive_summary: String,
    /// 3-6 thematic bullets; section omitted when empty.
    pub key_themes: Vec<String>,
    /// Multi-paragraph market size and growth commentary.
    pub market_size_and_growth: String,
    /// 3-6 key driver bullets; section omitted when empty.
    pub key_drivers: Vec<String>,
    /// 3-6 headwind bullets; section omitted when empty.
    pub headwinds: Vec<String>,
    /// Coverage universe table; first row is the header row.
    pub coverage_universe: Vec<Vec<String>>,
    /// Valuation multiples comparison table; first row is the header row.
    pub valuation_summary: Vec<Vec<String>>,
    /// (ticker, one-line thesis); section omitted when empty.
    pub top_picks: Vec<(String, String)>,
    /// (ticker, reason to avoid); section omitted when empty.
    pub avoid_list: Vec<(String, String)>,
    /// Multi-paragraph conclusion (split on blank lines).
    pub conclusion: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Convert a [`SectorOverviewInput`] into a [`crate::office::WordDocSpec`]
/// ready for [`crate::office::docx::write_word_doc`].
///
/// Optional sections are omitted entirely when their input collections are
/// empty, keeping the document lean:
/// - Key Themes (BulletList): skipped when `key_themes` is empty.
/// - Key Drivers (BulletList): skipped when `key_drivers` is empty.
/// - Headwinds (BulletList): skipped when `headwinds` is empty.
/// - Top Picks (Table): skipped when `top_picks` is empty.
/// - Names to Avoid (Table): skipped when `avoid_list` is empty.
///
/// Coverage Universe and Valuation Summary always render; empty input produces
/// a fallback paragraph so the heading is never a dead-end.
#[cfg(feature = "office")]
pub fn sector_overview_to_doc(input: &SectorOverviewInput) -> crate::office::WordDocSpec {
    use crate::office::types::{DocSection, WordDocSpec, WorkbookProperties};

    let mut sections: Vec<DocSection> = Vec::new();

    sections.push(build_cover(input));
    sections.push(build_text_section("Executive Summary", &input.executive_summary));

    if !input.key_themes.is_empty() {
        sections.push(build_bullet_section("Key Themes", &input.key_themes));
    }

    sections.push(build_text_section("Market Size and Growth", &input.market_size_and_growth));

    if !input.key_drivers.is_empty() {
        sections.push(build_bullet_section("Key Drivers", &input.key_drivers));
    }
    if !input.headwinds.is_empty() {
        sections.push(build_bullet_section("Headwinds", &input.headwinds));
    }

    sections.push(build_table_section(
        "Coverage Universe",
        &input.coverage_universe,
        "No coverage universe provided.",
    ));
    sections.push(build_table_section(
        "Valuation Summary",
        &input.valuation_summary,
        "No valuation summary provided.",
    ));

    if !input.top_picks.is_empty() {
        sections.push(build_pair_table_section(
            "Top Picks",
            &["Ticker", "Thesis"],
            &input.top_picks,
        ));
    }
    if !input.avoid_list.is_empty() {
        sections.push(build_pair_table_section(
            "Names to Avoid",
            &["Ticker", "Reason"],
            &input.avoid_list,
        ));
    }

    sections.push(build_text_section("Conclusion", &input.conclusion));
    sections.push(build_footer(input));

    WordDocSpec {
        sections,
        properties: WorkbookProperties {
            title: Some(format!("{}: Sector Overview", input.sector_name)),
            author: Some(input.author.clone()),
            company: None,
            subject: Some("Equity Research — Sector Overview".into()),
        },
    }
}

// ---------------------------------------------------------------------------
// Section builders
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
fn build_cover(input: &SectorOverviewInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 1,
                text: format!("{}: Sector Overview", input.sector_name),
            },
            DocBlock::Paragraph {
                runs: vec![TextRun { text: "Equity Research".into(), bold: true, italic: false }],
            },
            DocBlock::Paragraph {
                runs: vec![TextRun { text: input.date.clone(), bold: false, italic: true }],
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: format!("Author: {}", input.author),
                    bold: false,
                    italic: true,
                }],
            },
        ],
    }
}

/// Build a level-2 heading section followed by one paragraph per blank-line chunk.
#[cfg(feature = "office")]
fn build_text_section(heading: &str, body: &str) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading { level: 2, text: heading.to_owned() }];

    for para in split_paragraphs(body) {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun { text: para, bold: false, italic: false }],
        });
    }

    DocSection { blocks }
}

/// Build a level-2 heading section with a BulletList.
#[cfg(feature = "office")]
fn build_bullet_section(heading: &str, items: &[String]) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    DocSection {
        blocks: vec![
            DocBlock::Heading { level: 2, text: heading.to_owned() },
            DocBlock::BulletList { items: items.to_vec() },
        ],
    }
}

/// Build a level-2 heading section with a Table (first row = headers) or a
/// fallback paragraph when `rows` is empty.
#[cfg(feature = "office")]
fn build_table_section(
    heading: &str,
    rows: &[Vec<String>],
    fallback: &str,
) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading { level: 2, text: heading.to_owned() }];

    if rows.is_empty() {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun { text: fallback.to_owned(), bold: false, italic: false }],
        });
    } else {
        let headers = rows[0].clone();
        let data = rows[1..].to_vec();
        blocks.push(DocBlock::Table { headers, rows: data });
    }

    DocSection { blocks }
}

/// Build a level-2 heading section with a 2-column Table from `(String, String)` pairs.
#[cfg(feature = "office")]
fn build_pair_table_section(
    heading: &str,
    col_headers: &[&str; 2],
    pairs: &[(String, String)],
) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    let headers = vec![col_headers[0].to_owned(), col_headers[1].to_owned()];
    let rows: Vec<Vec<String>> = pairs
        .iter()
        .map(|(a, b)| vec![a.clone(), b.clone()])
        .collect();

    DocSection {
        blocks: vec![
            DocBlock::Heading { level: 2, text: heading.to_owned() },
            DocBlock::Table { headers, rows },
        ],
    }
}

#[cfg(feature = "office")]
fn build_footer(input: &SectorOverviewInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    DocSection {
        blocks: vec![DocBlock::Paragraph {
            runs: vec![TextRun {
                text: format!("{}, Equity Research", input.author),
                bold: false,
                italic: true,
            }],
        }],
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Split a multi-paragraph string on blank lines (`\n\n`).
/// Trims each chunk and discards empties.
#[cfg(feature = "office")]
fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;
    use crate::office::types::DocBlock;

    fn minimal_input() -> SectorOverviewInput {
        SectorOverviewInput {
            sector_name: "Technology".into(),
            date: "2026-05-08".into(),
            author: "Jane Analyst, CFA".into(),
            executive_summary: "The technology sector remains the engine of global growth.\n\nCloud, AI, and semiconductors are the dominant sub-themes.".into(),
            key_themes: vec!["AI adoption accelerating across enterprises".into()],
            market_size_and_growth: "The global tech market is estimated at $5.5T in 2026.".into(),
            key_drivers: vec!["Enterprise cloud migration".into()],
            headwinds: vec!["Macro-driven capex belt-tightening".into()],
            coverage_universe: vec![
                vec!["Ticker".into(), "Company".into(), "Market Cap".into(), "Rating".into(), "Target Price".into()],
                vec!["MSFT".into(), "Microsoft Corp".into(), "$3.0T".into(), "BUY".into(), "$480".into()],
            ],
            valuation_summary: vec![
                vec!["Ticker".into(), "EV/EBITDA".into(), "P/E".into()],
                vec!["MSFT".into(), "24x".into(), "34x".into()],
            ],
            top_picks: vec![("MSFT".into(), "Cloud + AI leadership at reasonable multiple".into())],
            avoid_list: vec![],
            conclusion: "We maintain a constructive sector stance with selective stock picking.".into(),
        }
    }

    #[test]
    fn sector_overview_to_doc_basic() {
        let input = minimal_input();
        let doc = sector_overview_to_doc(&input);

        // With avoid_list empty the sections are:
        // cover, exec_summary, key_themes, market_size_and_growth, key_drivers,
        // headwinds, coverage_universe, valuation_summary, top_picks,
        // conclusion, footer = 11 sections
        let count = doc.sections.len();
        assert!(
            (8..=12).contains(&count),
            "expected 8-12 sections, got {count}"
        );

        // First section's first block must be Heading level 1 containing the sector name.
        let first_block = &doc.sections[0].blocks[0];
        match first_block {
            DocBlock::Heading { level, text } => {
                assert_eq!(*level, 1, "cover heading must be level 1");
                assert!(
                    text.contains(&input.sector_name),
                    "cover heading must contain sector name, got: {text}"
                );
            }
            other => panic!("expected Heading level 1, got: {other:?}"),
        }

        // properties.title must contain the sector name.
        let title = doc.properties.title.as_deref().unwrap_or("");
        assert!(
            title.contains(&input.sector_name),
            "properties.title should contain sector_name; got: {title:?}"
        );
    }

    #[test]
    fn sector_overview_to_doc_round_trips_through_writer() {
        use crate::office::docx::write_word_doc;
        use tempfile::tempdir;

        let input = minimal_input();
        let doc = sector_overview_to_doc(&input);

        let dir = tempdir().expect("tempdir creation failed");
        let path = dir.path().join("sector_overview.docx");

        let result = write_word_doc(&doc, &path).expect("write_word_doc failed");

        assert!(path.exists(), "output file does not exist");
        assert!(
            result.bytes_written > 0,
            "expected nonzero bytes, got {}",
            result.bytes_written
        );

        let bytes = std::fs::read(&path).expect("failed to read output file");
        assert_eq!(
            &bytes[..4],
            b"PK\x03\x04",
            "docx must begin with ZIP magic bytes PK\\x03\\x04"
        );
    }
}
