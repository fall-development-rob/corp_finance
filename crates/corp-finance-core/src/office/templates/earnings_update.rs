//! Earnings Update template — converts an [`EarningsUpdateInput`] into a
//! [`WordDocSpec`] in a single call.
//!
//! Gated on the `office` feature. This is a compact research-note shape
//! (2-4 pages); optional sections are omitted entirely when their input vecs
//! are empty to keep the document lean.

// ---------------------------------------------------------------------------
// Input struct
// ---------------------------------------------------------------------------

/// All data required to render a post-earnings research note.
#[cfg(feature = "office")]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EarningsUpdateInput {
    pub ticker: String,
    pub company: String,
    pub date: String,
    pub author: String,
    /// e.g. "Q1 2026"
    pub quarter: String,
    /// One-line verdict / summary headline.
    pub headline: String,
    /// BUY / HOLD / SELL
    pub rating: String,
    /// Pre-formatted prior target price, e.g. "$150.00"
    pub target_price_old: String,
    /// Pre-formatted revised target price, e.g. "$165.00"
    pub target_price_new: String,
    /// Table data; first row is headers (Metric, Actual, Estimate, Beat/Miss).
    pub actuals_vs_estimates: Vec<Vec<String>>,
    /// 3-5 key takeaway bullets.
    pub key_takeaways: Vec<String>,
    /// 0-5 guidance change bullets (section omitted when empty).
    pub guidance_changes: Vec<String>,
    /// Table data; first row is headers (Metric, Old, New, Change).
    pub estimates_revisions: Vec<Vec<String>>,
    /// Multi-paragraph thesis update (split on blank lines).
    pub thesis_update: String,
    /// 0-5 forward catalysts (section omitted when empty).
    pub catalysts: Vec<String>,
    /// 0-5 key risks (section omitted when empty).
    pub risks: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Convert an [`EarningsUpdateInput`] into a [`crate::office::WordDocSpec`]
/// ready for [`crate::office::docx::write_word_doc`].
///
/// Optional sections are dropped entirely when their input vecs are empty:
/// - Guidance Changes (guidance_changes)
/// - Estimates Revisions (estimates_revisions — also skipped when the table is empty)
/// - Catalysts (catalysts)
/// - Risks (risks)
#[cfg(feature = "office")]
pub fn earnings_update_to_doc(input: &EarningsUpdateInput) -> crate::office::WordDocSpec {
    use crate::office::types::{DocSection, WordDocSpec, WorkbookProperties};

    let mut sections: Vec<DocSection> = Vec::new();

    sections.push(build_cover(input));
    sections.push(build_headline(input));
    sections.push(build_actuals_vs_estimates(input));
    sections.push(build_key_takeaways(input));

    if !input.guidance_changes.is_empty() {
        sections.push(build_bullet_section(
            "Guidance Changes",
            &input.guidance_changes,
        ));
    }

    if !input.estimates_revisions.is_empty() {
        sections.push(build_table_section(
            "Estimates Revisions",
            &input.estimates_revisions,
        ));
    }

    sections.push(build_thesis_update(input));

    if !input.catalysts.is_empty() {
        sections.push(build_bullet_section("Catalysts", &input.catalysts));
    }

    if !input.risks.is_empty() {
        sections.push(build_numbered_section("Risks", &input.risks));
    }

    sections.push(build_footer(input));

    WordDocSpec {
        sections,
        properties: WorkbookProperties {
            title: Some(format!(
                "{} {} Earnings Update",
                input.ticker, input.quarter
            )),
            author: Some(input.author.clone()),
            company: None,
            subject: Some("Equity Research — Earnings Update".into()),
        },
    }
}

// ---------------------------------------------------------------------------
// Section builders
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
fn build_cover(input: &EarningsUpdateInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 1,
                text: format!(
                    "{}: {} \u{2014} {} Earnings Update",
                    input.ticker, input.company, input.quarter
                ),
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: format!(
                        "{} | Target Price: {} \u{2192} {}",
                        input.rating, input.target_price_old, input.target_price_new
                    ),
                    bold: true,
                    italic: false,
                }],
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: input.date.clone(),
                    bold: false,
                    italic: true,
                }],
            },
        ],
    }
}

#[cfg(feature = "office")]
fn build_headline(input: &EarningsUpdateInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 2,
                text: "Headline".into(),
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: input.headline.clone(),
                    bold: true,
                    italic: false,
                }],
            },
        ],
    }
}

#[cfg(feature = "office")]
fn build_actuals_vs_estimates(input: &EarningsUpdateInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading {
        level: 2,
        text: "Actuals vs Estimates".into(),
    }];

    if input.actuals_vs_estimates.is_empty() {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun {
                text: "No actuals data provided.".into(),
                bold: false,
                italic: false,
            }],
        });
    } else {
        let headers = input.actuals_vs_estimates[0].clone();
        let rows = input.actuals_vs_estimates[1..].to_vec();
        blocks.push(DocBlock::Table { headers, rows });
    }

    DocSection { blocks }
}

