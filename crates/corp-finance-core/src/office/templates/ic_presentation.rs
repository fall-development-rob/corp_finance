//! IC Presentation template — turns [`IcPresentationInput`] into a
//! [`SlideDeckSpec`] in one call.
//!
//! Gated on the `office` feature; no additional feature flags required.
//! The caller supplies all institutional-deliverable data; this module maps
//! it to a canonical 9-slide IC presentation flow focused on decision.

#[cfg(feature = "office")]
use crate::office::types::{Slide, SlideDeckSpec, WorkbookProperties};

/// Structured input for an Investment Committee presentation.
#[cfg(feature = "office")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct IcPresentationInput {
    pub deal_name: String,
    /// e.g. "APPROVE — $250M EQUITY CHECK"
    pub recommendation: String,
    pub date: String,
    pub presenter: String,
    /// 3-6 investment thesis bullets.
    pub investment_thesis: Vec<String>,
    /// (metric label, value) pairs for the key-metrics table.
    pub key_metrics: Vec<(String, String)>,
    /// Returns table; first row is treated as headers.
    pub returns_table: Vec<Vec<String>>,
    /// 3-6 risk bullets.
    pub risks: Vec<String>,
    /// 3-6 mitigant bullets.
    pub mitigants: Vec<String>,
    /// (milestone, date) pairs for the process timeline.
    pub timeline: Vec<(String, String)>,
}

/// Convert an [`IcPresentationInput`] into a [`SlideDeckSpec`].
///
/// Slide flow (~9 slides, variable on optional sections):
/// 1. Title
/// 2. Recommendation
/// 3. Investment Thesis
/// 4. Key Metrics (table)
/// 5. Returns Analysis (table)
/// 6. Section divider "Risks & Mitigants" (only if risks or mitigants non-empty)
/// 7. Risks (skipped if empty)
/// 8. Mitigants (skipped if empty)
/// 9. Process Timeline (skipped if empty)
#[cfg(feature = "office")]
pub fn ic_presentation_to_deck(input: &IcPresentationInput) -> SlideDeckSpec {
    let mut slides = Vec::with_capacity(9);

    // 1 — Title
    slides.push(Slide::Title {
        title: format!("IC Presentation: {}", input.deal_name),
        subtitle: Some(format!(
            "Presented by {} on {}",
            input.presenter, input.date
        )),
    });

    // 2 — Recommendation (single-bullet; the punch line)
    slides.push(Slide::Content {
        title: "Recommendation".into(),
        bullets: vec![input.recommendation.clone()],
    });

    // 3 — Investment Thesis
    slides.push(Slide::Content {
        title: "Investment Thesis".into(),
        bullets: input.investment_thesis.clone(),
    });

    // 4 — Key Metrics
    slides.push(Slide::Table {
        title: "Key Metrics".into(),
        headers: vec!["Metric".into(), "Value".into()],
        rows: input
            .key_metrics
            .iter()
            .map(|(k, v)| vec![k.clone(), v.clone()])
            .collect(),
    });

    // 5 — Returns Analysis
    slides.push(build_returns_slide(&input.returns_table));

    // 6 — Section divider (conditional)
    if !input.risks.is_empty() || !input.mitigants.is_empty() {
        slides.push(Slide::Section {
            heading: "Risks & Mitigants".into(),
        });
    }

    // 7 — Risks (conditional)
    if !input.risks.is_empty() {
        slides.push(Slide::Content {
            title: "Risks".into(),
            bullets: input.risks.clone(),
        });
    }

    // 8 — Mitigants (conditional)
    if !input.mitigants.is_empty() {
        slides.push(Slide::Content {
            title: "Mitigants".into(),
            bullets: input.mitigants.clone(),
        });
    }

    // 9 — Process Timeline (conditional)
    if !input.timeline.is_empty() {
        slides.push(Slide::Table {
            title: "Process Timeline".into(),
            headers: vec!["Milestone".into(), "Date".into()],
            rows: input
                .timeline
                .iter()
                .map(|(m, d)| vec![m.clone(), d.clone()])
                .collect(),
        });
    }

    SlideDeckSpec {
        slides,
        properties: WorkbookProperties {
            title: Some(format!("IC Presentation — {}", input.deal_name)),
            author: Some(input.presenter.clone()),
            company: None,
            subject: None,
        },
    }
}

