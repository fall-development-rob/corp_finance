//! Equity Research workflow definitions.
//!
//! Nine static workflow templates covering the full equity research lifecycle:
//! initiating coverage, earnings analysis/preview, model updates, morning notes,
//! thesis tracking, catalyst calendars, idea generation, and sector overviews.

use super::types::*;

// ---------------------------------------------------------------------------
// Public registry
// ---------------------------------------------------------------------------

pub static WORKFLOWS: &[&WorkflowDefinition] = &[
    &INITIATING_COVERAGE,
    &EARNINGS_ANALYSIS,
    &EARNINGS_PREVIEW,
    &MODEL_UPDATE,
    &MORNING_NOTE,
    &THESIS_TRACKER,
    &CATALYST_CALENDAR,
    &IDEA_GENERATION,
    &SECTOR_OVERVIEW,
];

// ---------------------------------------------------------------------------
// 1. Initiating Coverage
// ---------------------------------------------------------------------------

static INITIATING_COVERAGE: WorkflowDefinition = WorkflowDefinition {
    id: "er-initiating-coverage",
    name: "Initiating Coverage Report",
    domain: WorkflowDomain::EquityResearch,
    description: "Full initiating coverage report with business overview, \
                  financial analysis, valuation, and investment thesis",
    required_inputs: &[
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: true,
            description: "Target company ticker symbol",
        },
        WorkflowInput {
            name: "peer_group",
            input_type: InputType::PeerGroup,
            required: false,
            description: "Peer group for comparable analysis",
        },
        WorkflowInput {
            name: "target_return",
            input_type: InputType::TargetReturn,
            required: false,
            description: "Target return threshold for recommendation",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Data Collection",
            description: "Gather financial statements, quotes, and key metrics",
            required_tools: &[
                "fmp_income_statement",
                "fmp_balance_sheet",
                "fmp_cash_flow",
                "fmp_quote",
                "fmp_key_metrics",
            ],
        },
        WorkflowStep {
            order: 2,
            name: "Earnings Quality",
            description: "Screen for manipulation and fundamental strength",
            required_tools: &[
                "beneish_mscore",
                "piotroski_fscore",
                "earnings_quality_composite",
            ],
        },
        WorkflowStep {
            order: 3,
            name: "Valuation",
            description: "DCF, comps, and target price derivation",
            required_tools: &[
                "wacc_calculator",
                "dcf_model",
                "comps_analysis",
                "target_price",
            ],
        },
        WorkflowStep {
            order: 4,
            name: "Scenario Analysis",
            description: "Base/bull/bear cases with sensitivity",
            required_tools: &["sensitivity_matrix", "monte_carlo_dcf"],
        },
        WorkflowStep {
            order: 5,
            name: "Report Assembly",
            description: "Compile investment thesis with risk assessment",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "CompletenessCheck",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
        QualityGate {
            name: "SourceVerification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
        QualityGate {
            name: "ScenarioCheck",
            check_type: QualityCheckType::ScenarioCheck,
            required: true,
        },
        QualityGate {
            name: "RiskFirstCheck",
            check_type: QualityCheckType::RiskFirstCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Investment Thesis",
        "Business Overview",
        "Industry Analysis",
        "Financial Analysis",
        "Valuation",
        "Scenario Analysis",
        "Risk Factors",
        "Target Price",
    ],
};

// ---------------------------------------------------------------------------
// 2. Earnings Analysis
// ---------------------------------------------------------------------------

static EARNINGS_ANALYSIS: WorkflowDefinition = WorkflowDefinition {
    id: "er-earnings-analysis",
    name: "Earnings Analysis",
    domain: WorkflowDomain::EquityResearch,
    description: "Post-earnings analysis with beat/miss assessment, \
                  guidance review, and estimate revisions",
    required_inputs: &[
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: true,
            description: "Company ticker symbol",
        },
        WorkflowInput {
            name: "quarter",
            input_type: InputType::FreeText,
            required: true,
            description: "Reporting quarter (e.g. Q3 2025)",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Earnings Data",
            description: "Pull reported vs consensus",
            required_tools: &["fmp_earnings", "fmp_analyst_estimates", "fmp_quote"],
        },
        WorkflowStep {
            order: 2,
            name: "Quality Screen",
            description: "Check earnings quality signals",
            required_tools: &["beneish_mscore", "piotroski_fscore"],
        },
        WorkflowStep {
            order: 3,
            name: "Estimate Impact",
            description: "Revise model and target",
            required_tools: &["three_statement_model", "target_price"],
        },
        WorkflowStep {
            order: 4,
            name: "Report",
            description: "Summarise beat/miss and outlook",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "CompletenessCheck",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
        QualityGate {
            name: "SourceVerification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
    ],
    output_sections: &[
        "Summary",
        "Beat/Miss Analysis",
        "Guidance Review",
        "Estimate Revisions",
        "Valuation Impact",
        "Recommendation",
    ],
};

// ---------------------------------------------------------------------------
// 3. Earnings Preview
// ---------------------------------------------------------------------------

static EARNINGS_PREVIEW: WorkflowDefinition = WorkflowDefinition {
    id: "er-earnings-preview",
    name: "Earnings Preview",
    domain: WorkflowDomain::EquityResearch,
    description: "Pre-earnings preview with consensus expectations, \
                  key metrics to watch, and scenario outcomes",
    required_inputs: &[WorkflowInput {
        name: "ticker",
        input_type: InputType::Ticker,
        required: true,
        description: "Company ticker symbol",
    }],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Consensus",
            description: "Gather analyst estimates and historical surprises",
            required_tools: &["fmp_analyst_estimates", "fmp_earnings"],
        },
        WorkflowStep {
            order: 2,
            name: "Key Metrics",
            description: "Identify critical KPIs and thresholds",
            required_tools: &["fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 3,
            name: "Scenarios",
            description: "Outline beat/miss/inline scenarios",
            required_tools: &["sensitivity_matrix"],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "CompletenessCheck",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
        QualityGate {
            name: "ScenarioCheck",
            check_type: QualityCheckType::ScenarioCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Consensus Expectations",
        "Key Metrics to Watch",
        "Scenario Outcomes",
        "Trading Strategy",
    ],
};

// ---------------------------------------------------------------------------
// 4. Model Update
// ---------------------------------------------------------------------------

static MODEL_UPDATE: WorkflowDefinition = WorkflowDefinition {
    id: "er-model-update",
    name: "Model Update",
    domain: WorkflowDomain::EquityResearch,
    description: "Refresh three-statement model with latest data \
                  and recalculate fair value",
    required_inputs: &[WorkflowInput {
        name: "ticker",
        input_type: InputType::Ticker,
        required: true,
        description: "Company ticker symbol",
    }],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Data Refresh",
            description: "Pull latest financial data",
            required_tools: &["fmp_income_statement", "fmp_balance_sheet", "fmp_cash_flow"],
        },
        WorkflowStep {
            order: 2,
            name: "Model Rebuild",
            description: "Update three-statement model",
            required_tools: &["three_statement_model"],
        },
        WorkflowStep {
            order: 3,
            name: "Revalue",
            description: "Recalculate DCF and target price",
            required_tools: &["wacc_calculator", "dcf_model", "target_price"],
        },
    ],
    quality_gates: &[QualityGate {
        name: "SourceVerification",
        check_type: QualityCheckType::SourceVerification,
        required: true,
    }],
    output_sections: &[
        "Model Changes",
        "Updated Financials",
        "Revised Valuation",
        "Target Price Change",
    ],
};

// ---------------------------------------------------------------------------
// 5. Morning Note
// ---------------------------------------------------------------------------

static MORNING_NOTE: WorkflowDefinition = WorkflowDefinition {
    id: "er-morning-note",
    name: "Morning Note",
    domain: WorkflowDomain::EquityResearch,
    description: "Daily morning briefing with market moves, \
                  earnings calendar, and key events",
    required_inputs: &[WorkflowInput {
        name: "tickers",
        input_type: InputType::FreeText,
        required: true,
        description: "Comma-separated tickers",
    }],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Market Data",
            description: "Pull quotes and recent prices",
            required_tools: &["fmp_quote", "fmp_historical_prices"],
        },
        WorkflowStep {
            order: 2,
            name: "Compile",
            description: "Assemble briefing note",
            required_tools: &[],
        },
    ],
    quality_gates: &[QualityGate {
        name: "CompletenessCheck",
        check_type: QualityCheckType::CompletenessCheck,
        required: true,
    }],
    output_sections: &[
        "Market Overview",
        "Coverage Universe Update",
        "Earnings Calendar",
        "Key Events",
    ],
};

