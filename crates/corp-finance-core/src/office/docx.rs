use std::fmt::Write as FmtWrite;
use std::io::Cursor;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

use super::types::{DocBlock, DocSection, TextRun, WordDocSpec, WriteDocResult};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write a [`WordDocSpec`] to disk as a .docx file.
///
/// # Errors
///
/// - `InvalidInput` when `sections` is empty.
/// - `InvalidInput` when a `Heading` block has `level` not in `1..=3`.
/// - `SerializationError` on filesystem failures.
pub fn write_word_doc(spec: &WordDocSpec, output_path: &Path) -> CorpFinanceResult<WriteDocResult> {
    validate_spec(spec)?;

    let mut docx = docx_rs::Docx::new();

    // TODO: docx-rs 0.4 does not expose a direct document-properties setter on
    // Docx::new() for title/author/company/subject via a fluent API comparable
    // to rust_xlsxwriter's DocProperties. The `doc_props` field is set in the
    // private `Default` impl. Serialising into custom XML properties is
    // possible via `custom_property()` but that writes to a separate XML part
    // that Word treats differently. We skip document-property injection here
    // to avoid producing a malformed .docx; a follow-up can add this via the
    // `custom_property` builder when the properties are needed by downstream
    // tooling.

    // Bullet/numbered lists need AbstractNumbering + Numbering definitions.
    // We pre-register two at ids 1 (bullet) and 2 (decimal) when any such
    // block exists in the spec.
    let needs_bullet = spec.sections.iter().any(|s| {
        s.blocks
            .iter()
            .any(|b| matches!(b, DocBlock::BulletList { .. }))
    });
    let needs_numbered = spec.sections.iter().any(|s| {
        s.blocks
            .iter()
            .any(|b| matches!(b, DocBlock::NumberedList { .. }))
    });

    if needs_bullet {
        docx = register_bullet_numbering(docx, 1);
    }
    if needs_numbered {
        docx = register_decimal_numbering(docx, 2);
    }

    let section_count = spec.sections.len();

    for section in &spec.sections {
        docx = write_section(docx, section)?;
    }

    let xml_docx = docx.build();
    let mut buf = Cursor::new(Vec::new());
    xml_docx
        .pack(&mut buf)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;

    let bytes = buf.into_inner();

    std::fs::write(output_path, &bytes)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;

    let sha256 = sha256_bytes(&bytes);
    let bytes_written = bytes.len() as u64;

    Ok(WriteDocResult {
        output_path: output_path.to_path_buf(),
        bytes_written,
        sha256,
        section_count,
    })
}

