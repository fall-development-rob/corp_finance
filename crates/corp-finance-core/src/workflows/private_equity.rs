//! Private equity workflow definitions.
//! Static compile-time definitions for PE deal and portfolio pipelines.

use super::types::*;

// ---------------------------------------------------------------------------
// Deal Screening
// ---------------------------------------------------------------------------

static DEAL_SCREENING: WorkflowDefinition = WorkflowDefinition {
    id: "pe-deal-screening",
    name: "Deal Screening",
    domain: WorkflowDomain::PrivateEquity,
    description:
        "Initial deal screening with investment criteria checklist and go/no-go recommendation",
    required_inputs: &[
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: false,
            description: "Ticker symbol if publicly traded",
        },
        WorkflowInput {
            name: "company_name",
            input_type: InputType::CompanyName,
            required: true,
            description: "Target company name",
        },
        WorkflowInput {
            name: "deal_size",
            input_type: InputType::Numeric,
            required: false,
            description: "Indicative deal size",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Financial Screen",
            description: "Pull financial data for initial screening",
            required_tools: &[
                "fmp_income_statement",
                "fmp_balance_sheet",
                "fmp_key_metrics",
                "fmp_ratios",
            ],
        },
        WorkflowStep {
            order: 2,
            name: "Credit Check",
            description: "Run credit assessment and Altman Z-Score",
            required_tools: &["altman_zscore", "credit_metrics"],
        },
        WorkflowStep {
            order: 3,
            name: "Screening Verdict",
            description: "Compile go/no-go recommendation",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Source Verification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Company Overview",
        "Investment Criteria Check",
        "Financial Summary",
        "Credit Assessment",
        "Go/No-Go Recommendation",
    ],
};

// ---------------------------------------------------------------------------
// Investment Committee Memo
// ---------------------------------------------------------------------------

static IC_MEMO: WorkflowDefinition = WorkflowDefinition {
    id: "pe-ic-memo",
    name: "Investment Committee Memo",
    domain: WorkflowDomain::PrivateEquity,
    description: "Full IC memo with deal rationale, returns analysis, risks, and recommendation",
    required_inputs: &[
        WorkflowInput {
            name: "company_name",
            input_type: InputType::CompanyName,
            required: true,
            description: "Target company name",
        },
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: false,
            description: "Ticker symbol if publicly traded",
        },
        WorkflowInput {
            name: "entry_ev",
            input_type: InputType::Numeric,
            required: true,
            description: "Entry enterprise value",
        },
        WorkflowInput {
            name: "entry_ebitda",
            input_type: InputType::Numeric,
            required: true,
            description: "Entry EBITDA",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Financials",
            description: "Pull comprehensive financial statements",
            required_tools: &["fmp_income_statement", "fmp_balance_sheet", "fmp_cash_flow"],
        },
        WorkflowStep {
            order: 2,
            name: "Returns",
            description: "Run LBO model and returns analysis",
            required_tools: &["lbo_model", "returns_calculator"],
        },
        WorkflowStep {
            order: 3,
            name: "Credit",
            description: "Assess credit metrics and distress risk",
            required_tools: &["credit_metrics", "altman_zscore"],
        },
        WorkflowStep {
            order: 4,
            name: "Valuation",
            description: "Run comparable company and DCF analysis",
            required_tools: &["comps_analysis", "dcf_model"],
        },
        WorkflowStep {
            order: 5,
            name: "Risk Assessment",
            description: "Identify and assess key risk factors",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Source Verification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
        QualityGate {
            name: "Scenario Check",
            check_type: QualityCheckType::ScenarioCheck,
            required: true,
        },
        QualityGate {
            name: "Risk First",
            check_type: QualityCheckType::RiskFirstCheck,
            required: true,
        },
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Executive Summary",
        "Investment Thesis",
        "Business Overview",
        "Financial Analysis",
        "Returns Analysis",
        "Risk Factors",
        "Exit Strategy",
        "Recommendation",
    ],
};

// ---------------------------------------------------------------------------
// Due Diligence Checklist
// ---------------------------------------------------------------------------

static DD_CHECKLIST: WorkflowDefinition = WorkflowDefinition {
    id: "pe-dd-checklist",
    name: "Due Diligence Checklist",
    domain: WorkflowDomain::PrivateEquity,
    description:
        "Comprehensive DD checklist across financial, legal, commercial, and operational streams",
    required_inputs: &[
        WorkflowInput {
            name: "company_name",
            input_type: InputType::CompanyName,
            required: true,
            description: "Target company name",
        },
        WorkflowInput {
            name: "deal_type",
            input_type: InputType::FreeText,
            required: true,
            description: "Type of deal (e.g., buyout, growth equity, recap)",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Financial DD Items",
            description: "Pull financials for DD question generation",
            required_tools: &["fmp_income_statement", "fmp_balance_sheet"],
        },
        WorkflowStep {
            order: 2,
            name: "Checklist Build",
            description: "Build comprehensive DD checklist across all streams",
            required_tools: &[],
        },
    ],
    quality_gates: &[QualityGate {
        name: "Completeness",
        check_type: QualityCheckType::CompletenessCheck,
        required: true,
    }],
    output_sections: &[
        "Financial DD",
        "Legal DD",
        "Commercial DD",
        "Operational DD",
        "Tax DD",
        "IT/Cyber DD",
        "ESG DD",
    ],
};