// ---------------------------------------------------------------------------
// 6. Thesis Tracker
// ---------------------------------------------------------------------------

static THESIS_TRACKER: WorkflowDefinition = WorkflowDefinition {
    id: "er-thesis-tracker",
    name: "Thesis Tracker",
    domain: WorkflowDomain::EquityResearch,
    description: "Track investment thesis milestones, catalysts, \
                  and conviction level changes",
    required_inputs: &[
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: true,
            description: "Company ticker symbol",
        },
        WorkflowInput {
            name: "thesis",
            input_type: InputType::FreeText,
            required: true,
            description: "Original investment thesis statement",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Current State",
            description: "Pull current data and compare to thesis",
            required_tools: &["fmp_quote", "fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Milestone Check",
            description: "Assess progress against thesis milestones",
            required_tools: &["fmp_earnings"],
        },
        WorkflowStep {
            order: 3,
            name: "Conviction Update",
            description: "Revise conviction and catalysts",
            required_tools: &[],
        },
    ],
    quality_gates: &[QualityGate {
        name: "SourceVerification",
        check_type: QualityCheckType::SourceVerification,
        required: true,
    }],
    output_sections: &[
        "Thesis Recap",
        "Milestone Progress",
        "Catalyst Update",
        "Conviction Assessment",
        "Action Items",
    ],
};

