//! CIM (Confidential Information Memorandum) template — converts a [`CimInput`]
//! into a [`WordDocSpec`] in one call.
//!
//! Gated on the `office` feature only; no compute-result dependency.

// ---------------------------------------------------------------------------
// Input struct
// ---------------------------------------------------------------------------

/// All data required to render a sell-side CIM (deal book).
#[cfg(feature = "office")]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CimInput {
    pub project_name: String,
    pub target_company: String,
    pub date: String,
    pub author: String,
    pub confidentiality_notice: String,
    /// Multi-paragraph; split on blank lines (`\n\n`).
    pub executive_summary: String,
    /// 4-8 investment-highlight bullets.
    pub investment_highlights: Vec<String>,
    /// Multi-paragraph; split on blank lines (`\n\n`).
    pub business_overview: String,
    /// Multi-paragraph; split on blank lines (`\n\n`).
    pub market_overview: String,
    /// Table data; first inner Vec is the header row.
    pub financial_summary: Vec<Vec<String>>,
    /// (name, title) pairs.
    pub management_team: Vec<(String, String)>,
    /// Multi-paragraph; split on blank lines (`\n\n`).
    pub transaction_overview: String,
    /// (milestone, date) pairs.
    pub process_timeline: Vec<(String, String)>,
    /// 4-6 key risks.
    pub key_risks: Vec<String>,
    /// Single paragraph.
    pub disclaimer: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Convert a [`CimInput`] into a [`crate::office::WordDocSpec`] ready for
/// [`crate::office::docx::write_word_doc`].
///
/// Section layout:
///  1. Cover — H1 title, bold project name, target company, italic date, PageBreak.
///  2. Confidentiality Notice — H2 + paragraphs + PageBreak.
///  3. Executive Summary — H2 + multi-paragraph body.
///  4. Investment Highlights — H2 + BulletList (omitted when empty).
///  5. Business Overview — H2 + multi-paragraph + PageBreak.
///  6. Market Overview — H2 + multi-paragraph.
///  7. Financial Summary — H2 + Table (or fallback paragraph when empty).
///  8. Management Team — H2 + Table with Name/Title columns (omitted when empty).
///  9. Transaction Overview — H2 + multi-paragraph + PageBreak.
/// 10. Process Timeline — H2 + Table with Milestone/Date columns (omitted when empty).
/// 11. Key Risks — H2 + NumberedList (omitted when empty).
/// 12. Disclaimer — H2 + italic body + italic "Prepared by {author}".
#[cfg(feature = "office")]
pub fn cim_to_doc(input: &CimInput) -> crate::office::WordDocSpec {
    use crate::office::types::{DocSection, WordDocSpec, WorkbookProperties};

    let mut sections: Vec<DocSection> = Vec::new();

    sections.push(build_cover(input));
    sections.push(build_confidentiality_notice(input));
    sections.push(build_text_section(
        "Executive Summary",
        &input.executive_summary,
    ));

    if !input.investment_highlights.is_empty() {
        sections.push(build_bullet_section(
            "Investment Highlights",
            &input.investment_highlights,
        ));
    }

    sections.push(build_text_section_with_page_break(
        "Business Overview",
        &input.business_overview,
    ));
    sections.push(build_text_section(
        "Market Overview",
        &input.market_overview,
    ));
    sections.push(build_financial_summary(input));

    if !input.management_team.is_empty() {
        sections.push(build_management_team(input));
    }

    sections.push(build_text_section_with_page_break(
        "Transaction Overview",
        &input.transaction_overview,
    ));

    if !input.process_timeline.is_empty() {
        sections.push(build_process_timeline(input));
    }

    if !input.key_risks.is_empty() {
        sections.push(build_numbered_section("Key Risks", &input.key_risks));
    }

    sections.push(build_disclaimer(input));

    WordDocSpec {
        sections,
        properties: WorkbookProperties {
            title: Some(format!("CIM \u{2014} {}", input.project_name)),
            author: Some(input.author.clone()),
            company: None,
            subject: Some("Confidential Information Memorandum".into()),
        },
    }
}

// ---------------------------------------------------------------------------
// Section builders
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
fn build_cover(input: &CimInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 1,
                text: "CONFIDENTIAL INFORMATION MEMORANDUM".into(),
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: input.project_name.clone(),
                    bold: true,
                    italic: false,
                }],
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: input.target_company.clone(),
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
            DocBlock::PageBreak,
        ],
    }
}

#[cfg(feature = "office")]
fn build_confidentiality_notice(input: &CimInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading {
        level: 2,
        text: "Confidentiality Notice".into(),
    }];

    for para in split_paragraphs(&input.confidentiality_notice) {
        blocks.push(DocBlock::Paragraph {
            runs: vec![TextRun {
                text: para,
                bold: false,
                italic: false,
            }],
        });
    }

    blocks.push(DocBlock::PageBreak);

    DocSection { blocks }
}