// ---------------------------------------------------------------------------
// DD Meeting Prep
// ---------------------------------------------------------------------------

static DD_MEETING_PREP: WorkflowDefinition = WorkflowDefinition {
    id: "pe-dd-meeting-prep",
    name: "DD Meeting Prep",
    domain: WorkflowDomain::PrivateEquity,
    description: "Preparation pack for management meetings with key questions and data requests",
    required_inputs: &[
        WorkflowInput {
            name: "company_name",
            input_type: InputType::CompanyName,
            required: true,
            description: "Target company name",
        },
        WorkflowInput {
            name: "meeting_focus",
            input_type: InputType::FreeText,
            required: true,
            description: "Focus area for the management meeting",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Background",
            description: "Pull background metrics and ratios",
            required_tools: &["fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Question Prep",
            description: "Prepare key questions and data requests",
            required_tools: &[],
        },
    ],
    quality_gates: &[QualityGate {
        name: "Completeness",
        check_type: QualityCheckType::CompletenessCheck,
        required: true,
    }],
    output_sections: &[
        "Company Background",
        "Key Questions",
        "Data Requests",
        "Red Flags to Probe",
    ],
};

// ---------------------------------------------------------------------------
// Returns Analysis
// ---------------------------------------------------------------------------

static RETURNS_ANALYSIS: WorkflowDefinition = WorkflowDefinition {
    id: "pe-returns-analysis",
    name: "Returns Analysis",
    domain: WorkflowDomain::PrivateEquity,
    description: "LBO returns with IRR/MOIC attribution across EBITDA growth, multiple expansion, and debt paydown",
    required_inputs: &[
        WorkflowInput {
            name: "entry_ev",
            input_type: InputType::Numeric,
            required: true,
            description: "Entry enterprise value",
        },
        WorkflowInput {
            name: "entry_ebitda",
            input_type: InputType::Numeric,
            required: true,
            description: "Entry EBITDA",
        },
        WorkflowInput {
            name: "exit_multiple",
            input_type: InputType::Numeric,
            required: false,
            description: "Exit EV/EBITDA multiple",
        },
        WorkflowInput {
            name: "hold_period",
            input_type: InputType::Numeric,
            required: false,
            description: "Holding period in years",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "LBO Model",
            description: "Build LBO model with debt schedule and sources/uses",
            required_tools: &["lbo_model", "debt_schedule", "sources_uses"],
        },
        WorkflowStep {
            order: 2,
            name: "Returns Calc",
            description: "Calculate IRR, MOIC, and returns attribution",
            required_tools: &["returns_calculator"],
        },
        WorkflowStep {
            order: 3,
            name: "Sensitivity",
            description: "Sensitivity analysis on key return drivers",
            required_tools: &["sensitivity_matrix"],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Source Verification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
        QualityGate {
            name: "Scenario Check",
            check_type: QualityCheckType::ScenarioCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Sources & Uses",
        "Operating Model",
        "Debt Schedule",
        "Returns Summary",
        "Attribution",
        "Sensitivity",
    ],
};

// ---------------------------------------------------------------------------
// Unit Economics
// ---------------------------------------------------------------------------

static UNIT_ECONOMICS: WorkflowDefinition = WorkflowDefinition {
    id: "pe-unit-economics",
    name: "Unit Economics",
    domain: WorkflowDomain::PrivateEquity,
    description: "Unit-level economics analysis with LTV/CAC, cohort analysis, and margin build",
    required_inputs: &[
        WorkflowInput {
            name: "company_name",
            input_type: InputType::CompanyName,
            required: true,
            description: "Target company name",
        },
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: false,
            description: "Ticker symbol if publicly traded",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Financial Data",
            description: "Pull income statement, metrics, and ratios",
            required_tools: &["fmp_income_statement", "fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Analysis",
            description: "Build unit economics and margin analysis",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Source Verification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Unit Economics Summary",
        "Revenue Build",
        "Cost Structure",
        "Margin Analysis",
        "LTV/CAC",
        "Scalability Assessment",
    ],
};

// ---------------------------------------------------------------------------
// Value Creation Plan
// ---------------------------------------------------------------------------

static VALUE_CREATION_PLAN: WorkflowDefinition = WorkflowDefinition {
    id: "pe-value-creation-plan",
    name: "Value Creation Plan",
    domain: WorkflowDomain::PrivateEquity,
    description:
        "100-day and long-term value creation plan with revenue, cost, and capital structure levers",
    required_inputs: &[
        WorkflowInput {
            name: "company_name",
            input_type: InputType::CompanyName,
            required: true,
            description: "Target company name",
        },
        WorkflowInput {
            name: "entry_ebitda",
            input_type: InputType::Numeric,
            required: true,
            description: "Entry EBITDA",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Baseline",
            description: "Pull current financials and ratios for baseline",
            required_tools: &["fmp_income_statement", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Lever Analysis",
            description: "Model revenue, cost, and capital structure levers",
            required_tools: &["three_statement_model"],
        },
        WorkflowStep {
            order: 3,
            name: "Plan Build",
            description: "Build 100-day and long-term value creation plan",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
        QualityGate {
            name: "Source Verification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
    ],
    output_sections: &[
        "Current State",
        "Revenue Levers",
        "Cost Levers",
        "Capital Structure",
        "100-Day Plan",
        "Long-Term Roadmap",
        "KPI Targets",
    ],
};

// ---------------------------------------------------------------------------
// Portfolio Monitoring
// ---------------------------------------------------------------------------

static PORTFOLIO_MONITORING: WorkflowDefinition = WorkflowDefinition {
    id: "pe-portfolio-monitoring",
    name: "Portfolio Monitoring",
    domain: WorkflowDomain::PrivateEquity,
    description:
        "Quarterly portfolio company monitoring with financial KPIs and covenant compliance",
    required_inputs: &[WorkflowInput {
        name: "tickers",
        input_type: InputType::FreeText,
        required: true,
        description: "Comma-separated portfolio tickers",
    }],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Current Data",
            description: "Pull current market data and key metrics for portfolio",
            required_tools: &["fmp_quote", "fmp_key_metrics"],
        },
        WorkflowStep {
            order: 2,
            name: "Credit Monitor",
            description: "Monitor credit metrics across portfolio",
            required_tools: &["credit_metrics"],
        },
        WorkflowStep {
            order: 3,
            name: "Dashboard",
            description: "Build portfolio monitoring dashboard",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Source Verification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Portfolio Overview",
        "Company Dashboards",
        "Covenant Compliance",
        "Watch List",
        "Action Items",
    ],
};

// ---------------------------------------------------------------------------
// Deal Sourcing
// ---------------------------------------------------------------------------

static DEAL_SOURCING: WorkflowDefinition = WorkflowDefinition {
    id: "pe-deal-sourcing",
    name: "Deal Sourcing",
    domain: WorkflowDomain::PrivateEquity,
    description: "Systematic deal sourcing with sector screening and target identification",
    required_inputs: &[
        WorkflowInput {
            name: "sector",
            input_type: InputType::FreeText,
            required: true,
            description: "Target sector for sourcing",
        },
        WorkflowInput {
            name: "criteria",
            input_type: InputType::FreeText,
            required: true,
            description: "Investment criteria and parameters",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Screen",
            description: "Screen sector for companies matching criteria",
            required_tools: &["fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Target List",
            description: "Build prioritised target list with comparables",
            required_tools: &["comps_analysis"],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
        QualityGate {
            name: "Source Verification",
            check_type: QualityCheckType::SourceVerification,
            required: true,
        },
    ],
    output_sections: &[
        "Sector Overview",
        "Screening Criteria",
        "Target List",
        "Prioritisation",
        "Outreach Plan",
    ],
};

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub static WORKFLOWS: &[&WorkflowDefinition] = &[
    &DEAL_SCREENING,
    &IC_MEMO,
    &DD_CHECKLIST,
    &DD_MEETING_PREP,
    &RETURNS_ANALYSIS,
    &UNIT_ECONOMICS,
    &VALUE_CREATION_PLAN,
    &PORTFOLIO_MONITORING,
    &DEAL_SOURCING,
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn workflow_count() {
        assert_eq!(WORKFLOWS.len(), 9, "Expected 9 PE workflows");
    }

    #[test]
    fn all_have_steps() {
        for wf in WORKFLOWS {
            assert!(!wf.steps.is_empty(), "Workflow '{}' has no steps", wf.id);
        }
    }

    #[test]
    fn ids_unique() {
        let mut ids = HashSet::new();
        for wf in WORKFLOWS {
            assert!(ids.insert(wf.id), "Duplicate workflow id: {}", wf.id);
        }
    }

    #[test]
    fn domain_is_private_equity() {
        for wf in WORKFLOWS {
            assert_eq!(
                wf.domain,
                WorkflowDomain::PrivateEquity,
                "Workflow '{}' has wrong domain: {:?}",
                wf.id,
                wf.domain
            );
        }
    }

    #[test]
    fn step_ordering() {
        for wf in WORKFLOWS {
            for (i, step) in wf.steps.iter().enumerate() {
                assert_eq!(
                    step.order,
                    (i as u32) + 1,
                    "Workflow '{}' step '{}' has order {} but expected {}",
                    wf.id,
                    step.name,
                    step.order,
                    i + 1
                );
            }
        }
    }
}
