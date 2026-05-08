//! Equity Research Initiation Report template — converts a [`ResearchInitInput`]
//! into a [`WordDocSpec`] in a single call.
//!
//! Gated on the `office` feature. No additional feature gates are required because
//! this template operates solely on caller-supplied, pre-formatted strings.

// ---------------------------------------------------------------------------
// Input struct
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct ResearchInitInput {
    pub ticker: String,
    pub company: String,
    pub date: String,
    pub rating: String,
    pub target_price: String,
    pub current_price: String,
    pub upside_pct: String,
    pub author: String,
    /// Multi-paragraph executive summary (split on blank lines).
    pub exec_summary: String,
    /// Multi-paragraph investment thesis (split on blank lines).
    pub investment_thesis: String,
    /// Multi-paragraph business description (split on blank lines).
    pub business_description: String,
    /// Table data; first inner Vec is the header row.
    pub financial_highlights: Vec<Vec<String>>,
    /// (label, value) pairs for the valuation summary table.
    pub valuation_summary: Vec<(String, String)>,
    pub catalysts: Vec<String>,
    pub risks: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Convert a [`ResearchInitInput`] into a [`crate::office::WordDocSpec`].
///
/// Optional sections (Catalysts, Risks) are omitted entirely when their input
/// vecs are empty, keeping the document lean.
#[cfg(feature = "office")]
pub fn research_init_to_doc(input: &ResearchInitInput) -> crate::office::WordDocSpec {
    use crate::office::types::{DocSection, WordDocSpec, WorkbookProperties};

    let mut sections: Vec<DocSection> = Vec::new();

    sections.push(build_cover(input));
    sections.push(build_text_section("Executive Summary", &input.exec_summary));
    sections.push(build_text_section("Investment Thesis", &input.investment_thesis));
    sections.push(build_text_section("Business Description", &input.business_description));
    sections.push(build_financial_highlights(input));
    sections.push(build_valuation_summary(input));

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
            title: Some(format!("{}: {} — Initiating Coverage", input.ticker, input.company)),
            author: Some(input.author.clone()),
            company: None,
            subject: Some("Equity Research — Initiating Coverage".into()),
        },
    }
}

// ---------------------------------------------------------------------------
// Section builders
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
fn build_cover(input: &ResearchInitInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 1,
                text: format!("{}: {}", input.ticker, input.company),
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: format!("Initiating Coverage — {}", input.rating),
                    bold: true,
                    italic: false,
                }],
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: format!(
                        "Target Price: {} (Current: {}, Upside: {})",
                        input.target_price, input.current_price, input.upside_pct
                    ),
                    bold: false,
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

/// Build a section with a level-2 heading followed by one paragraph per blank-line-separated chunk.
#[cfg(feature = "office")]
fn build_text_section(heading: &str, body: &str) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading {
        level: 2,
        text: heading.to_owned(),
    }];

    for para in split_paragraphs(body) {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun { text: para, bold: false, italic: false }],
        });
    }

    DocSection { blocks }
}

#[cfg(feature = "office")]
fn build_financial_highlights(input: &ResearchInitInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading {
        level: 2,
        text: "Financial Highlights".into(),
    }];

    if input.financial_highlights.is_empty() {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun {
                text: "No financial highlights provided.".into(),
                bold: false,
                italic: false,
            }],
        });
    } else {
        let headers = input.financial_highlights[0].clone();
        let rows: Vec<Vec<String>> = input.financial_highlights[1..].to_vec();
        blocks.push(DocBlock::Table { headers, rows });
    }

    DocSection { blocks }
}

#[cfg(feature = "office")]
fn build_valuation_summary(input: &ResearchInitInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    let rows: Vec<Vec<String>> = input
        .valuation_summary
        .iter()
        .map(|(label, value)| vec![label.clone(), value.clone()])
        .collect();

    DocSection {
        blocks: vec![
            DocBlock::Heading { level: 2, text: "Valuation Summary".into() },
            DocBlock::Table {
                headers: vec!["Metric".into(), "Value".into()],
                rows,
            },
        ],
    }
}

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

#[cfg(feature = "office")]
fn build_numbered_section(heading: &str, items: &[String]) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    DocSection {
        blocks: vec![
            DocBlock::Heading { level: 2, text: heading.to_owned() },
            DocBlock::NumberedList { items: items.to_vec() },
        ],
    }
}

#[cfg(feature = "office")]
fn build_footer(input: &ResearchInitInput) -> crate::office::types::DocSection {
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

    fn minimal_input() -> ResearchInitInput {
        ResearchInitInput {
            ticker: "ACME".into(),
            company: "Acme Corp".into(),
            date: "2026-05-08".into(),
            rating: "BUY".into(),
            target_price: "$185.00".into(),
            current_price: "$156.40".into(),
            upside_pct: "+18.3%".into(),
            author: "Jane Analyst".into(),
            exec_summary: "Strong fundamentals support initiation.\n\nRecurring revenue model de-risks near-term outlook.".into(),
            investment_thesis: "Market share gains in cloud segment drive upside.".into(),
            business_description: "Acme Corp is a diversified industrial conglomerate.".into(),
            financial_highlights: vec![
                vec!["Year".into(), "Revenue ($M)".into(), "EBITDA ($M)".into()],
                vec!["2025E".into(), "4,200".into(), "1,050".into()],
            ],
            valuation_summary: vec![
                ("DCF Fair Value".into(), "$192.50".into()),
            ],
            catalysts: vec![],
            risks: vec![],
        }
    }

    #[test]
    fn research_init_to_doc_basic() {
        let input = minimal_input();
        let doc = research_init_to_doc(&input);

        // Catalysts and Risks are empty → 7 sections: Cover, ExecSummary,
        // InvestmentThesis, BusinessDescription, FinancialHighlights,
        // ValuationSummary, Footer.
        let count = doc.sections.len();
        assert!(
            (5..=9).contains(&count),
            "expected 5-9 sections, got {count}"
        );

        // First section's first block must be Heading level 1 containing the company name.
        let first_block = &doc.sections[0].blocks[0];
        match first_block {
            DocBlock::Heading { level, text } => {
                assert_eq!(*level, 1, "cover heading must be level 1");
                assert!(
                    text.contains("Acme Corp"),
                    "cover heading must contain company name, got: {text}"
                );
            }
            other => panic!("expected Heading, got: {other:?}"),
        }

        // Properties title must contain the ticker.
        let title = doc.properties.title.as_deref().unwrap_or("");
        assert!(
            title.contains("ACME"),
            "properties.title must contain ticker, got: {title}"
        );
    }

    #[test]
    fn research_init_to_doc_round_trips_through_writer() {
        use crate::office::docx::write_word_doc;
        use std::path::PathBuf;
        use tempfile::tempdir;

        let input = minimal_input();
        let spec = research_init_to_doc(&input);

        let dir = tempdir().expect("tempdir creation failed");
        let path: PathBuf = dir.path().join("research_init.docx");

        let result = write_word_doc(&spec, &path).expect("write_word_doc failed");

        assert!(path.exists(), "output file does not exist");
        assert!(
            result.bytes_written > 0,
            "expected nonzero bytes, got {}",
            result.bytes_written
        );

        // Verify ZIP/OOXML magic bytes: PK\x03\x04
        let raw = std::fs::read(&path).expect("could not read output file");
        assert_eq!(
            &raw[..4],
            b"PK\x03\x04",
            "docx must start with ZIP magic bytes"
        );
    }
}