/// Write directly from a JSON-string spec. Convenience for NAPI / CLI callers.
pub fn write_word_doc_from_json(
    spec_json: &str,
    output_path: &Path,
) -> CorpFinanceResult<WriteDocResult> {
    let spec: WordDocSpec = serde_json::from_str(spec_json)?;
    write_word_doc(&spec, output_path)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_spec(spec: &WordDocSpec) -> CorpFinanceResult<()> {
    if spec.sections.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "sections".into(),
            reason: "document must contain at least one section".into(),
        });
    }
    for section in &spec.sections {
        for block in &section.blocks {
            if let DocBlock::Heading { level, .. } = block {
                if !(1..=3).contains(level) {
                    return Err(CorpFinanceError::InvalidInput {
                        field: "heading.level".into(),
                        reason: format!(
                            "heading level {} is out of range; must be 1, 2, or 3",
                            level
                        ),
                    });
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Numbering registration helpers
// ---------------------------------------------------------------------------

fn register_bullet_numbering(docx: docx_rs::Docx, abstract_id: usize) -> docx_rs::Docx {
    let abstract_num = docx_rs::AbstractNumbering::new(abstract_id).add_level(
        docx_rs::Level::new(
            0,
            docx_rs::Start::new(1),
            docx_rs::NumberFormat::new("bullet"),
            docx_rs::LevelText::new("\u{2022}"),
            docx_rs::LevelJc::new("left"),
        )
        .indent(Some(720), Some(docx_rs::SpecialIndentType::Hanging(360)), Some(360), None),
    );
    let num = docx_rs::Numbering::new(abstract_id, abstract_id);
    docx.add_abstract_numbering(abstract_num)
        .add_numbering(num)
}

fn register_decimal_numbering(docx: docx_rs::Docx, abstract_id: usize) -> docx_rs::Docx {
    let abstract_num = docx_rs::AbstractNumbering::new(abstract_id).add_level(
        docx_rs::Level::new(
            0,
            docx_rs::Start::new(1),
            docx_rs::NumberFormat::new("decimal"),
            docx_rs::LevelText::new("%1."),
            docx_rs::LevelJc::new("left"),
        )
        .indent(Some(720), Some(docx_rs::SpecialIndentType::Hanging(360)), Some(360), None),
    );
    let num = docx_rs::Numbering::new(abstract_id, abstract_id);
    docx.add_abstract_numbering(abstract_num)
        .add_numbering(num)
}

// ---------------------------------------------------------------------------
// Section writing
// ---------------------------------------------------------------------------

fn write_section(
    mut docx: docx_rs::Docx,
    section: &DocSection,
) -> CorpFinanceResult<docx_rs::Docx> {
    for block in &section.blocks {
        docx = write_block(docx, block)?;
    }
    Ok(docx)
}

// ---------------------------------------------------------------------------
// Block writing
// ---------------------------------------------------------------------------

fn write_block(
    docx: docx_rs::Docx,
    block: &DocBlock,
) -> CorpFinanceResult<docx_rs::Docx> {
    match block {
        DocBlock::Heading { level, text } => Ok(write_heading(docx, *level, text)),
        DocBlock::Paragraph { runs } => Ok(write_paragraph(docx, runs)),
        DocBlock::Table { headers, rows } => Ok(write_table(docx, headers, rows)),
        DocBlock::BulletList { items } => Ok(write_bullet_list(docx, items, 1)),
        DocBlock::NumberedList { items } => Ok(write_numbered_list(docx, items, 2)),
        DocBlock::PageBreak => Ok(write_page_break(docx)),
    }
}

fn write_heading(docx: docx_rs::Docx, level: u8, text: &str) -> docx_rs::Docx {
    let style_id = format!("Heading{level}");
    let para = docx_rs::Paragraph::new()
        .style(&style_id)
        .add_run(docx_rs::Run::new().add_text(text));
    docx.add_paragraph(para)
}

fn write_paragraph(docx: docx_rs::Docx, runs: &[TextRun]) -> docx_rs::Docx {
    let mut para = docx_rs::Paragraph::new();
    for run in runs {
        let mut r = docx_rs::Run::new().add_text(run.text.as_str());
        if run.bold {
            r = r.bold();
        }
        if run.italic {
            r = r.italic();
        }
        para = para.add_run(r);
    }
    docx.add_paragraph(para)
}

fn write_table(
    docx: docx_rs::Docx,
    headers: &[String],
    rows: &[Vec<String>],
) -> docx_rs::Docx {
    let mut table_rows: Vec<docx_rs::TableRow> = Vec::new();

    // Header row — bold text
    if !headers.is_empty() {
        let cells: Vec<docx_rs::TableCell> = headers
            .iter()
            .map(|h| {
                let para = docx_rs::Paragraph::new()
                    .add_run(docx_rs::Run::new().add_text(h.as_str()).bold());
                docx_rs::TableCell::new().add_paragraph(para)
            })
            .collect();
        table_rows.push(docx_rs::TableRow::new(cells));
    }

    // Data rows
    for row in rows {
        let cells: Vec<docx_rs::TableCell> = row
            .iter()
            .map(|cell_text| {
                let para = docx_rs::Paragraph::new()
                    .add_run(docx_rs::Run::new().add_text(cell_text.as_str()));
                docx_rs::TableCell::new().add_paragraph(para)
            })
            .collect();
        table_rows.push(docx_rs::TableRow::new(cells));
    }

    docx.add_table(docx_rs::Table::new(table_rows))
}

fn write_bullet_list(
    mut docx: docx_rs::Docx,
    items: &[String],
    numbering_id: usize,
) -> docx_rs::Docx {
    for item in items {
        let para = docx_rs::Paragraph::new()
            .numbering(
                docx_rs::NumberingId::new(numbering_id),
                docx_rs::IndentLevel::new(0),
            )
            .add_run(docx_rs::Run::new().add_text(item.as_str()));
        docx = docx.add_paragraph(para);
    }
    docx
}

fn write_numbered_list(
    mut docx: docx_rs::Docx,
    items: &[String],
    numbering_id: usize,
) -> docx_rs::Docx {
    for item in items {
        let para = docx_rs::Paragraph::new()
            .numbering(
                docx_rs::NumberingId::new(numbering_id),
                docx_rs::IndentLevel::new(0),
            )
            .add_run(docx_rs::Run::new().add_text(item.as_str()));
        docx = docx.add_paragraph(para);
    }
    docx
}

fn write_page_break(docx: docx_rs::Docx) -> docx_rs::Docx {
    let para = docx_rs::Paragraph::new()
        .add_run(docx_rs::Run::new().add_break(docx_rs::BreakType::Page));
    docx.add_paragraph(para)
}

// ---------------------------------------------------------------------------
// SHA-256 helper
// ---------------------------------------------------------------------------

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Read;

    use tempfile::TempDir;

    use super::*;
    use crate::office::types::{
        DocBlock, DocSection, TextRun, WordDocSpec, WorkbookProperties,
    };

    fn minimal_spec() -> WordDocSpec {
        WordDocSpec {
            sections: vec![DocSection {
                blocks: vec![
                    DocBlock::Heading {
                        level: 1,
                        text: "Executive Summary".into(),
                    },
                    DocBlock::Paragraph {
                        runs: vec![TextRun {
                            text: "This is the body text.".into(),
                            bold: false,
                            italic: false,
                        }],
                    },
                ],
            }],
            properties: WorkbookProperties::default(),
        }
    }

    fn assert_sha256_format(s: &str) {
        assert_eq!(s.len(), 64, "sha256 should be 64 chars");
        assert!(
            s.chars().all(|c| c.is_ascii_hexdigit()),
            "sha256 should be hex"
        );
        assert_eq!(&s.to_lowercase(), s, "sha256 should be lowercase");
    }

    fn zip_contains_bytes(path: &std::path::Path, needle: &[u8]) -> bool {
        let file = std::fs::File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).unwrap();
            if contents.windows(needle.len()).any(|w| w == needle) {
                return true;
            }
        }
        false
    }

    #[test]
    fn write_minimal_doc_one_heading_one_paragraph() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("minimal.docx");
        let spec = minimal_spec();

        let result = write_word_doc(&spec, &path).unwrap();

        assert!(result.bytes_written > 0, "should write non-zero bytes");
        assert_eq!(result.bytes_written, std::fs::metadata(&path).unwrap().len());
        assert_sha256_format(&result.sha256);
        assert_eq!(result.section_count, 1);
    }

    #[test]
    fn write_doc_with_all_three_heading_levels() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("headings.docx");
        let spec = WordDocSpec {
            sections: vec![DocSection {
                blocks: vec![
                    DocBlock::Heading { level: 1, text: "Level 1".into() },
                    DocBlock::Heading { level: 2, text: "Level 2".into() },
                    DocBlock::Heading { level: 3, text: "Level 3".into() },
                ],
            }],
            properties: WorkbookProperties::default(),
        };

        let result = write_word_doc(&spec, &path).unwrap();

        assert!(result.bytes_written > 0);
        assert_sha256_format(&result.sha256);
        // The heading style IDs appear in document.xml
        assert!(zip_contains_bytes(&path, b"Heading1"), "Heading1 style should appear");
        assert!(zip_contains_bytes(&path, b"Heading2"), "Heading2 style should appear");
        assert!(zip_contains_bytes(&path, b"Heading3"), "Heading3 style should appear");
    }

    #[test]
    fn write_doc_rejects_empty_sections() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.docx");
        let spec = WordDocSpec {
            sections: vec![],
            properties: WorkbookProperties::default(),
        };

        let err = write_word_doc(&spec, &path).unwrap_err();
        assert!(
            matches!(err, CorpFinanceError::InvalidInput { .. }),
            "expected InvalidInput, got: {err}"
        );
    }

    #[test]
    fn write_doc_rejects_invalid_heading_level() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("badlevel.docx");

        for bad_level in [0u8, 4u8, 99u8] {
            let spec = WordDocSpec {
                sections: vec![DocSection {
                    blocks: vec![DocBlock::Heading {
                        level: bad_level,
                        text: "Bad".into(),
                    }],
                }],
                properties: WorkbookProperties::default(),
            };
            let err = write_word_doc(&spec, &path).unwrap_err();
            assert!(
                matches!(err, CorpFinanceError::InvalidInput { .. }),
                "expected InvalidInput for level {bad_level}, got: {err}"
            );
        }
    }

    #[test]
    fn write_doc_with_bold_and_italic_runs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("styled_runs.docx");
        let spec = WordDocSpec {
            sections: vec![DocSection {
                blocks: vec![DocBlock::Paragraph {
                    runs: vec![
                        TextRun { text: "Bold text".into(), bold: true, italic: false },
                        TextRun { text: " italic text".into(), bold: false, italic: true },
                    ],
                }],
            }],
            properties: WorkbookProperties::default(),
        };

        let result = write_word_doc(&spec, &path).unwrap();

        assert!(result.bytes_written > 0);
        assert_sha256_format(&result.sha256);
        // Bold marker appears in document.xml as <w:b/>
        assert!(zip_contains_bytes(&path, b"<w:b"), "bold marker should appear in docx xml");
        // Italic marker appears as <w:i/>
        assert!(zip_contains_bytes(&path, b"<w:i"), "italic marker should appear in docx xml");
    }

    #[test]
    fn write_doc_with_table_headers_and_rows() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("table.docx");
        let spec = WordDocSpec {
            sections: vec![DocSection {
                blocks: vec![DocBlock::Table {
                    headers: vec!["Company".into(), "EV ($M)".into(), "EBITDA".into()],
                    rows: vec![
                        vec!["ACME Corp".into(), "1200".into(), "120".into()],
                        vec!["Beta Inc".into(), "850".into(), "95".into()],
                    ],
                }],
            }],
            properties: WorkbookProperties::default(),
        };

        let result = write_word_doc(&spec, &path).unwrap();

        assert!(result.bytes_written > 0);
        assert_sha256_format(&result.sha256);
        assert!(zip_contains_bytes(&path, b"ACME Corp"), "table data should appear in docx xml");
        assert!(zip_contains_bytes(&path, b"Company"), "table header should appear in docx xml");
    }

    #[test]
    fn write_doc_with_bullet_list() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bullets.docx");
        let spec = WordDocSpec {
            sections: vec![DocSection {
                blocks: vec![DocBlock::BulletList {
                    items: vec![
                        "Revenue growth of 15%".into(),
                        "EBITDA margin expansion".into(),
                        "Deleveraging to 3.0x".into(),
                    ],
                }],
            }],
            properties: WorkbookProperties::default(),
        };

        let result = write_word_doc(&spec, &path).unwrap();

        assert!(result.bytes_written > 0);
        assert_sha256_format(&result.sha256);
        assert!(
            zip_contains_bytes(&path, b"Revenue growth"),
            "bullet item text should appear"
        );
    }

    #[test]
    fn write_doc_with_numbered_list() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("numbered.docx");
        let spec = WordDocSpec {
            sections: vec![DocSection {
                blocks: vec![DocBlock::NumberedList {
                    items: vec![
                        "First due diligence step".into(),
                        "Second management interview".into(),
                        "Third financial model review".into(),
                    ],
                }],
            }],
            properties: WorkbookProperties::default(),
        };

        let result = write_word_doc(&spec, &path).unwrap();

        assert!(result.bytes_written > 0);
        assert_sha256_format(&result.sha256);
        assert!(
            zip_contains_bytes(&path, b"First due diligence"),
            "numbered item text should appear"
        );
    }

    #[test]
    fn write_doc_with_page_break_between_sections() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("page_break.docx");
        let spec = WordDocSpec {
            sections: vec![
                DocSection {
                    blocks: vec![
                        DocBlock::Heading { level: 1, text: "Section One".into() },
                        DocBlock::PageBreak,
                    ],
                },
                DocSection {
                    blocks: vec![DocBlock::Heading {
                        level: 1,
                        text: "Section Two".into(),
                    }],
                },
            ],
            properties: WorkbookProperties::default(),
        };

        let result = write_word_doc(&spec, &path).unwrap();

        assert!(result.bytes_written > 0);
        assert_sha256_format(&result.sha256);
        assert_eq!(result.section_count, 2);
        // Page break type marker appears in document.xml
        assert!(
            zip_contains_bytes(&path, b"page"),
            "page break type should appear in docx xml"
        );
    }

    #[test]
    fn write_doc_sha256_stability() {
        let dir1 = TempDir::new().unwrap();
        let path1 = dir1.path().join("stable1.docx");
        let dir2 = TempDir::new().unwrap();
        let path2 = dir2.path().join("stable2.docx");
        let spec = minimal_spec();

        let r1 = write_word_doc(&spec, &path1).unwrap();
        let r2 = write_word_doc(&spec, &path2).unwrap();

        assert_eq!(
            r1.sha256, r2.sha256,
            "identical spec should produce identical sha256"
        );
    }

    #[test]
    fn write_doc_zip_magic_bytes() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("magic.docx");
        let spec = minimal_spec();

        write_word_doc(&spec, &path).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            &bytes[..4],
            b"PK\x03\x04",
            "docx must begin with ZIP magic bytes PK\\x03\\x04"
        );
    }
}
