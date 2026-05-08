//! IC Memo template — converts an [`IcMemoInput`] into a [`WordDocSpec`] in one call.
//!
//! Gated on the `office` feature only; no compute-result dependency.

#[cfg(feature = "office")]
use crate::office::types::{DocBlock, DocSection, TextRun, WordDocSpec, WorkbookProperties};

// ---------------------------------------------------------------------------
// Input struct
// ---------------------------------------------------------------------------

/// All data required to render an institutional IC Memorandum.
#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct IcMemoInput {
    pub deal_name: String,
    pub target_company: String,
    pub date: String,
    /// e.g. "APPROVE", "REJECT", "DISCUSS"
    pub recommendation: String,
    pub author: String,
    /// Multi-paragraph thesis; split on blank lines (`\n\n`).
    pub investment_thesis: String,
    /// (label, value) pairs for the key-metrics table.
    pub key_metrics: Vec<(String, String)>,
    /// Table data; first inner Vec is headers, rest are data rows.
    pub financial_summary: Vec<Vec<String>>,
    pub risks: Vec<String>,
    pub mitigants: Vec<String>,
    pub conclusion: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Convert an [`IcMemoInput`] into a [`WordDocSpec`] ready for
/// [`crate::office::docx::write_word_doc`].
///
/// Section layout:
/// 1. Cover — H1 title, bold deal/target, italic date, spacer paragraph.
/// 2. Recommendation — H2 + bold recommendation text.
/// 3. Investment Thesis — H2 + one Paragraph per blank-line-delimited segment.
/// 4. Key Metrics — H2 + 2-column Table.
/// 5. Financial Summary — H2 + Table (or fallback paragraph when empty).
/// 6. Risks — H2 + NumberedList (section omitted when empty).
/// 7. Mitigants — H2 + NumberedList (section omitted when empty).
/// 8. Conclusion — H2 + Paragraph.
/// 9. Footer — italic "Prepared by {author}" paragraph.
#[cfg(feature = "office")]
pub fn ic_memo_to_doc(input: &IcMemoInput) -> WordDocSpec {
    let mut sections: Vec<DocSection> = Vec::new();

    sections.push(build_cover(input));
    sections.push(build_recommendation(input));
    sections.push(build_thesis(input));
    sections.push(build_key_metrics(input));
    sections.push(build_financial_summary(input));

    if !input.risks.is_empty() {
        sections.push(build_list_section("Risks", &input.risks));
    }
    if !input.mitigants.is_empty() {
        sections.push(build_list_section("Mitigants", &input.mitigants));
    }

    sections.push(build_conclusion(input));
    sections.push(build_footer(input));

    WordDocSpec {
        sections,
        properties: WorkbookProperties {
            title: Some(format!("IC Memo \u{2014} {}", input.deal_name)),
            author: Some(input.author.clone()),
            company: None,
            subject: Some("Investment Committee Memorandum".into()),
        },
    }
}

// ---------------------------------------------------------------------------
// Section builders
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
fn build_cover(input: &IcMemoInput) -> DocSection {
    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 1,
                text: "INVESTMENT COMMITTEE MEMORANDUM".into(),
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: format!("{} \u{2014} {}", input.deal_name, input.target_company),
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
            // Spacer
            DocBlock::Paragraph { runs: vec![] },
        ],
    }
}

#[cfg(feature = "office")]
fn build_recommendation(input: &IcMemoInput) -> DocSection {
    DocSection {
        blocks: vec![
            DocBlock::Heading { level: 2, text: "Recommendation".into() },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: input.recommendation.clone(),
                    bold: true,
                    italic: false,
                }],
            },
        ],
    }
}

#[cfg(feature = "office")]
fn build_thesis(input: &IcMemoInput) -> DocSection {
    let mut blocks = vec![DocBlock::Heading { level: 2, text: "Investment Thesis".into() }];

    for segment in input.investment_thesis.split("\n\n") {
        let trimmed = segment.trim();
        if !trimmed.is_empty() {
            blocks.push(DocBlock::Paragraph {
                runs: vec![TextRun { text: trimmed.into(), bold: false, italic: false }],
            });
        }
    }

    DocSection { blocks }
}