// ---------------------------------------------------------------------------
// 7. Catalyst Calendar
// ---------------------------------------------------------------------------

static CATALYST_CALENDAR: WorkflowDefinition = WorkflowDefinition {
    id: "er-catalyst-calendar",
    name: "Catalyst Calendar",
    domain: WorkflowDomain::EquityResearch,
    description: "Calendar of upcoming catalysts with expected impact \
                  and probability",
    required_inputs: &[WorkflowInput {
        name: "tickers",
        input_type: InputType::FreeText,
        required: true,
        description: "Comma-separated tickers",
    }],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Data Gather",
            description: "Pull earnings dates and estimates",
            required_tools: &["fmp_earnings", "fmp_analyst_estimates"],
        },
        WorkflowStep {
            order: 2,
            name: "Calendar Build",
            description: "Compile catalyst timeline",
            required_tools: &[],
        },
    ],
    quality_gates: &[QualityGate {
        name: "CompletenessCheck",
        check_type: QualityCheckType::CompletenessCheck,
        required: true,
    }],
    output_sections: &[
        "Catalyst Calendar",
        "Impact Assessment",
        "Probability Weighting",
    ],
};

// ---------------------------------------------------------------------------
// 8. Idea Generation
// ---------------------------------------------------------------------------

static IDEA_GENERATION: WorkflowDefinition = WorkflowDefinition {
    id: "er-idea-generation",
    name: "Idea Generation",
    domain: WorkflowDomain::EquityResearch,
    description: "Systematic screen for investment ideas using \
                  fundamental and quality filters",
    required_inputs: &[WorkflowInput {
        name: "criteria",
        input_type: InputType::FreeText,
        required: true,
        description: "Screening criteria description",
    }],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Screen",
            description: "Apply fundamental screens",
            required_tools: &["fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Quality Filter",
            description: "Apply earnings quality and forensic filters",
            required_tools: &["piotroski_fscore", "beneish_mscore"],
        },
        WorkflowStep {
            order: 3,
            name: "Rank",
            description: "Rank and prioritise candidates",
            required_tools: &["comps_analysis"],
        },
    ],
    quality_gates: &[QualityGate {
        name: "SourceVerification",
        check_type: QualityCheckType::SourceVerification,
        required: true,
    }],
    output_sections: &[
        "Screening Criteria",
        "Candidates",
        "Quality Assessment",
        "Top Ideas",
        "Next Steps",
    ],
};

// ---------------------------------------------------------------------------
// 9. Sector Overview
// ---------------------------------------------------------------------------

static SECTOR_OVERVIEW: WorkflowDefinition = WorkflowDefinition {
    id: "er-sector-overview",
    name: "Sector Overview",
    domain: WorkflowDomain::EquityResearch,
    description: "Sector-level analysis with competitive landscape, \
                  valuation comparison, and sector thesis",
    required_inputs: &[
        WorkflowInput {
            name: "sector",
            input_type: InputType::FreeText,
            required: true,
            description: "Sector name or classification",
        },
        WorkflowInput {
            name: "tickers",
            input_type: InputType::FreeText,
            required: true,
            description: "Comma-separated tickers in sector",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Sector Data",
            description: "Pull comparable data across sector",
            required_tools: &["fmp_quote", "fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Comps Table",
            description: "Build sector valuation comparison",
            required_tools: &["comps_analysis"],
        },
        WorkflowStep {
            order: 3,
            name: "Sector View",
            description: "Formulate sector thesis",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "CompletenessCheck",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
        QualityGate {
            name: "SourceVerification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
    ],
    output_sections: &[
        "Sector Overview",
        "Competitive Landscape",
        "Valuation Comparison",
        "Sector Thesis",
        "Top Picks",
    ],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_count() {
        assert_eq!(WORKFLOWS.len(), 9);
    }

    #[test]
    fn test_all_have_steps() {
        for w in WORKFLOWS {
            assert!(!w.steps.is_empty(), "{} has no steps", w.id);
        }
    }

    #[test]
    fn test_all_have_quality_gates() {
        for w in WORKFLOWS {
            assert!(!w.quality_gates.is_empty(), "{} has no quality gates", w.id);
        }
    }

    #[test]
    fn test_ids_unique() {
        let ids: Vec<&str> = WORKFLOWS.iter().map(|w| w.id).collect();
        let unique: std::collections::HashSet<&str> = ids.iter().cloned().collect();
        assert_eq!(ids.len(), unique.len(), "Duplicate workflow IDs found");
    }

    #[test]
    fn test_domain_consistent() {
        for w in WORKFLOWS {
            assert_eq!(
                w.domain,
                WorkflowDomain::EquityResearch,
                "{} has wrong domain",
                w.id
            );
        }
    }

    #[test]
    fn test_step_ordering() {
        for w in WORKFLOWS {
            for (i, step) in w.steps.iter().enumerate() {
                assert_eq!(
                    step.order,
                    (i + 1) as u32,
                    "{} step {} has wrong order",
                    w.id,
                    step.name
                );
            }
        }
    }
}
