//! Financial red flag composite scoring.
//!
//! Combines multiple financial signals into a composite 0-100 risk score
//! across four categories: earnings quality, growth quality, financial
//! health, and governance. All arithmetic uses `rust_decimal::Decimal`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagScoringInput {
    pub beneish_m_score: Option<Decimal>,
    pub altman_z_score: Option<Decimal>,
    pub piotroski_f_score: Option<u8>,
    pub cfo_to_net_income: Decimal,
    pub revenue_growth: Decimal,
    pub receivables_growth: Decimal,
    pub inventory_growth: Decimal,
    pub sga_to_revenue_change: Decimal,
    pub debt_to_equity: Decimal,
    pub interest_coverage: Decimal,
    pub audit_opinion: String,
    pub auditor_change: bool,
    pub related_party_transactions: bool,
    pub restatement_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlag {
    pub category: String,
    pub flag: String,
    pub severity: String,
    pub points: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryScores {
    pub earnings_quality: Decimal,
    pub growth_quality: Decimal,
    pub financial_health: Decimal,
    pub governance: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagScoringOutput {
    pub composite_score: Decimal,
    pub risk_level: String,
    pub flags: Vec<RedFlag>,
    pub category_scores: CategoryScores,
    pub total_flags: u32,
    pub critical_flags: u32,
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn add_flag(flags: &mut Vec<RedFlag>, cat: &str, msg: &str, sev: &str, pts: Decimal) -> Decimal {
    flags.push(RedFlag {
        category: cat.into(),
        flag: msg.into(),
        severity: sev.into(),
        points: pts,
    });
    pts
}

fn cap(val: Decimal, max: Decimal) -> Decimal {
    if val > max {
        max
    } else {
        val
    }
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate financial red flag composite score.
pub fn calculate_red_flag_scoring(
    input: &RedFlagScoringInput,
) -> CorpFinanceResult<RedFlagScoringOutput> {
    let valid_opinions = ["clean", "qualified", "adverse", "disclaimer"];
    if !valid_opinions.contains(&input.audit_opinion.as_str()) {
        return Err(CorpFinanceError::InvalidInput {
            field: "audit_opinion".into(),
            reason: "Must be 'clean', 'qualified', 'adverse', or 'disclaimer'.".into(),
        });
    }

    let mut flags: Vec<RedFlag> = Vec::new();
    let (mut eq, mut gq, mut fh, mut gov) =
        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);

    // --- Earnings Quality (max 25) ---
    if let Some(m) = input.beneish_m_score {
        if m > dec!(-1.78) {
            eq += add_flag(
                &mut flags,
                "Earnings Quality",
                &format!("Beneish M-Score {m} exceeds -1.78 threshold (manipulation likely)"),
                "high",
                dec!(15),
            );
        }
    }
    if input.cfo_to_net_income < Decimal::ZERO {
        eq += add_flag(
            &mut flags,
            "Earnings Quality",
            "Negative CFO-to-net-income ratio indicates poor cash conversion",
            "high",
            dec!(10),
        );
    } else if input.cfo_to_net_income < dec!(0.5) {
        eq += add_flag(
            &mut flags,
            "Earnings Quality",
            "Low CFO-to-net-income ratio suggests accrual-heavy earnings",
            "medium",
            dec!(5),
        );
    }
    if let Some(f) = input.piotroski_f_score {
        if f < 3 {
            eq += add_flag(
                &mut flags,
                "Earnings Quality",
                &format!("Piotroski F-Score {f} indicates weak financial position"),
                "medium",
                dec!(5),
            );
        }
    }
    eq = cap(eq, dec!(25));

    // --- Growth Quality (max 25) ---
    if input.receivables_growth > input.revenue_growth {
        gq += add_flag(
            &mut flags,
            "Growth Quality",
            "Receivables growing faster than revenue suggests aggressive recognition",
            "medium",
            dec!(8),
        );
    }
    if input.inventory_growth > input.revenue_growth {
        gq += add_flag(
            &mut flags,
            "Growth Quality",
            "Inventory growing faster than revenue suggests demand weakness",
            "medium",
            dec!(7),
        );
    }
    if input.revenue_growth > dec!(0.50) {
        gq += add_flag(
            &mut flags,
            "Growth Quality",
            "Revenue growth exceeds 50%, aggressive growth flag",
            "low",
            dec!(5),
        );
    }
    if input.sga_to_revenue_change > Decimal::ZERO {
        gq += add_flag(
            &mut flags,
            "Growth Quality",
            "SGA-to-revenue ratio increasing suggests declining efficiency",
            "low",
            dec!(5),
        );
    }
    gq = cap(gq, dec!(25));

    // --- Financial Health (max 25) ---
    if let Some(z) = input.altman_z_score {
        if z < dec!(1.81) {
            fh += add_flag(
                &mut flags,
                "Financial Health",
                &format!("Altman Z-Score {z} in distress zone (below 1.81)"),
                "high",
                dec!(15),
            );
        }
    }
    if input.debt_to_equity > dec!(5) {
        fh += add_flag(
            &mut flags,
            "Financial Health",
            "Debt-to-equity exceeds 5x, extremely leveraged",
            "high",
            dec!(10),
        );
    } else if input.debt_to_equity > dec!(3) {
        fh += add_flag(
            &mut flags,
            "Financial Health",
            "Debt-to-equity exceeds 3x, highly leveraged",
            "medium",
            dec!(5),
        );
    }
    if input.interest_coverage < dec!(1.5) {
        fh += add_flag(
            &mut flags,
            "Financial Health",
            "Interest coverage below 1.5x, debt service at risk",
            "medium",
            dec!(5),
        );
    }
    fh = cap(fh, dec!(25));

    // --- Governance (max 25) ---
    match input.audit_opinion.as_str() {
        "adverse" | "disclaimer" => {
            let label = if input.audit_opinion == "adverse" {
                "Adverse audit opinion"
            } else {
                "Disclaimer of opinion from auditor"
            };
            gov += add_flag(&mut flags, "Governance", label, "high", dec!(15));
        }
        "qualified" => {
            gov += add_flag(
                &mut flags,
                "Governance",
                "Qualified audit opinion",
                "high",
                dec!(10),
            );
        }
        _ => {}
    }
    if input.auditor_change {
        gov += add_flag(
            &mut flags,
            "Governance",
            "Recent auditor change",
            "medium",
            dec!(5),
        );
    }
    if input.related_party_transactions {
        gov += add_flag(
            &mut flags,
            "Governance",
            "Material related-party transactions present",
            "medium",
            dec!(5),
        );
    }
    if input.restatement_history {
        gov += add_flag(
            &mut flags,
            "Governance",
            "History of financial restatements",
            "high",
            dec!(10),
        );
    }
    gov = cap(gov, dec!(25));

    // --- Composite ---
    let composite_score = cap(eq + gq + fh + gov, dec!(100));
    let risk_level = match composite_score {
        s if s <= dec!(15) => "Clean",
        s if s <= dec!(30) => "Low Risk",
        s if s <= dec!(50) => "Moderate Risk",
        s if s <= dec!(75) => "High Risk",
        _ => "Critical",
    }
    .to_string();

    let total_flags = flags.len() as u32;
    let critical_flags = flags.iter().filter(|f| f.severity == "high").count() as u32;

    Ok(RedFlagScoringOutput {
        composite_score,
        risk_level,
        flags,
        category_scores: CategoryScores {
            earnings_quality: eq,
            growth_quality: gq,
            financial_health: fh,
            governance: gov,
        },
        total_flags,
        critical_flags,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn clean_input() -> RedFlagScoringInput {
        RedFlagScoringInput {
            beneish_m_score: Some(dec!(-2.5)),
            altman_z_score: Some(dec!(3.5)),
            piotroski_f_score: Some(7),
            cfo_to_net_income: dec!(1.2),
            revenue_growth: dec!(0.10),
            receivables_growth: dec!(0.08),
            inventory_growth: dec!(0.05),
            sga_to_revenue_change: dec!(-0.01),
            debt_to_equity: dec!(1.5),
            interest_coverage: dec!(5.0),
            audit_opinion: "clean".into(),
            auditor_change: false,
            related_party_transactions: false,
            restatement_history: false,
        }
    }

    fn distressed_input() -> RedFlagScoringInput {
        RedFlagScoringInput {
            beneish_m_score: Some(dec!(-1.0)),
            altman_z_score: Some(dec!(1.2)),
            piotroski_f_score: Some(2),
            cfo_to_net_income: dec!(-0.5),
            revenue_growth: dec!(0.60),
            receivables_growth: dec!(0.80),
            inventory_growth: dec!(0.70),
            sga_to_revenue_change: dec!(0.05),
            debt_to_equity: dec!(6.0),
            interest_coverage: dec!(0.8),
            audit_opinion: "adverse".into(),
            auditor_change: true,
            related_party_transactions: true,
            restatement_history: true,
        }
    }

    #[test]
    fn test_clean_company_score() {
        let out = calculate_red_flag_scoring(&clean_input()).unwrap();
        assert_eq!(out.composite_score, Decimal::ZERO);
        assert_eq!(out.risk_level, "Clean");
    }

    #[test]
    fn test_clean_company_no_flags() {
        let out = calculate_red_flag_scoring(&clean_input()).unwrap();
        assert_eq!(out.total_flags, 0);
        assert_eq!(out.critical_flags, 0);
    }

    #[test]
    fn test_distressed_high_score() {
        let out = calculate_red_flag_scoring(&distressed_input()).unwrap();
        assert!(
            out.composite_score > dec!(50),
            "score={}",
            out.composite_score
        );
    }

    #[test]
    fn test_distressed_critical_or_high() {
        let out = calculate_red_flag_scoring(&distressed_input()).unwrap();
        assert!(out.risk_level == "Critical" || out.risk_level == "High Risk");
    }

    #[test]
    fn test_distressed_many_flags() {
        let out = calculate_red_flag_scoring(&distressed_input()).unwrap();
        assert!(out.total_flags >= 8);
    }

    #[test]
    fn test_distressed_critical_flags() {
        assert!(
            calculate_red_flag_scoring(&distressed_input())
                .unwrap()
                .critical_flags
                >= 3
        );
    }

    #[test]
    fn test_beneish_flag() {
        let mut i = clean_input();
        i.beneish_m_score = Some(dec!(-1.5));
        let out = calculate_red_flag_scoring(&i).unwrap();
        assert!(out.category_scores.earnings_quality >= dec!(15));
        assert!(out.flags.iter().any(|f| f.flag.contains("Beneish")));
    }

    #[test]
    fn test_no_beneish_when_none() {
        let mut i = clean_input();
        i.beneish_m_score = None;
        assert!(!calculate_red_flag_scoring(&i)
            .unwrap()
            .flags
            .iter()
            .any(|f| f.flag.contains("Beneish")));
    }

    #[test]
    fn test_low_cfo() {
        let mut i = clean_input();
        i.cfo_to_net_income = dec!(0.3);
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .earnings_quality
                >= dec!(5)
        );
    }

    #[test]
    fn test_negative_cfo() {
        let mut i = clean_input();
        i.cfo_to_net_income = dec!(-0.5);
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .earnings_quality
                >= dec!(10)
        );
    }

    #[test]
    fn test_piotroski() {
        let mut i = clean_input();
        i.piotroski_f_score = Some(1);
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .earnings_quality
                >= dec!(5)
        );
    }

    #[test]
    fn test_receivables_growth() {
        let mut i = clean_input();
        i.receivables_growth = dec!(0.20);
        i.revenue_growth = dec!(0.05);
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .growth_quality
                >= dec!(8)
        );
    }

    #[test]
    fn test_inventory_growth() {
        let mut i = clean_input();
        i.inventory_growth = dec!(0.20);
        i.revenue_growth = dec!(0.05);
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .growth_quality
                >= dec!(7)
        );
    }

    #[test]
    fn test_aggressive_revenue() {
        let mut i = clean_input();
        i.revenue_growth = dec!(0.55);
        assert!(calculate_red_flag_scoring(&i)
            .unwrap()
            .flags
            .iter()
            .any(|f| f.flag.contains("50%")));
    }

    #[test]
    fn test_altman_distress() {
        let mut i = clean_input();
        i.altman_z_score = Some(dec!(1.5));
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .financial_health
                >= dec!(15)
        );
    }

    #[test]
    fn test_no_altman_when_none() {
        let mut i = clean_input();
        i.altman_z_score = None;
        assert!(!calculate_red_flag_scoring(&i)
            .unwrap()
            .flags
            .iter()
            .any(|f| f.flag.contains("Altman")));
    }

    #[test]
    fn test_high_leverage() {
        let mut i = clean_input();
        i.debt_to_equity = dec!(6.0);
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .financial_health
                >= dec!(10)
        );
    }

    #[test]
    fn test_moderate_leverage() {
        let mut i = clean_input();
        i.debt_to_equity = dec!(4.0);
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .financial_health
                >= dec!(5)
        );
    }

    #[test]
    fn test_qualified_audit() {
        let mut i = clean_input();
        i.audit_opinion = "qualified".into();
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .governance
                >= dec!(10)
        );
    }

    #[test]
    fn test_adverse_audit() {
        let mut i = clean_input();
        i.audit_opinion = "adverse".into();
        assert!(
            calculate_red_flag_scoring(&i)
                .unwrap()
                .category_scores
                .governance
                >= dec!(15)
        );
    }

    #[test]
    fn test_governance_only() {
        let mut i = clean_input();
        i.auditor_change = true;
        i.related_party_transactions = true;
        i.restatement_history = true;
        let out = calculate_red_flag_scoring(&i).unwrap();
        assert!(out.category_scores.governance >= dec!(20));
        assert_eq!(out.category_scores.earnings_quality, Decimal::ZERO);
    }

    #[test]
    fn test_score_capped_100() {
        assert!(
            calculate_red_flag_scoring(&distressed_input())
                .unwrap()
                .composite_score
                <= dec!(100)
        );
    }

    #[test]
    fn test_categories_capped_25() {
        let out = calculate_red_flag_scoring(&distressed_input()).unwrap();
        assert!(out.category_scores.earnings_quality <= dec!(25));
        assert!(out.category_scores.growth_quality <= dec!(25));
        assert!(out.category_scores.financial_health <= dec!(25));
        assert!(out.category_scores.governance <= dec!(25));
    }

    #[test]
    fn test_invalid_audit() {
        let mut i = clean_input();
        i.audit_opinion = "unknown".into();
        assert!(calculate_red_flag_scoring(&i).is_err());
    }

    #[test]
    fn test_risk_clean() {
        assert_eq!(
            calculate_red_flag_scoring(&clean_input())
                .unwrap()
                .risk_level,
            "Clean"
        );
    }

    #[test]
    fn test_risk_low() {
        let mut i = clean_input();
        i.sga_to_revenue_change = dec!(0.02);
        i.revenue_growth = dec!(0.55);
        i.receivables_growth = dec!(0.60);
        assert_eq!(
            calculate_red_flag_scoring(&i).unwrap().risk_level,
            "Low Risk"
        );
    }

    #[test]
    fn test_serde() {
        let out = calculate_red_flag_scoring(&clean_input()).unwrap();
        let _: RedFlagScoringOutput =
            serde_json::from_str(&serde_json::to_string(&out).unwrap()).unwrap();
    }

    #[test]
    fn test_all_optional_none() {
        let mut i = clean_input();
        i.beneish_m_score = None;
        i.altman_z_score = None;
        i.piotroski_f_score = None;
        assert_eq!(
            calculate_red_flag_scoring(&i).unwrap().composite_score,
            Decimal::ZERO
        );
    }
}
