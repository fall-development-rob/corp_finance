//! Pitch Deck generator — turns a [`PitchDeckInput`] into an institutional
//! [`SlideDeckSpec`] in one call, following the canonical sell-side IB flow.
//!
//! Gated on the `office` feature only; no secondary compute feature required.
//!
//! # Optional-section handling
//!
//! Any Section divider whose immediately-following Content slide would be empty
//! is omitted together with that Content slide.  Specifically:
//! - Slides 3+4 (Executive Summary) — skipped when `executive_summary` is empty.
//! - Slides 5+6+7 (Market & Business) — Section is skipped when both
//!   `market_overview` AND `business_overview` are empty; individual Content
//!   slides are skipped independently if their respective Vec is empty.
//! - Slides 8+9 (Financials / Financial Highlights) — Section is skipped when
//!   `financial_highlights` has fewer than 1 row (no headers at all).
//! - Returns Summary (slide 10) — never skipped; renders empty rows if pairs empty.
//! - Slides 11+12 (Process) — Section + Timeline skipped when
//!   `process_timeline` is empty.
//! - Slide 13 (Conclusion) — skipped when `conclusion` is empty.

#[cfg(feature = "office")]
use crate::office::types::{Slide, SlideDeckSpec, WorkbookProperties};

// ---------------------------------------------------------------------------
// Input struct
// ---------------------------------------------------------------------------

/// All fields required to build a canonical sell-side IB pitch deck.
#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct PitchDeckInput {
    /// Company or deal name — rendered as the title slide heading.
    pub deal_name: String,
    /// Secondary title line, e.g. "Project Atlas — Confidential".
    pub subtitle: String,
    /// Deck date, e.g. "May 2026".
    pub date: String,
    /// Bullet labels for the Agenda slide.
    pub agenda: Vec<String>,
    /// 3-6 executive-summary bullets.
    pub executive_summary: Vec<String>,
    /// 3-6 market overview bullets.
    pub market_overview: Vec<String>,
    /// 3-6 business overview bullets.
    pub business_overview: Vec<String>,
    /// Financial highlights table; first row is the header row.
    pub financial_highlights: Vec<Vec<String>>,
    /// Returns summary rows as (metric, value) pairs.
    pub returns_summary: Vec<(String, String)>,
    /// Process timeline rows as (milestone, date) pairs.
    pub process_timeline: Vec<(String, String)>,
    /// 3-6 closing / conclusion bullets.
    pub conclusion: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public function
// ---------------------------------------------------------------------------

/// Convert a [`PitchDeckInput`] into a [`SlideDeckSpec`] ready for
/// [`crate::office::pptx::write_slide_deck`].
///
/// No allocation beyond `Vec` pushes; all string data is cloned from `input`.
#[cfg(feature = "office")]
pub fn pitch_deck_to_deck(input: &PitchDeckInput) -> SlideDeckSpec {
    let mut slides: Vec<Slide> = Vec::with_capacity(13);

    // Slide 1 — Title
    slides.push(Slide::Title {
        title: input.deal_name.clone(),
        subtitle: Some(format!("{}\n{}", input.subtitle, input.date)),
    });

    // Slide 2 — Agenda
    slides.push(Slide::Content {
        title: "Agenda".into(),
        bullets: input.agenda.clone(),
    });

    // Slides 3+4 — Executive Summary (skipped when empty)
    if !input.executive_summary.is_empty() {
        slides.push(Slide::Section {
            heading: "Executive Summary".into(),
        });
        slides.push(Slide::Content {
            title: "Executive Summary".into(),
            bullets: input.executive_summary.clone(),
        });
    }

    // Slides 5+6+7 — Market & Business
    let has_market = !input.market_overview.is_empty();
    let has_business = !input.business_overview.is_empty();
    if has_market || has_business {
        slides.push(Slide::Section {
            heading: "Market & Business".into(),
        });
        if has_market {
            slides.push(Slide::Content {
                title: "Market Overview".into(),
                bullets: input.market_overview.clone(),
            });
        }
        if has_business {
            slides.push(Slide::Content {
                title: "Business Overview".into(),
                bullets: input.business_overview.clone(),
            });
        }
    }

    // Slides 8+9 — Financials
    if !input.financial_highlights.is_empty() {
        slides.push(Slide::Section {
            heading: "Financials".into(),
        });
        slides.push(build_financial_highlights_slide(input));
    }

    // Slide 10 — Returns Summary (always rendered)
    slides.push(build_returns_summary_slide(input));

    // Slides 11+12 — Process
    if !input.process_timeline.is_empty() {
        slides.push(Slide::Section {
            heading: "Process".into(),
        });
        slides.push(build_process_timeline_slide(input));
    }

    // Slide 13 — Conclusion
    if !input.conclusion.is_empty() {
        slides.push(Slide::Content {
            title: "Conclusion".into(),
            bullets: input.conclusion.clone(),
        });
    }

    SlideDeckSpec {
        slides,
        properties: WorkbookProperties {
            title: Some(format!("Pitch Deck — {}", input.deal_name)),
            author: Some("corp-finance-core".into()),
            company: None,
            subject: Some("Sell-side IB Pitch Deck".into()),
        },
    }
}