#[cfg(feature = "office")]
fn build_key_takeaways(input: &EarningsUpdateInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading {
        level: 2,
        text: "Key Takeaways".into(),
    }];

    if input.key_takeaways.is_empty() {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun {
                text: "No takeaways provided.".into(),
                bold: false,
                italic: false,
            }],
        });
    } else {
        blocks.push(DocBlock::BulletList {
            items: input.key_takeaways.clone(),
        });
    }

    DocSection { blocks }
}

#[cfg(feature = "office")]
fn build_thesis_update(input: &EarningsUpdateInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading {
        level: 2,
        text: "Thesis Update".into(),
    }];

    for para in split_paragraphs(&input.thesis_update) {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun {
                text: para,
                bold: false,
                italic: false,
            }],
        });
    }

    DocSection { blocks }
}

/// Build a section with a level-2 heading followed by a BulletList.
#[cfg(feature = "office")]
fn build_bullet_section(heading: &str, items: &[String]) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 2,
                text: heading.to_owned(),
            },
            DocBlock::BulletList {
                items: items.to_vec(),
            },
        ],
    }
}

/// Build a section with a level-2 heading followed by a NumberedList.
#[cfg(feature = "office")]
fn build_numbered_section(heading: &str, items: &[String]) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 2,
                text: heading.to_owned(),
            },
            DocBlock::NumberedList {
                items: items.to_vec(),
            },
        ],
    }
}

/// Build a section with a level-2 heading followed by a Table.
/// First row of `table_data` is treated as headers; remainder are data rows.
#[cfg(feature = "office")]
fn build_table_section(
    heading: &str,
    table_data: &[Vec<String>],
) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    let headers = table_data[0].clone();
    let rows = table_data[1..].to_vec();

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 2,
                text: heading.to_owned(),
            },
            DocBlock::Table { headers, rows },
        ],
    }
}

#[cfg(feature = "office")]
fn build_footer(input: &EarningsUpdateInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    DocSection {
        blocks: vec![DocBlock::Paragraph {
            runs: vec![TextRun {
                text: format!("Analyst: {}", input.author),
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

    fn minimal_input() -> EarningsUpdateInput {
        EarningsUpdateInput {
            ticker: "ACME".into(),
            company: "Acme Corp".into(),
            date: "2026-05-08".into(),
            author: "Jane Analyst, CFA".into(),
            quarter: "Q1 2026".into(),
            headline: "Beats on revenue; raises FY guidance; thesis intact.".into(),
            rating: "BUY".into(),
            target_price_old: "$150.00".into(),
            target_price_new: "$165.00".into(),
            actuals_vs_estimates: vec![
                vec![
                    "Metric".into(),
                    "Actual".into(),
                    "Estimate".into(),
                    "Beat/Miss".into(),
                ],
                vec![
                    "Revenue".into(),
                    "$1.25B".into(),
                    "$1.18B".into(),
                    "Beat +6%".into(),
                ],
            ],
            key_takeaways: vec![
                "Revenue beat driven by cloud segment outperformance.".into(),
                "Operating margins expanded 120 bps YoY to 24.3%.".into(),
            ],
            guidance_changes: vec![],
            estimates_revisions: vec![
                vec!["Metric".into(), "Old".into(), "New".into(), "Change".into()],
                vec!["FY26 EPS".into(), "$4.50".into(), "$4.80".into(), "+6.7%".into()],
            ],
            thesis_update: "The Q1 beat reinforces our view that Acme is gaining durable market share in enterprise cloud.".into(),
            catalysts: vec!["Annual Investor Day — Q3 2026".into()],
            risks: vec![],
        }
    }

    #[test]
    fn earnings_update_to_doc_basic() {
        let input = minimal_input();
        let doc = earnings_update_to_doc(&input);

        // With 0 guidance changes, 0 risks, the sections are:
        // Cover, Headline, Actuals vs Estimates, Key Takeaways,
        // Estimates Revisions, Thesis Update, Catalysts, Footer = 8
        let count = doc.sections.len();
        assert!(
            (7..=10).contains(&count),
            "expected 7-10 sections, got {count}"
        );

        // First section's first block must be Heading level 1 containing the ticker.
        let first_block = &doc.sections[0].blocks[0];
        match first_block {
            DocBlock::Heading { level, text } => {
                assert_eq!(*level, 1, "cover heading must be level 1");
                assert!(
                    text.contains(&input.ticker),
                    "cover heading must contain ticker, got: {text}"
                );
            }
            other => panic!("expected Heading, got: {other:?}"),
        }

        // properties.title must contain both ticker and quarter.
        let title = doc.properties.title.as_deref().unwrap_or("");
        assert!(
            title.contains(&input.ticker),
            "properties.title must contain ticker; got: {title:?}"
        );
        assert!(
            title.contains(&input.quarter),
            "properties.title must contain quarter; got: {title:?}"
        );
    }

    #[test]
    fn earnings_update_to_doc_round_trips_through_writer() {
        use crate::office::docx::write_word_doc;
        use tempfile::tempdir;

        let input = minimal_input();
        let doc = earnings_update_to_doc(&input);

        let dir = tempdir().expect("tempdir creation failed");
        let path = dir.path().join("earnings_update.docx");

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
