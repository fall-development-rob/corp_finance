//! Wealth management workflow definitions.
//! Covers client reviews, financial planning, portfolio rebalancing,
//! tax-loss harvesting, client reporting, and investment proposals.

use super::types::*;

// ---------------------------------------------------------------------------
// Workflow Registry
// ---------------------------------------------------------------------------

pub static WORKFLOWS: &[&WorkflowDefinition] = &[
    &CLIENT_REVIEW,
    &FINANCIAL_PLAN,
    &PORTFOLIO_REBALANCE,
    &TAX_LOSS_HARVEST,
    &CLIENT_REPORT,
    &INVESTMENT_PROPOSAL,
];

// ---------------------------------------------------------------------------
// 1. Client Review
// ---------------------------------------------------------------------------

static CLIENT_REVIEW: WorkflowDefinition = WorkflowDefinition {
    id: "wm-client-review",
    name: "Client Review",
    domain: WorkflowDomain::WealthManagement,
    description: "Prep for client meeting",
    required_inputs: &[
        WorkflowInput {
            name: "client_name",
            input_type: InputType::FreeText,
            required: true,
            description: "Client name",
        },
        WorkflowInput {
            name: "tickers",
            input_type: InputType::FreeText,
            required: true,
            description: "Portfolio holdings",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Portfolio Data",
            description: "Gather current portfolio data and metrics",
            required_tools: &["fmp_quote", "fmp_historical_prices", "fmp_key_metrics"],
        },
        WorkflowStep {
            order: 2,
            name: "Performance",
            description: "Calculate portfolio performance and returns",
            required_tools: &["total_shareholder_return", "returns_calculator"],
        },
        WorkflowStep {
            order: 3,
            name: "Prep",
            description: "Prepare meeting agenda and discussion points",
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
            name: "Completeness Check",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Portfolio Summary",
        "Performance Review",
        "Market Update",
        "Discussion Points",
        "Action Items",
    ],
};

// ---------------------------------------------------------------------------
// 2. Financial Plan
// ---------------------------------------------------------------------------

static FINANCIAL_PLAN: WorkflowDefinition = WorkflowDefinition {
    id: "wm-financial-plan",
    name: "Financial Plan",
    domain: WorkflowDomain::WealthManagement,
    description: "Comprehensive financial plan",
    required_inputs: &[
        WorkflowInput {
            name: "client_name",
            input_type: InputType::FreeText,
            required: true,
            description: "Client name",
        },
        WorkflowInput {
            name: "current_assets",
            input_type: InputType::Numeric,
            required: true,
            description: "Current total assets",
        },
        WorkflowInput {
            name: "target_return",
            input_type: InputType::TargetReturn,
            required: false,
            description: "Target annual return",
        },
        WorkflowInput {
            name: "retirement_age",
            input_type: InputType::Numeric,
            required: false,
            description: "Target retirement age",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Current State",
            description: "Assess current financial position",
            required_tools: &["fmp_quote"],
        },
        WorkflowStep {
            order: 2,
            name: "Retirement",
            description: "Model retirement scenarios and funding gaps",
            required_tools: &["retirement_planning"],
        },
        WorkflowStep {
            order: 3,
            name: "Tax",
            description: "Develop tax and estate planning strategy",
            required_tools: &["tax_estate_planning"],
        },
        WorkflowStep {
            order: 4,
            name: "Plan Build",
            description: "Assemble comprehensive financial plan",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Completeness Check",
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
        "Executive Summary",
        "Current Financial Position",
        "Goals & Objectives",
        "Investment Strategy",
        "Retirement Planning",
        "Tax Strategy",
        "Estate Planning",
        "Action Plan",
    ],
};

// ---------------------------------------------------------------------------
// 3. Portfolio Rebalance
// ---------------------------------------------------------------------------

static PORTFOLIO_REBALANCE: WorkflowDefinition = WorkflowDefinition {
    id: "wm-portfolio-rebalance",
    name: "Portfolio Rebalance",
    domain: WorkflowDomain::WealthManagement,
    description: "Rebalancing analysis",
    required_inputs: &[
        WorkflowInput {
            name: "tickers",
            input_type: InputType::FreeText,
            required: true,
            description: "Current portfolio holdings",
        },
        WorkflowInput {
            name: "target_allocation",
            input_type: InputType::FreeText,
            required: true,
            description: "Target allocation weights",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Current Weights",
            description: "Determine current portfolio weights from market data",
            required_tools: &["fmp_quote"],
        },
        WorkflowStep {
            order: 2,
            name: "Optimization",
            description: "Run portfolio optimization for target allocation",
            required_tools: &["mean_variance_optimization", "risk_parity"],
        },
        WorkflowStep {
            order: 3,
            name: "Trade List",
            description: "Generate rebalancing trade list",
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
            name: "Completeness Check",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Current Allocation",
        "Target Allocation",
        "Drift Analysis",
        "Recommended Trades",
        "Tax Impact",
    ],
};

// ---------------------------------------------------------------------------
// 4. Tax-Loss Harvesting
// ---------------------------------------------------------------------------

static TAX_LOSS_HARVEST: WorkflowDefinition = WorkflowDefinition {
    id: "wm-tax-loss-harvest",
    name: "Tax-Loss Harvesting",
    domain: WorkflowDomain::WealthManagement,
    description: "TLH opportunity scan",
    required_inputs: &[
        WorkflowInput {
            name: "tickers",
            input_type: InputType::FreeText,
            required: true,
            description: "Portfolio holdings",
        },
        WorkflowInput {
            name: "cost_basis",
            input_type: InputType::FreeText,
            required: true,
            description: "JSON of ticker:cost_basis",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Price Check",
            description: "Retrieve current and historical prices for loss identification",
            required_tools: &["fmp_quote", "fmp_historical_prices"],
        },
        WorkflowStep {
            order: 2,
            name: "Harvest Candidates",
            description: "Identify tax-loss harvesting candidates and replacements",
            required_tools: &[],
        },
    ],
    quality_gates: &[QualityGate {
        name: "Source Verification",
        check_type: QualityCheckType::SourceVerification,
        required: true,
    }],
    output_sections: &[
        "Loss Candidates",
        "Wash Sale Rules",
        "Replacement Securities",
        "Tax Impact Estimate",
    ],
};

// ---------------------------------------------------------------------------
// 5. Client Report
// ---------------------------------------------------------------------------

static CLIENT_REPORT: WorkflowDefinition = WorkflowDefinition {
    id: "wm-client-report",
    name: "Client Report",
    domain: WorkflowDomain::WealthManagement,
    description: "Quarterly performance report",
    required_inputs: &[
        WorkflowInput {
            name: "client_name",
            input_type: InputType::FreeText,
            required: true,
            description: "Client name",
        },
        WorkflowInput {
            name: "tickers",
            input_type: InputType::FreeText,
            required: true,
            description: "Portfolio holdings",
        },
        WorkflowInput {
            name: "period",
            input_type: InputType::FreeText,
            required: true,
            description: "Reporting period",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Data",
            description: "Collect market data for the reporting period",
            required_tools: &["fmp_quote", "fmp_historical_prices"],
        },
        WorkflowStep {
            order: 2,
            name: "Attribution",
            description: "Perform performance and factor attribution analysis",
            required_tools: &["brinson_attribution", "factor_attribution"],
        },
        WorkflowStep {
            order: 3,
            name: "Report",
            description: "Compile quarterly performance report",
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
            name: "Completeness Check",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Portfolio Performance",
        "Benchmark Comparison",
        "Attribution Analysis",
        "Market Commentary",
        "Outlook",
    ],
};

// ---------------------------------------------------------------------------
// 6. Investment Proposal
// ---------------------------------------------------------------------------

static INVESTMENT_PROPOSAL: WorkflowDefinition = WorkflowDefinition {
    id: "wm-investment-proposal",
    name: "Investment Proposal",
    domain: WorkflowDomain::WealthManagement,
    description: "New investment proposal",
    required_inputs: &[
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: true,
            description: "Investment ticker symbol",
        },
        WorkflowInput {
            name: "rationale",
            input_type: InputType::FreeText,
            required: true,
            description: "Investment rationale",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Research",
            description: "Gather fundamental data and financial statements",
            required_tools: &[
                "fmp_quote",
                "fmp_key_metrics",
                "fmp_ratios",
                "fmp_income_statement",
            ],
        },
        WorkflowStep {
            order: 2,
            name: "Valuation",
            description: "Run DCF and comparable company analysis",
            required_tools: &["dcf_model", "comps_analysis"],
        },
        WorkflowStep {
            order: 3,
            name: "Proposal",
            description: "Build investment proposal with recommendation",
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
            name: "Risk First Check",
            check_type: QualityCheckType::RiskFirstCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Investment Thesis",
        "Company Overview",
        "Valuation",
        "Risk Assessment",
        "Portfolio Fit",
        "Recommendation",
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
        assert_eq!(WORKFLOWS.len(), 6, "Expected 6 wealth management workflows");
    }

    #[test]
    fn test_all_have_steps() {
        for w in WORKFLOWS {
            assert!(
                !w.steps.is_empty(),
                "Workflow '{}' must have at least one step",
                w.id
            );
        }
    }

    #[test]
    fn test_ids_unique() {
        let mut ids = std::collections::HashSet::new();
        for w in WORKFLOWS {
            assert!(ids.insert(w.id), "Duplicate workflow id: {}", w.id);
        }
    }

    #[test]
    fn test_domain() {
        for w in WORKFLOWS {
            assert_eq!(
                w.domain,
                WorkflowDomain::WealthManagement,
                "Workflow '{}' should be WealthManagement domain",
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
                    "Step '{}' in workflow '{}' has wrong order: expected {}, got {}",
                    step.name,
                    w.id,
                    i + 1,
                    step.order
                );
            }
        }
    }
}
