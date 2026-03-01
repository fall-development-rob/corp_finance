//! Investment banking workflow definitions.
//! Static compile-time definitions for IB document production pipelines.

use super::types::*;

// ---------------------------------------------------------------------------
// CIM Builder
// ---------------------------------------------------------------------------

static CIM_BUILDER: WorkflowDefinition = WorkflowDefinition {
    id: "ib-cim-builder",
    name: "Confidential Information Memorandum",
    domain: WorkflowDomain::InvestmentBanking,
    description: "Comprehensive CIM for sell-side M&A with business overview, financials, and growth opportunities",
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
            name: "deal_type",
            input_type: InputType::FreeText,
            required: true,
            description: "Sell-side/Buy-side/Recap",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Financial Analysis",
            description: "Pull comprehensive financial data for the target",
            required_tools: &[
                "fmp_income_statement",
                "fmp_balance_sheet",
                "fmp_cash_flow",
                "fmp_key_metrics",
            ],
        },
        WorkflowStep {
            order: 2,
            name: "Valuation",
            description: "Run multi-method valuation analysis",
            required_tools: &["comps_analysis", "dcf_model", "wacc_calculator"],
        },
        WorkflowStep {
            order: 3,
            name: "Market Position",
            description: "Assess market positioning and trading metrics",
            required_tools: &["fmp_ratios", "fmp_quote"],
        },
        WorkflowStep {
            order: 4,
            name: "Growth Analysis",
            description: "Model forward growth and projections",
            required_tools: &["three_statement_model"],
        },
        WorkflowStep {
            order: 5,
            name: "Document Assembly",
            description: "Compile all sections into final CIM document",
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
        QualityGate {
            name: "Confidentiality",
            check_type: QualityCheckType::ConfidentialityCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Executive Summary",
        "Investment Highlights",
        "Business Overview",
        "Industry Overview",
        "Financial Overview",
        "Growth Opportunities",
        "Transaction Overview",
    ],
};

// ---------------------------------------------------------------------------
// Deal Teaser
// ---------------------------------------------------------------------------

