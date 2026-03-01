//! Workflow audit trail generation for compliance and auditability.
//! Produces deterministic hashes for all inputs and outputs.

use serde::{Deserialize, Serialize};

#[cfg(test)]
use super::types::{QualityResult, StepResult, ToolCallRecord};
use super::types::{WorkflowExecution, WorkflowStatus};
use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAuditInput {
    pub workflow_id: String,
    pub execution: WorkflowExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAuditOutput {
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub total_tool_calls: u32,
    pub unique_tools_used: Vec<String>,
    pub quality_score: Option<String>,
    pub quality_gates_passed: u32,
    pub quality_gates_total: u32,
    pub step_audit: Vec<StepAudit>,
    pub audit_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepAudit {
    pub step_order: u32,
    pub step_name: String,
    pub completed: bool,
    pub tool_count: u32,
    pub tools: Vec<String>,
}

// ---------------------------------------------------------------------------
// Simple deterministic hash (no external dep — just djb2 for audit fingerprint)
// ---------------------------------------------------------------------------

fn djb2_hash(data: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in data.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{hash:016x}")
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn generate_audit_trail(input: &WorkflowAuditInput) -> CorpFinanceResult<WorkflowAuditOutput> {
    let exec = &input.execution;

    let mut total_tool_calls = 0u32;
    let mut unique_tools = std::collections::BTreeSet::new();
    let mut step_audit = Vec::new();

    for step in &exec.completed_steps {
        let tool_count = step.tools_used.len() as u32;
        total_tool_calls += tool_count;
        let tools: Vec<String> = step
            .tools_used
            .iter()
            .map(|t| t.tool_name.clone())
            .collect();
        for t in &tools {
            unique_tools.insert(t.clone());
        }
        step_audit.push(StepAudit {
            step_order: step.step_order,
            step_name: step.step_name.clone(),
            completed: step.completed,
            tool_count,
            tools,
        });
    }

    let qg_passed = exec.quality_results.iter().filter(|q| q.passed).count() as u32;
    let qg_total = exec.quality_results.len() as u32;
    let quality_score = if qg_total > 0 {
        Some(format!("{}/{}", qg_passed, qg_total))
    } else {
        None
    };

    // Deterministic audit fingerprint from execution data
    let audit_data = serde_json::to_string(&exec)
        .map_err(|e| CorpFinanceError::SerializationError(e.to_string()))?;
    let audit_hash = djb2_hash(&audit_data);

    Ok(WorkflowAuditOutput {
        workflow_id: input.workflow_id.clone(),
        status: exec.status,
        total_steps: exec.total_steps,
        completed_steps: exec.completed_steps.len() as u32,
        total_tool_calls,
        unique_tools_used: unique_tools.into_iter().collect(),
        quality_score,
        quality_gates_passed: qg_passed,
        quality_gates_total: qg_total,
        step_audit,
        audit_hash,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflows::types::QualityCheckType;

    fn sample_execution() -> WorkflowExecution {
        WorkflowExecution {
            workflow_id: "er-initiating-coverage".to_string(),
            status: WorkflowStatus::Complete,
            current_step: 5,
            total_steps: 5,
            completed_steps: vec![
                StepResult {
                    step_order: 1,
                    step_name: "Data Collection".to_string(),
                    tools_used: vec![
                        ToolCallRecord {
                            tool_name: "fmp_income_statement".to_string(),
                            input_hash: "abc123".to_string(),
                            output_hash: "def456".to_string(),
                            timestamp: "2024-01-01T10:00:00Z".to_string(),
                        },
                        ToolCallRecord {
                            tool_name: "fmp_balance_sheet".to_string(),
                            input_hash: "ghi789".to_string(),
                            output_hash: "jkl012".to_string(),
                            timestamp: "2024-01-01T10:00:01Z".to_string(),
                        },
                    ],
                    outputs: serde_json::json!({"revenue": 1000000}),
                    completed: true,
                },
                StepResult {
                    step_order: 2,
                    step_name: "Valuation".to_string(),
                    tools_used: vec![ToolCallRecord {
                        tool_name: "dcf_model".to_string(),
                        input_hash: "mno345".to_string(),
                        output_hash: "pqr678".to_string(),
                        timestamp: "2024-01-01T10:01:00Z".to_string(),
                    }],
                    outputs: serde_json::json!({"fair_value": 150}),
                    completed: true,
                },
            ],
            quality_results: vec![
                QualityResult {
                    gate_name: "Completeness".to_string(),
                    check_type: QualityCheckType::CompletenessCheck,
                    passed: true,
                    details: "5/5 sections".to_string(),
                },
                QualityResult {
                    gate_name: "Source Verification".to_string(),
                    check_type: QualityCheckType::SourceVerification,
                    passed: true,
                    details: "3 tool calls".to_string(),
                },
            ],
        }
    }

    #[test]
    fn test_audit_trail_generation() {
        let input = WorkflowAuditInput {
            workflow_id: "er-initiating-coverage".to_string(),
            execution: sample_execution(),
        };
        let output = generate_audit_trail(&input).unwrap();
        assert_eq!(output.workflow_id, "er-initiating-coverage");
        assert_eq!(output.completed_steps, 2);
        assert_eq!(output.total_tool_calls, 3);
        assert_eq!(output.unique_tools_used.len(), 3);
        assert!(output.unique_tools_used.contains(&"dcf_model".to_string()));
        assert_eq!(output.quality_gates_passed, 2);
        assert_eq!(output.quality_gates_total, 2);
        assert!(!output.audit_hash.is_empty());
    }

    #[test]
    fn test_audit_deterministic() {
        let input = WorkflowAuditInput {
            workflow_id: "test".to_string(),
            execution: sample_execution(),
        };
        let out1 = generate_audit_trail(&input).unwrap();
        let out2 = generate_audit_trail(&input).unwrap();
        assert_eq!(
            out1.audit_hash, out2.audit_hash,
            "Audit hash must be deterministic"
        );
    }

    #[test]
    fn test_audit_empty_execution() {
        let input = WorkflowAuditInput {
            workflow_id: "test".to_string(),
            execution: WorkflowExecution {
                workflow_id: "test".to_string(),
                status: WorkflowStatus::NotStarted,
                current_step: 0,
                total_steps: 3,
                completed_steps: vec![],
                quality_results: vec![],
            },
        };
        let output = generate_audit_trail(&input).unwrap();
        assert_eq!(output.completed_steps, 0);
        assert_eq!(output.total_tool_calls, 0);
        assert!(output.quality_score.is_none());
    }
}