#[cfg(feature = "office")]
fn build_key_metrics(input: &IcMemoInput) -> DocSection {
    let rows: Vec<Vec<String>> = input
        .key_metrics
        .iter()
        .map(|(label, value)| vec![label.clone(), value.clone()])
        .collect();

    DocSection {
        blocks: vec![
            DocBlock::Heading { level: 2, text: "Key Metrics".into() },
            DocBlock::Table {
                headers: vec!["Metric".into(), "Value".into()],
                rows,
            },
        ],
    }
}

#[cfg(feature = "office")]
fn build_financial_summary(input: &IcMemoInput) -> DocSection {
    let mut blocks = vec![DocBlock::Heading { level: 2, text: "Financial Summary".into() }];

    if input.financial_summary.is_empty() {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun {
                text: "No financial summary provided.".into(),
                bold: false,
                italic: false,
            }],
        });
    } else {
        let headers = input.financial_summary[0].clone();
        let rows = input.financial_summary[1..].to_vec();
        blocks.push(DocBlock::Table { headers, rows });
    }

    DocSection { blocks }
}

#[cfg(feature = "office")]
fn build_list_section(heading: &str, items: &[String]) -> DocSection {
    DocSection {
        blocks: vec![
            DocBlock::Heading { level: 2, text: heading.into() },
            DocBlock::NumberedList { items: items.to_vec() },
        ],
    }
}

#[cfg(feature = "office")]
fn build_conclusion(input: &IcMemoInput) -> DocSection {
    DocSection {
        blocks: vec![
            DocBlock::Heading { level: 2, text: "Conclusion".into() },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: input.conclusion.clone(),
                    bold: false,
                    italic: false,
                }],
            },
        ],
    }
}

#[cfg(feature = "office")]
fn build_footer(input: &IcMemoInput) -> DocSection {
    DocSection {
        blocks: vec![DocBlock::Paragraph {
            runs: vec![TextRun {
                text: format!("Prepared by {}", input.author),
                bold: false,
                italic: true,
            }],
        }],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    fn minimal_input() -> IcMemoInput {
        IcMemoInput {
            deal_name: "Project Atlas".into(),
            target_company: "Acme Corp".into(),
            date: "2026-05-08".into(),
            recommendation: "APPROVE".into(),
            author: "Jane Smith, CFA".into(),
            investment_thesis: "Acme Corp is the market leader in widget manufacturing.\n\nThe company benefits from strong free cash flow and a defensible moat.".into(),
            key_metrics: vec![("EV/EBITDA".into(), "8.5x".into())],
            financial_summary: vec![
                vec!["Year".into(), "Revenue".into(), "EBITDA".into()],
                vec!["2026E".into(), "$500M".into(), "$100M".into()],
            ],
            risks: vec![],
            mitigants: vec![],
            conclusion: "We recommend approval subject to final credit sign-off.".into(),
        }
    }

    #[test]
    fn ic_memo_to_doc_basic() {
        let input = minimal_input();
        let doc = ic_memo_to_doc(&input);

        // With no risks and no mitigants, sections are:
        // cover, recommendation, thesis, key_metrics, financial_summary,
        // conclusion, footer = 7 sections
        let count = doc.sections.len();
        assert!(
            (5..=9).contains(&count),
            "expected 5-9 sections, got {count}"
        );

        // First block of first section is Heading level 1
        let first_block = &doc.sections[0].blocks[0];
        assert!(
            matches!(
                first_block,
                DocBlock::Heading { level: 1, text }
                    if text == "INVESTMENT COMMITTEE MEMORANDUM"
            ),
            "first block must be H1 INVESTMENT COMMITTEE MEMORANDUM, got: {first_block:?}"
        );

        // properties.title contains the deal name
        let title = doc.properties.title.as_deref().unwrap_or("");
        assert!(
            title.contains(&input.deal_name),
            "properties.title should contain deal_name; got: {title:?}"
        );
    }

    #[test]
    fn ic_memo_to_doc_round_trips_through_writer() {
        use crate::office::docx::write_word_doc;
        use tempfile::tempdir;

        let input = minimal_input();
        let doc = ic_memo_to_doc(&input);

        let dir = tempdir().expect("tempdir creation failed");
        let path = dir.path().join("ic_memo.docx");

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
