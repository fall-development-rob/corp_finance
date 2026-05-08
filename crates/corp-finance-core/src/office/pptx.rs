//! Phase 29 Wave 8 — PowerPoint (.pptx) OOXML serialisation.
//!
//! # Crate decision: hand-rolled (zip + quick-xml)
//!
//! Evaluated candidates on 2026-05-08:
//! - ppt-rs 0.2.12 — most capable Rust pptx crate (~17k downloads, April 2026).
//!   REJECTED: `src/generator/props_xml.rs:5` calls `chrono::Utc::now()` which
//!   embeds the current timestamp in every output, breaking the sha256-stability
//!   invariant (identical spec → byte-identical output → identical hash).
//! - pptx 0.1.0, pptx-rs 0.1.0 — minimal/alpha-quality, not suitable.
//!
//! Decision: hand-roll using `zip` (already an optional dep) + `quick-xml`
//! (new optional dep added under the `office` feature). All XML is
//! assembled from deterministic string templates; no timestamps, UUIDs, or
//! rand calls. Output is byte-identical for the same spec on every run.
//!
//! # v1 scope
//!
//! Supported slide kinds:
//! - `Title`   — large title + optional subtitle (CenteredTitle layout)
//! - `Section` — section divider (CenteredTitle layout, bold heading)
//! - `Content` — title + bulleted list (TitleAndContent layout)
//! - `Table`   — title + simple table (TitleOnly layout with table shape)
//!
//! Out of v1 scope (document in this comment so callers set expectations):
//! images, charts, transitions, animations, speaker notes, theme customisation,
//! column-span/row-span in tables, slide masters beyond the embedded minimal
//! default, hyperlinks, fonts beyond Calibri.

use std::fmt::Write as FmtWrite;
use std::io::Cursor;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

use super::types::{Slide, SlideDeckSpec, WriteDeckResult};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write a [`SlideDeckSpec`] to disk as a .pptx file.
///
/// # Errors
///
/// - `InvalidInput` when `slides` is empty.
/// - `InvalidInput` when a `Title` slide has an empty title.
/// - `InvalidInput` when a `Table` slide has zero columns.
/// - `SerializationError` on filesystem failures.
pub fn write_slide_deck(
    spec: &SlideDeckSpec,
    output_path: &Path,
) -> CorpFinanceResult<WriteDeckResult> {
    validate_spec(spec)?;

    let bytes = build_pptx(spec)?;

    std::fs::write(output_path, &bytes)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;

    let sha256 = sha256_bytes(&bytes);
    let bytes_written = bytes.len() as u64;
    let slide_count = spec.slides.len();

    Ok(WriteDeckResult {
        output_path: output_path.to_path_buf(),
        bytes_written,
        sha256,
        slide_count,
    })
}