/// Build a level-2 section with one paragraph per blank-line-separated chunk.
#[cfg(feature = "office")]
fn build_text_section(heading: &str, body: &str) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading {
        level: 2,
        text: heading.to_owned(),
    }];

    for para in split_paragraphs(body) {
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

/// Like [`build_text_section`] but appends a [`DocBlock::PageBreak`] at the end.
#[cfg(feature = "office")]
fn build_text_section_with_page_break(
    heading: &str,
    body: &str,
) -> crate::office::types::DocSection {
    use crate::office::types::DocBlock;

    let mut section = build_text_section(heading, body);
    section.blocks.push(DocBlock::PageBreak);
    section
}

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

#[cfg(feature = "office")]
fn build_financial_summary(input: &CimInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    let mut blocks = vec![DocBlock::Heading {
        level: 2,
        text: "Financial Summary".into(),
    }];

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
fn build_management_team(input: &CimInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    let rows: Vec<Vec<String>> = input
        .management_team
        .iter()
        .map(|(name, title)| vec![name.clone(), title.clone()])
        .collect();

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 2,
                text: "Management Team".into(),
            },
            DocBlock::Table {
                headers: vec!["Name".into(), "Title".into()],
                rows,
            },
        ],
    }
}

#[cfg(feature = "office")]
fn build_process_timeline(input: &CimInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection};

    let rows: Vec<Vec<String>> = input
        .process_timeline
        .iter()
        .map(|(milestone, date)| vec![milestone.clone(), date.clone()])
        .collect();

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 2,
                text: "Process Timeline".into(),
            },
            DocBlock::Table {
                headers: vec!["Milestone".into(), "Date".into()],
                rows,
            },
        ],
    }
}

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

#[cfg(feature = "office")]
fn build_disclaimer(input: &CimInput) -> crate::office::types::DocSection {
    use crate::office::types::{DocBlock, DocSection, TextRun};

    DocSection {
        blocks: vec![
            DocBlock::Heading {
                level: 2,
                text: "Disclaimer".into(),
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: input.disclaimer.clone(),
                    bold: false,
                    italic: true,
                }],
            },
            DocBlock::Paragraph {
                runs: vec![TextRun {
                    text: format!("Prepared by {}", input.author),
                    bold: false,
                    italic: true,
                }],
            },
        ],
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

    fn minimal_input() -> CimInput {
        CimInput {
            project_name: "Project Falcon".into(),
            target_company: "Falcon Industries Ltd.".into(),
            date: "2026-05-08".into(),
            author: "Jane Banker, CFA".into(),
            confidentiality_notice: "This document is strictly confidential and is being furnished solely for informational purposes.".into(),
            executive_summary: "Falcon Industries is a leading manufacturer of precision components.\n\nThe company has achieved consistent double-digit revenue growth over the past five years.".into(),
            investment_highlights: vec!["Market leader with 35% share in core segment".into()],
            business_overview: "Founded in 1998, Falcon Industries operates across three business segments.".into(),
            market_overview: "The global precision components market is estimated at $12 billion and growing at 8% per annum.".into(),
            financial_summary: vec![
                vec!["Year".into(), "Revenue ($M)".into(), "EBITDA ($M)".into()],
                vec!["2025A".into(), "480".into(), "96".into()],
            ],
            management_team: vec![("John Smith".into(), "Chief Executive Officer".into())],
            transaction_overview: "The Company is seeking a strategic buyer or financial sponsor to support its next phase of growth.".into(),
            process_timeline: vec![("Management Presentations".into(), "June 2026".into())],
            key_risks: vec!["Customer concentration risk with top-3 customers representing ~45% of revenue".into()],
            disclaimer: "This Confidential Information Memorandum has been prepared by Falcon Advisory LLC solely for informational purposes.".into(),
        }
    }

    #[test]
    fn cim_to_doc_basic() {
        let input = minimal_input();
        let doc = cim_to_doc(&input);

        let count = doc.sections.len();
        assert!(
            (9..=12).contains(&count),
            "expected 9-12 sections, got {count}"
        );

        // First block of first section must be Heading level 1 with exact text.
        let first_block = &doc.sections[0].blocks[0];
        assert!(
            matches!(
                first_block,
                DocBlock::Heading { level: 1, text }
                    if text == "CONFIDENTIAL INFORMATION MEMORANDUM"
            ),
            "first block must be H1 'CONFIDENTIAL INFORMATION MEMORANDUM', got: {first_block:?}"
        );

        // properties.title must contain the project name.
        let title = doc.properties.title.as_deref().unwrap_or("");
        assert!(
            title.contains(&input.project_name),
            "properties.title should contain project_name; got: {title:?}"
        );
    }

    #[test]
    fn cim_to_doc_round_trips_through_writer() {
        use crate::office::docx::write_word_doc;
        use tempfile::tempdir;

        let input = minimal_input();
        let doc = cim_to_doc(&input);

        let dir = tempdir().expect("tempdir creation failed");
        let path = dir.path().join("cim.docx");

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