static DEAL_TEASER: WorkflowDefinition = WorkflowDefinition {
    id: "ib-deal-teaser",
    name: "Deal Teaser",
    domain: WorkflowDomain::InvestmentBanking,
    description: "Anonymous one-page teaser for initial buyer outreach",
    required_inputs: &[
        WorkflowInput {
            name: "company_name",
            input_type: InputType::CompanyName,
            required: true,
            description: "Target company name",
        },
        WorkflowInput {
            name: "sector",
            input_type: InputType::FreeText,
            required: true,
            description: "Industry sector",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Key Metrics",
            description: "Pull key financial metrics and ratios",
            required_tools: &["fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Teaser Draft",
            description: "Draft anonymous one-page teaser",
            required_tools: &[],
        },
    ],
    quality_gates: &[
        QualityGate {
            name: "Confidentiality",
            check_type: QualityCheckType::ConfidentialityCheck,
            required: true,
        },
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Situation Overview",
        "Company Highlights",
        "Key Financial Metrics",
        "Transaction Summary",
    ],
};

// ---------------------------------------------------------------------------
// Buyer List
// ---------------------------------------------------------------------------

static BUYER_LIST: WorkflowDefinition = WorkflowDefinition {
    id: "ib-buyer-list",
    name: "Buyer List",
    domain: WorkflowDomain::InvestmentBanking,
    description: "Strategic and financial buyer universe with ranking and rationale",
    required_inputs: &[
        WorkflowInput {
            name: "company_name",
            input_type: InputType::CompanyName,
            required: true,
            description: "Target company name",
        },
        WorkflowInput {
            name: "sector",
            input_type: InputType::FreeText,
            required: true,
            description: "Industry sector",
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
            name: "Target Profile",
            description: "Build target company financial profile",
            required_tools: &["fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Universe Build",
            description: "Identify strategic and financial buyer universe",
            required_tools: &["comps_analysis"],
        },
        WorkflowStep {
            order: 3,
            name: "Ranking",
            description: "Rank buyers by strategic fit and likelihood",
            required_tools: &[],
        },
    ],
    quality_gates: &[QualityGate {
        name: "Completeness",
        check_type: QualityCheckType::CompletenessCheck,
        required: true,
    }],
    output_sections: &[
        "Strategic Buyers",
        "Financial Sponsors",
        "Ranking Matrix",
        "Outreach Strategy",
    ],
};

// ---------------------------------------------------------------------------
// Merger Model
// ---------------------------------------------------------------------------

static MERGER_MODEL: WorkflowDefinition = WorkflowDefinition {
    id: "ib-merger-model",
    name: "Merger Model",
    domain: WorkflowDomain::InvestmentBanking,
    description: "Accretion/dilution analysis with synergy phasing and pro-forma financials",
    required_inputs: &[
        WorkflowInput {
            name: "acquirer_ticker",
            input_type: InputType::Ticker,
            required: true,
            description: "Acquirer ticker symbol",
        },
        WorkflowInput {
            name: "target_ticker",
            input_type: InputType::Ticker,
            required: true,
            description: "Target ticker symbol",
        },
        WorkflowInput {
            name: "offer_price",
            input_type: InputType::Numeric,
            required: false,
            description: "Offer price per share",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Acquirer Data",
            description: "Pull acquirer financial data and market price",
            required_tools: &["fmp_income_statement", "fmp_balance_sheet", "fmp_quote"],
        },
        WorkflowStep {
            order: 2,
            name: "Target Data",
            description: "Pull target financial data and market price",
            required_tools: &["fmp_income_statement", "fmp_balance_sheet", "fmp_quote"],
        },
        WorkflowStep {
            order: 3,
            name: "Merger Analysis",
            description: "Run merger model and credit impact analysis",
            required_tools: &["merger_model", "credit_metrics"],
        },
        WorkflowStep {
            order: 4,
            name: "Sensitivity",
            description: "Sensitivity analysis on key merger assumptions",
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
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Transaction Summary",
        "Accretion/Dilution",
        "Synergy Analysis",
        "Pro-Forma Financials",
        "Sensitivity Analysis",
        "Credit Impact",
    ],
};

// ---------------------------------------------------------------------------
// Process Letter
// ---------------------------------------------------------------------------

static PROCESS_LETTER: WorkflowDefinition = WorkflowDefinition {
    id: "ib-process-letter",
    name: "Process Letter",
    domain: WorkflowDomain::InvestmentBanking,
    description: "Bid process letter with timeline, requirements, and submission instructions",
    required_inputs: &[
        WorkflowInput {
            name: "deal_name",
            input_type: InputType::FreeText,
            required: true,
            description: "Name of the deal or transaction",
        },
        WorkflowInput {
            name: "deadline",
            input_type: InputType::FreeText,
            required: true,
            description: "Submission deadline",
        },
    ],
    steps: &[WorkflowStep {
        order: 1,
        name: "Draft",
        description: "Draft process letter with timeline and requirements",
        required_tools: &[],
    }],
    quality_gates: &[
        QualityGate {
            name: "Confidentiality",
            check_type: QualityCheckType::ConfidentialityCheck,
            required: true,
        },
        QualityGate {
            name: "Completeness",
            check_type: QualityCheckType::CompletenessCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Introduction",
        "Process Overview",
        "Timeline",
        "Submission Requirements",
        "Confidentiality",
    ],
};

// ---------------------------------------------------------------------------
// Pitch Deck
// ---------------------------------------------------------------------------

static PITCH_DECK: WorkflowDefinition = WorkflowDefinition {
    id: "ib-pitch-deck",
    name: "Pitch Deck",
    domain: WorkflowDomain::InvestmentBanking,
    description:
        "Client pitch deck with market overview, valuation perspectives, and strategic options",
    required_inputs: &[
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: true,
            description: "Company ticker symbol",
        },
        WorkflowInput {
            name: "context",
            input_type: InputType::FreeText,
            required: true,
            description: "Strategic context",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Market Data",
            description: "Pull market data, pricing history, and key metrics",
            required_tools: &["fmp_quote", "fmp_historical_prices", "fmp_key_metrics"],
        },
        WorkflowStep {
            order: 2,
            name: "Valuation",
            description: "Run comparable company and DCF valuation",
            required_tools: &["comps_analysis", "dcf_model"],
        },
        WorkflowStep {
            order: 3,
            name: "Strategic Options",
            description: "Evaluate strategic alternatives",
            required_tools: &[],
        },
        WorkflowStep {
            order: 4,
            name: "Assembly",
            description: "Assemble pitch deck with all sections",
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
        "Situation Overview",
        "Market Context",
        "Valuation Perspectives",
        "Strategic Alternatives",
        "Recommended Path",
        "Appendix",
    ],
};

// ---------------------------------------------------------------------------
// Strip Profile
// ---------------------------------------------------------------------------

static STRIP_PROFILE: WorkflowDefinition = WorkflowDefinition {
    id: "ib-strip-profile",
    name: "Strip Profile",
    domain: WorkflowDomain::InvestmentBanking,
    description: "One-page company strip profile with key metrics and valuation snapshot",
    required_inputs: &[WorkflowInput {
        name: "ticker",
        input_type: InputType::Ticker,
        required: true,
        description: "Company ticker symbol",
    }],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Data Pull",
            description: "Pull quote, metrics, ratios, and income statement data",
            required_tools: &[
                "fmp_quote",
                "fmp_key_metrics",
                "fmp_ratios",
                "fmp_income_statement",
            ],
        },
        WorkflowStep {
            order: 2,
            name: "Profile Build",
            description: "Build one-page strip profile",
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
        "Key Metrics",
        "Valuation Snapshot",
        "Trading History",
    ],
};

// ---------------------------------------------------------------------------
// Deal Tracker
// ---------------------------------------------------------------------------

static DEAL_TRACKER: WorkflowDefinition = WorkflowDefinition {
    id: "ib-deal-tracker",
    name: "Deal Tracker",
    domain: WorkflowDomain::InvestmentBanking,
    description: "Pipeline tracking with status, milestones, and next steps",
    required_inputs: &[WorkflowInput {
        name: "deals",
        input_type: InputType::FreeText,
        required: true,
        description: "Comma-separated deal names",
    }],
    steps: &[WorkflowStep {
        order: 1,
        name: "Tracker Build",
        description: "Build deal pipeline tracker with status and milestones",
        required_tools: &[],
    }],
    quality_gates: &[QualityGate {
        name: "Completeness",
        check_type: QualityCheckType::CompletenessCheck,
        required: true,
    }],
    output_sections: &[
        "Pipeline Overview",
        "Deal Status",
        "Milestones",
        "Next Steps",
    ],
};

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub static WORKFLOWS: &[&WorkflowDefinition] = &[
    &CIM_BUILDER,
    &DEAL_TEASER,
    &BUYER_LIST,
    &MERGER_MODEL,
    &PROCESS_LETTER,
    &PITCH_DECK,
    &STRIP_PROFILE,
    &DEAL_TRACKER,
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
        assert_eq!(WORKFLOWS.len(), 8, "Expected 8 IB workflows");
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
    fn domain_is_investment_banking() {
        for wf in WORKFLOWS {
            assert_eq!(
                wf.domain,
                WorkflowDomain::InvestmentBanking,
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