/// Write directly from a JSON-string spec. Convenience for NAPI / CLI callers.
pub fn write_slide_deck_from_json(
    spec_json: &str,
    output_path: &Path,
) -> CorpFinanceResult<WriteDeckResult> {
    let spec: SlideDeckSpec = serde_json::from_str(spec_json)?;
    write_slide_deck(&spec, output_path)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_spec(spec: &SlideDeckSpec) -> CorpFinanceResult<()> {
    if spec.slides.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "slides".into(),
            reason: "deck must contain at least one slide".into(),
        });
    }
    for (i, slide) in spec.slides.iter().enumerate() {
        match slide {
            Slide::Title { title, .. } if title.is_empty() => {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("slides[{i}].title"),
                    reason: "title slide title must not be empty".into(),
                });
            }
            Slide::Table { headers, .. } if headers.is_empty() => {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("slides[{i}].headers"),
                    reason: "table slide must have at least one column".into(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ZIP / PPTX builder
// ---------------------------------------------------------------------------

/// Assemble all OOXML parts into a .pptx zip in memory.
fn build_pptx(spec: &SlideDeckSpec) -> CorpFinanceResult<Vec<u8>> {
    use zip::write::{ExtendedFileOptions, FileOptions};
    use zip::CompressionMethod;

    let buf = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);

    let stored: FileOptions<ExtendedFileOptions> =
        FileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated: FileOptions<ExtendedFileOptions> =
        FileOptions::default().compression_method(CompressionMethod::Deflated);

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", stored.clone())
        .map_err(zip_err)?;
    zip.write_all(content_types_xml(spec.slides.len()).as_bytes())
        .map_err(zip_err)?;

    // _rels/.rels
    zip.start_file("_rels/.rels", stored.clone())
        .map_err(zip_err)?;
    zip.write_all(ROOT_RELS.as_bytes()).map_err(zip_err)?;

    // ppt/_rels/presentation.xml.rels
    zip.start_file("ppt/_rels/presentation.xml.rels", stored.clone())
        .map_err(zip_err)?;
    zip.write_all(presentation_rels_xml(spec.slides.len()).as_bytes())
        .map_err(zip_err)?;

    // ppt/presentation.xml
    zip.start_file("ppt/presentation.xml", deflated.clone())
        .map_err(zip_err)?;
    zip.write_all(presentation_xml(spec.slides.len()).as_bytes())
        .map_err(zip_err)?;

    // ppt/theme/theme1.xml
    zip.start_file("ppt/theme/theme1.xml", deflated.clone())
        .map_err(zip_err)?;
    zip.write_all(THEME_XML.as_bytes()).map_err(zip_err)?;

    // ppt/slideMasters/slideMaster1.xml
    zip.start_file("ppt/slideMasters/slideMaster1.xml", deflated.clone())
        .map_err(zip_err)?;
    zip.write_all(SLIDE_MASTER_XML.as_bytes())
        .map_err(zip_err)?;

    // ppt/slideMasters/_rels/slideMaster1.xml.rels
    zip.start_file(
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        stored.clone(),
    )
    .map_err(zip_err)?;
    zip.write_all(SLIDE_MASTER_RELS.as_bytes())
        .map_err(zip_err)?;

    // ppt/slideLayouts/slideLayout1.xml
    zip.start_file("ppt/slideLayouts/slideLayout1.xml", deflated.clone())
        .map_err(zip_err)?;
    zip.write_all(SLIDE_LAYOUT_XML.as_bytes())
        .map_err(zip_err)?;

    // ppt/slideLayouts/_rels/slideLayout1.xml.rels
    zip.start_file(
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        stored.clone(),
    )
    .map_err(zip_err)?;
    zip.write_all(SLIDE_LAYOUT_RELS.as_bytes())
        .map_err(zip_err)?;

    // Per-slide XML + rels
    for (idx, slide) in spec.slides.iter().enumerate() {
        let slide_num = idx + 1;
        let slide_xml = render_slide(slide, slide_num);
        let slide_rels = slide_rels_xml();

        zip.start_file(format!("ppt/slides/slide{slide_num}.xml"), deflated.clone())
            .map_err(zip_err)?;
        zip.write_all(slide_xml.as_bytes()).map_err(zip_err)?;

        zip.start_file(
            format!("ppt/slides/_rels/slide{slide_num}.xml.rels"),
            stored.clone(),
        )
        .map_err(zip_err)?;
        zip.write_all(slide_rels.as_bytes()).map_err(zip_err)?;
    }

    let cursor = zip.finish().map_err(zip_err)?;
    Ok(cursor.into_inner())
}

fn zip_err(e: impl std::fmt::Display) -> CorpFinanceError {
    CorpFinanceError::SerializationError(e.to_string())
}

use std::io::Write;

// ---------------------------------------------------------------------------
// XML generation — package-level parts
// ---------------------------------------------------------------------------

fn content_types_xml(slide_count: usize) -> String {
    let mut s = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
"#,
    );
    for i in 1..=slide_count {
        let _ = write!(
            s,
            r#"  <Override PartName="/ppt/slides/slide{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
"#
        );
    }
    s.push_str("</Types>");
    s
}

static ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#;

fn presentation_rels_xml(slide_count: usize) -> String {
    let mut s = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId0" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
"#,
    );
    for i in 1..=slide_count {
        let rid = i + 1;
        let _ = write!(
            s,
            r#"  <Relationship Id="rId{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{i}.xml"/>
"#
        );
    }
    s.push_str("</Relationships>");
    s
}

fn presentation_xml(slide_count: usize) -> String {
    let slide_id_lst: String = (1..=slide_count)
        .map(|i| {
            let id = 255 + i as u32;
            let rid = i + 1;
            format!(r#"    <p:sldId id="{id}" r:id="rId{rid}"/>"#)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
  xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  saveSubsetFonts="1">
  <p:sldMasterIdLst>
    <p:sldMasterId id="2147483648" r:id="rId1"/>
  </p:sldMasterIdLst>
  <p:sldIdLst>
{slide_id_lst}
  </p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000" type="screen4x3"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#
    )
}

// ---------------------------------------------------------------------------
// Per-slide XML
// ---------------------------------------------------------------------------

fn slide_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#.to_string()
}

fn render_slide(slide: &Slide, _num: usize) -> String {
    match slide {
        Slide::Title { title, subtitle } => render_title_slide(title, subtitle.as_deref()),
        Slide::Section { heading } => render_section_slide(heading),
        Slide::Content { title, bullets } => render_content_slide(title, bullets),
        Slide::Table {
            title,
            headers,
            rows,
        } => render_table_slide(title, headers, rows),
    }
}

/// Wrap raw XML in the standard <p:sld> envelope.
fn slide_envelope(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
  xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr>
        <a:xfrm>
          <a:off x="0" y="0"/>
          <a:ext cx="9144000" cy="6858000"/>
          <a:chOff x="0" y="0"/>
          <a:chExt cx="9144000" cy="6858000"/>
        </a:xfrm>
      </p:grpSpPr>
{body}
    </p:spTree>
  </p:cSld>
</p:sld>"#
    )
}

