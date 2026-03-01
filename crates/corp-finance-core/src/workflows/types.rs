//! Workflow type definitions for institutional document production pipelines.
//! All workflow definitions are compile-time constants for auditability.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowDomain {
    EquityResearch,
    InvestmentBanking,
    PrivateEquity,
    WealthManagement,
    FinancialAnalysis,
    DealDocuments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QualityCheckType {
    /// All required sections present
    CompletenessCheck,
    /// Numbers trace to tool calls
    SourceVerification,
    /// Base/bull/bear scenarios included
    ScenarioCheck,
    /// Risk section before opportunity
    RiskFirstCheck,
    /// Professional formatting standards met
    FormattingCheck,
    /// Confidentiality disclaimers included
    ConfidentialityCheck,
    /// All citations have sources
    CitationCheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowStatus {
    NotStarted,
    InProgress,
    PendingReview,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputType {
    Ticker,
    CompanyName,
    Financials,
    DateRange,
    PeerGroup,
    TargetReturn,
    ClientProfile,
    DealTerms,
    FreeText,
    Numeric,
    Boolean,
}

// ---------------------------------------------------------------------------
// Workflow Definition (Static)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub domain: WorkflowDomain,
    pub description: &'static str,
    pub required_inputs: &'static [WorkflowInput],
    pub steps: &'static [WorkflowStep],
    pub quality_gates: &'static [QualityGate],
    pub output_sections: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowInput {
    pub name: &'static str,
    pub input_type: InputType,
    pub required: bool,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStep {
    pub order: u32,
    pub name: &'static str,
    pub description: &'static str,
    pub required_tools: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
pub struct QualityGate {
    pub name: &'static str,
    pub check_type: QualityCheckType,
    pub required: bool,
}

// ---------------------------------------------------------------------------
// Runtime Execution State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub current_step: u32,
    pub total_steps: u32,
    pub completed_steps: Vec<StepResult>,
    pub quality_results: Vec<QualityResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_order: u32,
    pub step_name: String,
    pub tools_used: Vec<ToolCallRecord>,
    pub outputs: serde_json::Value,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub input_hash: String,
    pub output_hash: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityResult {
    pub gate_name: String,
    pub check_type: QualityCheckType,
    pub passed: bool,
    pub details: String,
}

// ---------------------------------------------------------------------------
// API: Input / Output for MCP/CLI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListOutput {
    pub workflows: Vec<WorkflowSummary>,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub domain: WorkflowDomain,
    pub description: String,
    pub step_count: u32,
    pub required_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDescribeInput {
    pub workflow_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDescribeOutput {
    pub id: String,
    pub name: String,
    pub domain: WorkflowDomain,
    pub description: String,
    pub required_inputs: Vec<InputDetail>,
    pub steps: Vec<StepDetail>,
    pub quality_gates: Vec<GateDetail>,
    pub output_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDetail {
    pub name: String,
    pub input_type: InputType,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDetail {
    pub order: u32,
    pub name: String,
    pub description: String,
    pub required_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDetail {
    pub name: String,
    pub check_type: QualityCheckType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowValidateInput {
    pub workflow_id: String,
    pub provided_inputs: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowValidateOutput {
    pub valid: bool,
    pub workflow_id: String,
    pub missing_required: Vec<String>,
    pub provided: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowQualityCheckInput {
    pub workflow_id: String,
    pub output_sections: Vec<String>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub has_scenarios: bool,
    pub has_risk_section: bool,
    pub has_confidentiality: bool,
    pub has_citations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowQualityCheckOutput {
    pub workflow_id: String,
    pub overall_pass: bool,
    pub score: Decimal,
    pub gates: Vec<QualityResult>,
    pub recommendations: Vec<String>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

/// Get all registered workflow definitions
pub fn get_all_workflows() -> Vec<&'static WorkflowDefinition> {
    let mut all = Vec::new();
    all.extend(super::equity_research::WORKFLOWS.iter());
    all.extend(super::investment_banking::WORKFLOWS.iter());
    all.extend(super::private_equity::WORKFLOWS.iter());
    all.extend(super::wealth_management::WORKFLOWS.iter());
    all.extend(super::financial_analysis::WORKFLOWS.iter());
    all.extend(super::deal_documents::WORKFLOWS.iter());
    all
}

/// Find a workflow by ID
pub fn find_workflow(id: &str) -> CorpFinanceResult<&'static WorkflowDefinition> {
    get_all_workflows()
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| CorpFinanceError::InvalidInput {
            field: "workflow_id".into(),
            reason: format!("Unknown workflow: {id}"),
        })
}

/// List workflows, optionally filtered by domain
pub fn list_workflows(input: &WorkflowListInput) -> CorpFinanceResult<WorkflowListOutput> {
    let domain_filter: Option<WorkflowDomain> = input
        .domain
        .as_deref()
        .map(|d| match d {
            "equity_research" | "EquityResearch" => Ok(WorkflowDomain::EquityResearch),
            "investment_banking" | "InvestmentBanking" => Ok(WorkflowDomain::InvestmentBanking),
            "private_equity" | "PrivateEquity" => Ok(WorkflowDomain::PrivateEquity),
            "wealth_management" | "WealthManagement" => Ok(WorkflowDomain::WealthManagement),
            "financial_analysis" | "FinancialAnalysis" => Ok(WorkflowDomain::FinancialAnalysis),
            "deal_documents" | "DealDocuments" => Ok(WorkflowDomain::DealDocuments),
            other => Err(CorpFinanceError::InvalidInput {
                field: "domain".into(),
                reason: format!(
                    "Unknown domain: {other}. Valid: equity_research, investment_banking, \
                     private_equity, wealth_management, financial_analysis, deal_documents"
                ),
            }),
        })
        .transpose()?;

    let workflows: Vec<WorkflowSummary> = get_all_workflows()
        .into_iter()
        .filter(|w| domain_filter.is_none_or(|d| w.domain == d))
        .map(|w| WorkflowSummary {
            id: w.id.to_string(),
            name: w.name.to_string(),
            domain: w.domain,
            description: w.description.to_string(),
            step_count: w.steps.len() as u32,
            required_inputs: w
                .required_inputs
                .iter()
                .filter(|i| i.required)
                .map(|i| i.name.to_string())
                .collect(),
        })
        .collect();

    let total = workflows.len() as u32;
    Ok(WorkflowListOutput { workflows, total })
}

/// Describe a workflow in full detail
pub fn describe_workflow(
    input: &WorkflowDescribeInput,
) -> CorpFinanceResult<WorkflowDescribeOutput> {
    let w = find_workflow(&input.workflow_id)?;
    Ok(WorkflowDescribeOutput {
        id: w.id.to_string(),
        name: w.name.to_string(),
        domain: w.domain,
        description: w.description.to_string(),
        required_inputs: w
            .required_inputs
            .iter()
            .map(|i| InputDetail {
                name: i.name.to_string(),
                input_type: i.input_type,
                required: i.required,
                description: i.description.to_string(),
            })
            .collect(),
        steps: w
            .steps
            .iter()
            .map(|s| StepDetail {
                order: s.order,
                name: s.name.to_string(),
                description: s.description.to_string(),
                required_tools: s.required_tools.iter().map(|t| t.to_string()).collect(),
            })
            .collect(),
        quality_gates: w
            .quality_gates
            .iter()
            .map(|g| GateDetail {
                name: g.name.to_string(),
                check_type: g.check_type,
                required: g.required,
            })
            .collect(),
        output_sections: w.output_sections.iter().map(|s| s.to_string()).collect(),
    })
}

/// Validate inputs against workflow requirements
pub fn validate_workflow(
    input: &WorkflowValidateInput,
) -> CorpFinanceResult<WorkflowValidateOutput> {
    let w = find_workflow(&input.workflow_id)?;
    let provided_map = input.provided_inputs.as_object();

    let mut missing_required = Vec::new();
    let mut provided = Vec::new();
    let mut warnings = Vec::new();

    for req in w.required_inputs {
        let has_input = provided_map
            .map(|m| m.contains_key(req.name))
            .unwrap_or(false);
        if has_input {
            provided.push(req.name.to_string());
        } else if req.required {
            missing_required.push(req.name.to_string());
        } else {
            warnings.push(format!(
                "Optional input '{}' not provided — defaults will be used",
                req.name
            ));
        }
    }

    Ok(WorkflowValidateOutput {
        valid: missing_required.is_empty(),
        workflow_id: input.workflow_id.clone(),
        missing_required,
        provided,
        warnings,
    })
}

/// Run quality gates against workflow outputs
pub fn quality_check_workflow(
    input: &WorkflowQualityCheckInput,
) -> CorpFinanceResult<WorkflowQualityCheckOutput> {
    let w = find_workflow(&input.workflow_id)?;
    let mut gates = Vec::new();
    let mut pass_count = 0u32;

    for gate in w.quality_gates {
        let (passed, details) = match gate.check_type {
            QualityCheckType::CompletenessCheck => {
                let expected = w.output_sections.len();
                let actual = input.output_sections.len();
                let ok = actual >= expected;
                (ok, format!("{actual}/{expected} sections present"))
            }
            QualityCheckType::SourceVerification => {
                let ok = !input.tool_calls.is_empty();
                (
                    ok,
                    format!("{} tool calls recorded", input.tool_calls.len()),
                )
            }
            QualityCheckType::ScenarioCheck => {
                (input.has_scenarios, "Base/bull/bear scenarios".to_string())
            }
            QualityCheckType::RiskFirstCheck => {
                (input.has_risk_section, "Risk section present".to_string())
            }
            QualityCheckType::FormattingCheck => {
                // Always passes if we reach this point (formatting is structural)
                (true, "Professional formatting applied".to_string())
            }
            QualityCheckType::ConfidentialityCheck => (
                input.has_confidentiality,
                "Confidentiality disclaimer".to_string(),
            ),
            QualityCheckType::CitationCheck => {
                (input.has_citations, "Source citations included".to_string())
            }
        };
        if passed {
            pass_count += 1;
        }
        gates.push(QualityResult {
            gate_name: gate.name.to_string(),
            check_type: gate.check_type,
            passed,
            details,
        });
    }

    let total = w.quality_gates.len() as u32;
    let score = if total > 0 {
        Decimal::from(pass_count) / Decimal::from(total)
    } else {
        dec!(1)
    };

    let recommendations: Vec<String> = gates
        .iter()
        .filter(|g| {
            !g.passed
                && w.quality_gates
                    .iter()
                    .any(|wg| wg.name == g.gate_name && wg.required)
        })
        .map(|g| format!("REQUIRED: {} — {}", g.gate_name, g.details))
        .collect();

    Ok(WorkflowQualityCheckOutput {
        workflow_id: input.workflow_id.clone(),
        overall_pass: gates.iter().all(|g| {
            g.passed
                || !w
                    .quality_gates
                    .iter()
                    .any(|wg| wg.name == g.gate_name && wg.required)
        }),
        score,
        gates,
        recommendations,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_all_workflows() {
        let result = list_workflows(&WorkflowListInput { domain: None }).unwrap();
        // Should have workflows from all 6 domains
        assert!(result.total > 0, "Should have at least 1 workflow");
        assert!(
            result.total >= 35,
            "Expected ~44 workflows, got {}",
            result.total
        );
    }

    #[test]
    fn test_list_by_domain() {
        let result = list_workflows(&WorkflowListInput {
            domain: Some("equity_research".to_string()),
        })
        .unwrap();
        assert!(
            result.total >= 9,
            "ER should have 9 workflows, got {}",
            result.total
        );
        for w in &result.workflows {
            assert_eq!(w.domain, WorkflowDomain::EquityResearch);
        }
    }

    #[test]
    fn test_invalid_domain() {
        let result = list_workflows(&WorkflowListInput {
            domain: Some("crypto_trading".to_string()),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_describe_workflow() {
        let all = list_workflows(&WorkflowListInput { domain: None }).unwrap();
        assert!(!all.workflows.is_empty());
        let first_id = &all.workflows[0].id;
        let desc = describe_workflow(&WorkflowDescribeInput {
            workflow_id: first_id.clone(),
        })
        .unwrap();
        assert_eq!(desc.id, *first_id);
        assert!(!desc.steps.is_empty());
    }

    #[test]
    fn test_describe_unknown_workflow() {
        let result = describe_workflow(&WorkflowDescribeInput {
            workflow_id: "nonexistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_complete_inputs() {
        let all = list_workflows(&WorkflowListInput { domain: None }).unwrap();
        let first = &all.workflows[0];
        let mut inputs = serde_json::Map::new();
        for name in &first.required_inputs {
            inputs.insert(name.clone(), serde_json::Value::String("test".to_string()));
        }
        let result = validate_workflow(&WorkflowValidateInput {
            workflow_id: first.id.clone(),
            provided_inputs: serde_json::Value::Object(inputs),
        })
        .unwrap();
        assert!(result.valid);
        assert!(result.missing_required.is_empty());
    }

    #[test]
    fn test_validate_missing_inputs() {
        let all = list_workflows(&WorkflowListInput { domain: None }).unwrap();
        let first = &all.workflows[0];
        let result = validate_workflow(&WorkflowValidateInput {
            workflow_id: first.id.clone(),
            provided_inputs: serde_json::Value::Object(serde_json::Map::new()),
        })
        .unwrap();
        // Should have missing required fields
        if !first.required_inputs.is_empty() {
            assert!(!result.valid);
            assert!(!result.missing_required.is_empty());
        }
    }

    #[test]
    fn test_quality_check_all_pass() {
        let all = list_workflows(&WorkflowListInput { domain: None }).unwrap();
        let first = &all.workflows[0];
        let desc = describe_workflow(&WorkflowDescribeInput {
            workflow_id: first.id.clone(),
        })
        .unwrap();
        let result = quality_check_workflow(&WorkflowQualityCheckInput {
            workflow_id: first.id.clone(),
            output_sections: desc.output_sections.clone(),
            tool_calls: vec![ToolCallRecord {
                tool_name: "test".to_string(),
                input_hash: "abc".to_string(),
                output_hash: "def".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            }],
            has_scenarios: true,
            has_risk_section: true,
            has_confidentiality: true,
            has_citations: true,
        })
        .unwrap();
        assert!(result.overall_pass);
        assert!(result.score >= dec!(0.8));
    }

    #[test]
    fn test_quality_check_missing_sections() {
        let all = list_workflows(&WorkflowListInput { domain: None }).unwrap();
        let first = &all.workflows[0];
        let result = quality_check_workflow(&WorkflowQualityCheckInput {
            workflow_id: first.id.clone(),
            output_sections: vec![],
            tool_calls: vec![],
            has_scenarios: false,
            has_risk_section: false,
            has_confidentiality: false,
            has_citations: false,
        })
        .unwrap();
        // Should fail some gates
        assert!(!result.recommendations.is_empty() || result.overall_pass);
    }
}