/// Build the Returns Analysis slide, guarding against sparse input.
///
/// If `returns_table` has at least 2 rows the first row is used as headers
/// and the remainder as data rows. Fewer than 2 rows produces an empty-data
/// table with synthesised headers so the deck never panics.
#[cfg(feature = "office")]
fn build_returns_slide(returns_table: &[Vec<String>]) -> Slide {
    if returns_table.len() >= 2 {
        Slide::Table {
            title: "Returns Analysis".into(),
            headers: returns_table[0].clone(),
            rows: returns_table[1..].to_vec(),
        }
    } else {
        Slide::Table {
            title: "Returns Analysis".into(),
            headers: vec!["Scenario".into(), "MOIC".into(), "IRR".into()],
            rows: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    fn minimal_input() -> IcPresentationInput {
        IcPresentationInput {
            deal_name: "Acme Holdings".into(),
            recommendation: "APPROVE — $250M EQUITY CHECK".into(),
            date: "2026-05-08".into(),
            presenter: "Jane Smith".into(),
            investment_thesis: vec!["Market-leading platform with 30% EBITDA margins".into()],
            key_metrics: vec![("EV/EBITDA".into(), "8.5x".into())],
            returns_table: vec![
                vec!["Scenario".into(), "MOIC".into(), "IRR".into()],
                vec!["Base".into(), "2.5x".into(), "22%".into()],
            ],
            risks: vec!["Customer concentration >40% in top-3 accounts".into()],
            mitigants: vec!["Long-term MSAs with 3-year renewal cycles".into()],
            timeline: vec![("Sign LOI".into(), "2026-05-15".into())],
        }
    }

    #[test]
    fn ic_presentation_to_deck_basic() {
        let input = minimal_input();
        let deck = ic_presentation_to_deck(&input);

        // Fully populated input: 9 slides (1 title + 1 rec + 1 thesis + 1 metrics
        // + 1 returns + 1 section divider + 1 risks + 1 mitigants + 1 timeline).
        assert!(
            deck.slides.len() >= 8,
            "expected at least 8 slides, got {}",
            deck.slides.len()
        );

        // Slide 1 — Title with deal_name in title
        match &deck.slides[0] {
            Slide::Title { title, .. } => {
                assert!(
                    title.contains("Acme Holdings"),
                    "title slide must contain deal_name; got: {title}"
                );
            }
            other => panic!("slide 0 should be Title, got {other:?}"),
        }

        // Slide 2 — Content titled "Recommendation"
        match &deck.slides[1] {
            Slide::Content { title, .. } => {
                assert_eq!(title, "Recommendation", "slide 1 title mismatch");
            }
            other => panic!("slide 1 should be Content, got {other:?}"),
        }

        // Properties
        assert!(
            deck.properties
                .title
                .as_deref()
                .unwrap_or("")
                .contains("Acme Holdings"),
            "workbook title must contain deal_name"
        );
        assert_eq!(
            deck.properties.author.as_deref(),
            Some("Jane Smith"),
            "author should be presenter"
        );
    }

    #[test]
    fn ic_presentation_to_deck_round_trips_through_writer() {
        use crate::office::pptx::write_slide_deck;
        use tempfile::tempdir;

        let input = minimal_input();
        let deck = ic_presentation_to_deck(&input);

        let dir = tempdir().expect("tempdir creation failed");
        let path = dir.path().join("ic_presentation.pptx");

        let result = write_slide_deck(&deck, &path).expect("write_slide_deck failed");

        assert!(path.exists(), "output file must exist");
        assert!(
            result.bytes_written > 0,
            "expected nonzero bytes written, got {}",
            result.bytes_written
        );

        let bytes = std::fs::read(&path).expect("failed to read output file");
        assert_eq!(
            &bytes[..4],
            b"PK\x03\x04",
            "pptx must begin with ZIP magic bytes PK\\x03\\x04"
        );
    }
}