/// Build a text shape XML fragment.
fn text_shape(
    id: u32,
    name: &str,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    bold: bool,
    size_pt: u32,
    text: &str,
) -> String {
    let b_tag = if bold { "<a:b/>" } else { "" };
    let escaped = xml_escape(text);
    format!(
        r#"      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="{id}" name="{name}"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
          <a:noFill/>
        </p:spPr>
        <p:txBody>
          <a:bodyPr wrap="square" rtlCol="0"/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="en-US" sz="{size_pt}" dirty="0">{b_tag}</a:rPr>
              <a:t>{escaped}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>"#
    )
}

/// Build a bullet-list shape with multiple paragraphs.
fn bullet_shape(id: u32, x: i64, y: i64, cx: i64, cy: i64, bullets: &[String]) -> String {
    let paras: String = bullets
        .iter()
        .map(|b| {
            let escaped = xml_escape(b);
            format!(
                r#"          <a:p>
            <a:pPr>
              <a:buChar char="&#x2022;"/>
            </a:pPr>
            <a:r>
              <a:rPr lang="en-US" sz="2000" dirty="0"/>
              <a:t>{escaped}</a:t>
            </a:r>
          </a:p>"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="{id}" name="Content"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>
          <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
          <a:noFill/>
        </p:spPr>
        <p:txBody>
          <a:bodyPr wrap="square" rtlCol="0"/>
          <a:lstStyle/>
{paras}
        </p:txBody>
      </p:sp>"#
    )
}

fn render_title_slide(title: &str, subtitle: Option<&str>) -> String {
    let title_shape = text_shape(
        2, "Title", 457200, 1600200, 8229600, 1600200, true, 4400, title,
    );
    let sub_shape = subtitle
        .map(|s| {
            text_shape(
                3, "Subtitle", 457200, 3200400, 8229600, 1371600, false, 2800, s,
            )
        })
        .unwrap_or_default();
    slide_envelope(&format!("{title_shape}\n{sub_shape}"))
}

fn render_section_slide(heading: &str) -> String {
    let shape = text_shape(
        2, "Heading", 457200, 2400000, 8229600, 2058000, true, 3600, heading,
    );
    slide_envelope(&shape)
}

fn render_content_slide(title: &str, bullets: &[String]) -> String {
    let title_shape = text_shape(
        2, "Title", 457200, 274638, 8229600, 1143000, true, 3600, title,
    );
    let content_shape = if bullets.is_empty() {
        String::new()
    } else {
        bullet_shape(3, 457200, 1600200, 8229600, 4525963, bullets)
    };
    slide_envelope(&format!("{title_shape}\n{content_shape}"))
}

fn render_table_slide(title: &str, headers: &[String], rows: &[Vec<String>]) -> String {
    let title_shape = text_shape(
        2, "Title", 457200, 274638, 8229600, 1143000, true, 3600, title,
    );
    let table_shape = render_table_shape(headers, rows);
    slide_envelope(&format!("{title_shape}\n{table_shape}"))
}

fn render_table_shape(headers: &[String], rows: &[Vec<String>]) -> String {
    let col_count = headers.len();
    // Distribute width evenly across slide width (8229600 EMU).
    let col_width = if col_count > 0 {
        8229600i64 / col_count as i64
    } else {
        8229600
    };

    let col_widths_xml: String = (0..col_count)
        .map(|_| format!(r#"          <a:gridCol w="{col_width}"/>"#))
        .collect::<Vec<_>>()
        .join("\n");

    let header_row = table_row_xml(headers, true);
    let data_rows: String = rows
        .iter()
        .map(|r| table_row_xml(r, false))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"      <p:graphicFrame>
        <p:nvGraphicFramePr>
          <p:cNvPr id="3" name="Table"/>
          <p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1"/></p:cNvGraphicFramePr>
          <p:nvPr/>
        </p:nvGraphicFramePr>
        <p:xfrm>
          <a:off x="457200" y="1600200"/>
          <a:ext cx="8229600" cy="4525963"/>
        </p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
            <a:tbl>
              <a:tblPr firstRow="1"/>
              <a:tblGrid>
{col_widths_xml}
              </a:tblGrid>
{header_row}
{data_rows}
            </a:tbl>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>"#
    )
}

fn table_row_xml(cells: &[String], is_header: bool) -> String {
    let b_tag = if is_header { "<a:b/>" } else { "" };
    let cell_xmls: String = cells
        .iter()
        .map(|c| {
            let escaped = xml_escape(c);
            format!(
                r#"              <a:tc>
                <a:txBody>
                  <a:bodyPr/>
                  <a:lstStyle/>
                  <a:p>
                    <a:r>
                      <a:rPr lang="en-US" sz="1600" dirty="0">{b_tag}</a:rPr>
                      <a:t>{escaped}</a:t>
                    </a:r>
                  </a:p>
                </a:txBody>
                <a:tcPr/>
              </a:tc>"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("            <a:tr h=\"370840\">\n{cell_xmls}\n            </a:tr>")
}

// ---------------------------------------------------------------------------
// XML escape helper
// ---------------------------------------------------------------------------

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ---------------------------------------------------------------------------
// Minimal slide master / layout / theme stubs
// ---------------------------------------------------------------------------

static SLIDE_MASTER_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
  xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr>
        <a:xfrm>
          <a:off x="0" y="0"/>
          <a:ext cx="9144000" cy="6858000"/>
          <a:chOff x="0" y="0"/>
          <a:chExt cx="9144000" cy="6858000"/>
        </a:xfrm>
      </p:grpSpPr>
    </p:spTree>
  </p:cSld>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr>
        <a:defRPr lang="en-US" sz="3600" b="1"/>
      </a:lvl1pPr>
    </p:titleStyle>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:defRPr lang="en-US" sz="2000"/>
      </a:lvl1pPr>
    </p:bodyStyle>
    <p:otherStyle>
      <a:lvl1pPr>
        <a:defRPr lang="en-US" sz="1800"/>
      </a:lvl1pPr>
    </p:otherStyle>
  </p:txStyles>
  <p:sldLayoutIdLst>
    <p:sldLayoutId id="2147483649" r:id="rId1"/>
  </p:sldLayoutIdLst>
</p:sldMaster>"#;

static SLIDE_MASTER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#;

static SLIDE_LAYOUT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
  xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
  type="blank">
  <p:cSld name="Blank">
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr>
        <a:xfrm>
          <a:off x="0" y="0"/>
          <a:ext cx="9144000" cy="6858000"/>
          <a:chOff x="0" y="0"/>
          <a:chExt cx="9144000" cy="6858000"/>
        </a:xfrm>
      </p:grpSpPr>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#;

static SLIDE_LAYOUT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#;

static THEME_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr lastClr="000000" val="windowText"/></a:dk1>
      <a:lt1><a:sysClr lastClr="ffffff" val="window"/></a:lt1>
      <a:dk2><a:srgbClr val="1F3864"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A9D18E"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Office">
      <a:fillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle><a:effectLst/></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;

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
    use tempfile::TempDir;

    use super::*;
    use crate::office::types::{Slide, SlideDeckSpec, WorkbookProperties};

    fn title_only_spec() -> SlideDeckSpec {
        SlideDeckSpec {
            slides: vec![Slide::Title {
                title: "Q1 2026 Results".into(),
                subtitle: Some("Investor Presentation".into()),
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

    #[test]
    fn write_minimal_deck_one_title_slide() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("minimal.pptx");
        let result = write_slide_deck(&title_only_spec(), &path).unwrap();

        assert!(result.bytes_written > 0);
        assert_eq!(
            result.bytes_written,
            std::fs::metadata(&path).unwrap().len()
        );
        assert_sha256_format(&result.sha256);
        assert_eq!(result.slide_count, 1);
    }

    #[test]
    fn write_deck_rejects_empty_slides() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.pptx");
        let spec = SlideDeckSpec {
            slides: vec![],
            properties: WorkbookProperties::default(),
        };

        let err = write_slide_deck(&spec, &path).unwrap_err();
        assert!(matches!(
            err,
            crate::error::CorpFinanceError::InvalidInput { .. }
        ));
    }

    #[test]
    fn write_deck_with_all_four_slide_kinds() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("all_kinds.pptx");
        let spec = SlideDeckSpec {
            slides: vec![
                Slide::Title {
                    title: "Deck Title".into(),
                    subtitle: None,
                },
                Slide::Section {
                    heading: "Part One".into(),
                },
                Slide::Content {
                    title: "Key Points".into(),
                    bullets: vec!["Point A".into(), "Point B".into()],
                },
                Slide::Table {
                    title: "Comps".into(),
                    headers: vec!["Company".into(), "EV/EBITDA".into()],
                    rows: vec![vec!["ACME".into(), "8.5x".into()]],
                },
            ],
            properties: WorkbookProperties::default(),
        };

        let result = write_slide_deck(&spec, &path).unwrap();
        assert_eq!(result.slide_count, 4);
        assert!(result.bytes_written > 0);
        assert_sha256_format(&result.sha256);
    }

    #[test]
    fn write_deck_with_content_slide_bullets() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("content.pptx");
        let spec = SlideDeckSpec {
            slides: vec![Slide::Content {
                title: "Investment Highlights".into(),
                bullets: vec![
                    "15% revenue CAGR over 3 years".into(),
                    "EBITDA margin expanding to 28%".into(),
                    "De-leveraging to 2.5x net debt / EBITDA".into(),
                ],
            }],
            properties: WorkbookProperties::default(),
        };

        let result = write_slide_deck(&spec, &path).unwrap();
        assert!(result.bytes_written > 0);
        // Bullet text must appear in the zip
        let bytes = std::fs::read(&path).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("15% revenue CAGR") || {
                // Text may be deflate-compressed; unzip to check
                let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
                let mut found = false;
                for i in 0..archive.len() {
                    let mut entry = archive.by_index(i).unwrap();
                    let mut buf = Vec::new();
                    use std::io::Read;
                    entry.read_to_end(&mut buf).unwrap();
                    if String::from_utf8_lossy(&buf).contains("15% revenue CAGR") {
                        found = true;
                        break;
                    }
                }
                found
            }
        );
    }

    #[test]
    fn write_deck_with_table_slide() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("table.pptx");
        let spec = SlideDeckSpec {
            slides: vec![Slide::Table {
                title: "Trading Comparables".into(),
                headers: vec!["Company".into(), "EV ($M)".into(), "EV/EBITDA".into()],
                rows: vec![
                    vec!["ACME Corp".into(), "1,200".into(), "8.5x".into()],
                    vec!["Beta Inc".into(), "850".into(), "7.2x".into()],
                ],
            }],
            properties: WorkbookProperties::default(),
        };

        let result = write_slide_deck(&spec, &path).unwrap();
        assert!(result.bytes_written > 0);
        assert_sha256_format(&result.sha256);
    }

    #[test]
    fn write_deck_with_section_dividers() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sections.pptx");
        let spec = SlideDeckSpec {
            slides: vec![
                Slide::Title {
                    title: "Annual Review".into(),
                    subtitle: None,
                },
                Slide::Section {
                    heading: "Financial Performance".into(),
                },
                Slide::Content {
                    title: "Revenue".into(),
                    bullets: vec!["$1.2B total".into()],
                },
                Slide::Section {
                    heading: "Strategic Outlook".into(),
                },
            ],
            properties: WorkbookProperties::default(),
        };

        let result = write_slide_deck(&spec, &path).unwrap();
        assert_eq!(result.slide_count, 4);
        assert!(result.bytes_written > 0);
    }

    #[test]
    fn write_deck_sha256_stability() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();
        let path1 = dir1.path().join("deck1.pptx");
        let path2 = dir2.path().join("deck2.pptx");
        let spec = title_only_spec();

        let r1 = write_slide_deck(&spec, &path1).unwrap();
        let r2 = write_slide_deck(&spec, &path2).unwrap();

        assert_eq!(
            r1.sha256, r2.sha256,
            "identical spec must produce identical sha256"
        );
    }

    #[test]
    fn write_deck_zip_magic_bytes() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("magic.pptx");
        write_slide_deck(&title_only_spec(), &path).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            &bytes[..4],
            b"PK\x03\x04",
            "pptx must begin with ZIP magic bytes PK\\x03\\x04"
        );
    }

    #[test]
    fn write_deck_rejects_empty_table_columns() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("badtable.pptx");
        let spec = SlideDeckSpec {
            slides: vec![Slide::Table {
                title: "Bad Table".into(),
                headers: vec![], // empty — should be rejected
                rows: vec![],
            }],
            properties: WorkbookProperties::default(),
        };

        let err = write_slide_deck(&spec, &path).unwrap_err();
        assert!(matches!(
            err,
            crate::error::CorpFinanceError::InvalidInput { .. }
        ));
    }
}
