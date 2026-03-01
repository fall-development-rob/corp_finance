//! Financial analysis workflow definitions.
//! Covers model auditing, deck reviews, and competitive analysis.

use super::types::*;

// ---------------------------------------------------------------------------
// Workflow Registry
// ---------------------------------------------------------------------------

pub static WORKFLOWS: &[&WorkflowDefinition] = &[&MODEL_AUDIT, &DECK_REVIEW, &COMPETITIVE_ANALYSIS];

// ---------------------------------------------------------------------------
// 1. Model Audit
// ---------------------------------------------------------------------------

static MODEL_AUDIT: WorkflowDefinition = WorkflowDefinition {
    id: "fa-model-audit",
    name: "Model Audit",
    domain: WorkflowDomain::FinancialAnalysis,
    description: "Check financial model for errors",
    required_inputs: &[
        WorkflowInput {
            name: "model_description",
            input_type: InputType::FreeText,
            required: true,
            description: "Description of the financial model to audit",
        },
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: false,
            description: "Ticker symbol for data verification",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Data Verification",
            description: "Verify model inputs against source financial statements",
            required_tools: &["fmp_income_statement", "fmp_balance_sheet", "fmp_cash_flow"],
        },
        WorkflowStep {
            order: 2,
            name: "Cross-Check",
            description: "Cross-check model outputs against independent calculations",
            required_tools: &["three_statement_model", "dupont_analysis"],
        },
        WorkflowStep {
            order: 3,
            name: "Audit Report",
            description: "Compile audit findings and recommendations",
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
        "Model Structure",
        "Data Integrity",
        "Formula Checks",
        "Cross-Verification",
        "Error Log",
        "Recommendations",
    ],
};

// ---------------------------------------------------------------------------
// 2. Deck Review
// ---------------------------------------------------------------------------

static DECK_REVIEW: WorkflowDefinition = WorkflowDefinition {
    id: "fa-deck-review",
    name: "Deck Review",
    domain: WorkflowDomain::FinancialAnalysis,
    description: "Review presentation for accuracy",
    required_inputs: &[
        WorkflowInput {
            name: "deck_description",
            input_type: InputType::FreeText,
            required: true,
            description: "Description of the presentation to review",
        },
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: false,
            description: "Ticker symbol for fact checking",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Fact Check",
            description: "Verify facts and figures against current market data",
            required_tools: &["fmp_quote", "fmp_key_metrics"],
        },
        WorkflowStep {
            order: 2,
            name: "Review",
            description: "Review for consistency, formatting, and completeness",
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
            name: "Citation Check",
            check_type: QualityCheckType::CitationCheck,
            required: true,
        },
    ],
    output_sections: &[
        "Fact Verification",
        "Data Currency",
        "Consistency Check",
        "Formatting Review",
        "Corrections",
    ],
};

// ---------------------------------------------------------------------------
// 3. Competitive Analysis
// ---------------------------------------------------------------------------

static COMPETITIVE_ANALYSIS: WorkflowDefinition = WorkflowDefinition {
    id: "fa-competitive-analysis",
    name: "Competitive Analysis",
    domain: WorkflowDomain::FinancialAnalysis,
    description: "Competitive landscape analysis",
    required_inputs: &[
        WorkflowInput {
            name: "ticker",
            input_type: InputType::Ticker,
            required: true,
            description: "Target company ticker symbol",
        },
        WorkflowInput {
            name: "peers",
            input_type: InputType::FreeText,
            required: true,
            description: "Peer company tickers for comparison",
        },
    ],
    steps: &[
        WorkflowStep {
            order: 1,
            name: "Peer Data",
            description: "Collect financial data for target and peer companies",
            required_tools: &["fmp_quote", "fmp_key_metrics", "fmp_ratios"],
        },
        WorkflowStep {
            order: 2,
            name: "Comps",
            description: "Run comparable company analysis",
            required_tools: &["comps_analysis"],
        },
        WorkflowStep {
            order: 3,
            name: "Analysis",
            description: "Synthesise competitive positioning and strategic implications",
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
        "Industry Overview",
        "Competitive Positioning",
        "Financial Comparison",
        "SWOT",
        "Strategic Implications",
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
        assert_eq!(
            WORKFLOWS.len(),
            3,
            "Expected 3 financial analysis workflows"
        );
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
                WorkflowDomain::FinancialAnalysis,
                "Workflow '{}' should be FinancialAnalysis domain",
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
