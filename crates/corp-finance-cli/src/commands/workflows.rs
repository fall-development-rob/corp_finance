use clap::Args;
use serde_json::Value;

use corp_finance_core::workflows::audit;
use corp_finance_core::workflows::types;

use crate::input;

#[derive(Args)]
pub struct WorkflowListArgs {
    #[arg(long)]
    pub domain: Option<String>,
}

#[derive(Args)]
pub struct WorkflowDescribeArgs {
    #[arg(long)]
    pub workflow_id: String,
}

#[derive(Args)]
pub struct WorkflowValidateArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct WorkflowQualityCheckArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct WorkflowAuditArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_workflow_list(args: WorkflowListArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data = types::WorkflowListInput {
        domain: args.domain,
    };
    let result = types::list_workflows(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_workflow_describe(
    args: WorkflowDescribeArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data = types::WorkflowDescribeInput {
        workflow_id: args.workflow_id,
    };
    let result = types::describe_workflow(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_workflow_validate(
    args: WorkflowValidateArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: types::WorkflowValidateInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = types::validate_workflow(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_workflow_quality_check(
    args: WorkflowQualityCheckArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: types::WorkflowQualityCheckInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = types::quality_check_workflow(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_workflow_audit(args: WorkflowAuditArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: audit::WorkflowAuditInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = audit::generate_audit_trail(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