// ---------------------------------------------------------------------------
// Slide builders (private)
// ---------------------------------------------------------------------------

#[cfg(feature = "office")]
fn build_financial_highlights_slide(input: &PitchDeckInput) -> Slide {
    if input.financial_highlights.len() >= 2 {
        Slide::Table {
            title: "Financial Highlights".into(),
            headers: input.financial_highlights[0].clone(),
            rows: input.financial_highlights[1..].to_vec(),
        }
    } else {
        // Only a header row or empty — emit safe empty-rows table.
        let headers = if input.financial_highlights.is_empty() {
            vec!["Metric".into(), "Value".into()]
        } else {
            input.financial_highlights[0].clone()
        };
        Slide::Table {
            title: "Financial Highlights".into(),
            headers,
            rows: vec![],
        }
    }
}

#[cfg(feature = "office")]
fn build_returns_summary_slide(input: &PitchDeckInput) -> Slide {
    let rows: Vec<Vec<String>> = input
        .returns_summary
        .iter()
        .map(|(metric, value)| vec![metric.clone(), value.clone()])
        .collect();
    Slide::Table {
        title: "Returns Summary".into(),
        headers: vec!["Metric".into(), "Value".into()],
        rows,
    }
}

#[cfg(feature = "office")]
fn build_process_timeline_slide(input: &PitchDeckInput) -> Slide {
    let rows: Vec<Vec<String>> = input
        .process_timeline
        .iter()
        .map(|(milestone, date)| vec![milestone.clone(), date.clone()])
        .collect();
    Slide::Table {
        title: "Process Timeline".into(),
        headers: vec!["Milestone".into(), "Date".into()],
        rows,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    fn minimal_input() -> PitchDeckInput {
        PitchDeckInput {
            deal_name: "Project Apex".into(),
            subtitle: "Project Apex — Confidential".into(),
            date: "May 2026".into(),
            agenda: vec![
                "Executive Summary".into(),
                "Market Overview".into(),
                "Financial Highlights".into(),
            ],
            executive_summary: vec![
                "Leading market position with 35% share".into(),
                "EBITDA margins expanding to 28%".into(),
                "Multiple strategic acquirers identified".into(),
            ],
            market_overview: vec!["$12B TAM growing at 8% CAGR".into()],
            business_overview: vec!["Founded 2015; 400 employees across 3 geographies".into()],
            financial_highlights: vec![
                vec!["Metric".into(), "FY24A".into(), "FY25E".into()],
                vec!["Revenue ($M)".into(), "320".into(), "358".into()],
                vec!["EBITDA ($M)".into(), "80".into(), "100".into()],
            ],
            returns_summary: vec![
                ("IRR".into(), "22%".into()),
                ("MOIC".into(), "2.8x".into()),
            ],
            process_timeline: vec![
                ("Launch".into(), "Jun 2026".into()),
                ("LOIs".into(), "Aug 2026".into()),
                ("Close".into(), "Oct 2026".into()),
            ],
            conclusion: vec![
                "Rare asset with defensible moat".into(),
                "Clear path to value creation".into(),
                "Process launching immediately".into(),
            ],
        }
    }

    #[test]
    fn pitch_deck_to_deck_basic() {
        let input = minimal_input();
        let deck = pitch_deck_to_deck(&input);

        // Fully-populated input: Title + Agenda + (Section+Content)*ES +
        // (Section+Content+Content)*MktBiz + (Section+Table)*Fin + Returns +
        // (Section+Table)*Process + Conclusion = 13 slides.
        assert_eq!(
            deck.slides.len(),
            13,
            "expected 13 slides for fully-populated input, got {}",
            deck.slides.len()
        );

        // First slide must be Title with deal_name.
        match &deck.slides[0] {
            Slide::Title { title, .. } => {
                assert_eq!(title, "Project Apex");
            }
            other => panic!("expected Title slide, got {:?}", other),
        }

        // Properties title must contain deal_name.
        assert_eq!(
            deck.properties.title.as_deref(),
            Some("Pitch Deck — Project Apex")
        );
    }

    #[test]
    fn pitch_deck_to_deck_round_trips_through_writer() {
        use crate::office::pptx::write_slide_deck;
        use tempfile::tempdir;

        let input = minimal_input();
        let deck = pitch_deck_to_deck(&input);

        let dir = tempdir().expect("tempdir creation failed");
        let path = dir.path().join("pitch_deck.pptx");

        let result = write_slide_deck(&deck, &path).expect("write_slide_deck failed");

        assert!(path.exists(), "output file does not exist");
        assert!(
            result.bytes_written > 0,
            "expected nonzero bytes, got {}",
            result.bytes_written
        );

        // Verify PK\x03\x04 ZIP magic bytes.
        let bytes = std::fs::read(&path).expect("failed to read output file");
        assert!(
            bytes.starts_with(b"PK\x03\x04"),
            "output does not start with ZIP magic bytes"
        );
    }
}
