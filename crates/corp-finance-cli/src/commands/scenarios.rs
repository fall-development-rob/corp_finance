use clap::Args;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use corp_finance_core::credit::metrics::{self, CreditMetricsInput};
use corp_finance_core::valuation::wacc::{self, WaccInput};

use crate::input;

/// Arguments for sensitivity analysis
#[derive(Args)]
pub struct SensitivityArgs {
    /// Model to run sensitivity on: wacc, credit
    #[arg(long)]
    pub model: String,

    /// First sensitivity variable in format name:min:max:step
    /// (e.g. "risk_free_rate:0.02:0.06:0.005")
    #[arg(long)]
    pub var1: String,

    /// Second sensitivity variable (optional, creates a 2D table)
    #[arg(long)]
    pub var2: Option<String>,

    /// Path to JSON file with base case inputs
    #[arg(long)]
    pub base_inputs: String,
}

#[derive(Debug, Clone)]
struct SensVar {
    name: String,
    min: Decimal,
    max: Decimal,
    step: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
struct SensitivityOutput {
    model: String,
    var1_name: String,
    var2_name: Option<String>,
    results: Vec<SensitivityRow>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SensitivityRow {
    var1_value: Decimal,
    var2_value: Option<Decimal>,
    output_value: Decimal,
    output_label: String,
}

fn parse_sens_var(spec: &str) -> Result<SensVar, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 4 {
        return Err(format!(
            "Sensitivity variable must be name:min:max:step, got '{}'",
            spec
        )
        .into());
    }
    Ok(SensVar {
        name: parts[0].to_string(),
        min: parts[1].parse()?,
        max: parts[2].parse()?,
        step: parts[3].parse()?,
    })
}

fn generate_range(var: &SensVar) -> Vec<Decimal> {
    let mut values = Vec::new();
    let mut current = var.min;
    while current <= var.max {
        values.push(current);
        current += var.step;
    }
    if values.is_empty() {
        values.push(var.min);
    }
    values
}

fn set_json_field(obj: &mut Value, field: &str, value: Decimal) {
    if let Some(map) = obj.as_object_mut() {
        map.insert(field.to_string(), Value::String(value.to_string()));
    }
}

pub fn run_sensitivity(args: SensitivityArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let var1 = parse_sens_var(&args.var1)?;
    let var2 = args.var2.as_ref().map(|s| parse_sens_var(s)).transpose()?;

    let base_json: Value = input::file::read_json_value(&args.base_inputs)?;

    let var1_range = generate_range(&var1);
    let var2_range = var2.as_ref().map(generate_range);

    let mut results = Vec::new();

    match args.model.to_lowercase().as_str() {
        "wacc" => {
            let run = |v1: Decimal,
                       v2: Option<Decimal>|
             -> Result<SensitivityRow, Box<dyn std::error::Error>> {
                let mut json = base_json.clone();
                set_json_field(&mut json, &var1.name, v1);
                if let (Some(ref v2_var), Some(v2_val)) = (&var2, v2) {
                    set_json_field(&mut json, &v2_var.name, v2_val);
                }
                let wacc_input: WaccInput = serde_json::from_value(json)?;
                let result = wacc::calculate_wacc(&wacc_input)?;
                Ok(SensitivityRow {
                    var1_value: v1,
                    var2_value: v2,
                    output_value: result.result.wacc,
                    output_label: "wacc".to_string(),
                })
            };

            if let Some(ref v2_vals) = var2_range {
                for &v1 in &var1_range {
                    for &v2 in v2_vals {
                        results.push(run(v1, Some(v2))?);
                    }
                }
            } else {
                for &v1 in &var1_range {
                    results.push(run(v1, None)?);
                }
            }
        }
        "credit" => {
            let run = |v1: Decimal,
                       v2: Option<Decimal>|
             -> Result<SensitivityRow, Box<dyn std::error::Error>> {
                let mut json = base_json.clone();
                set_json_field(&mut json, &var1.name, v1);
                if let (Some(ref v2_var), Some(v2_val)) = (&var2, v2) {
                    set_json_field(&mut json, &v2_var.name, v2_val);
                }
                let credit_input: CreditMetricsInput = serde_json::from_value(json)?;
                let result = metrics::calculate_credit_metrics(&credit_input)?;
                Ok(SensitivityRow {
                    var1_value: v1,
                    var2_value: v2,
                    output_value: result.result.net_debt_to_ebitda,
                    output_label: "net_debt_to_ebitda".to_string(),
                })
            };

            if let Some(ref v2_vals) = var2_range {
                for &v1 in &var1_range {
                    for &v2 in v2_vals {
                        results.push(run(v1, Some(v2))?);
                    }
                }
            } else {
                for &v1 in &var1_range {
                    results.push(run(v1, None)?);
                }
            }
        }
        other => {
            return Err(
                format!("Unknown model '{}'. Available models: wacc, credit", other).into(),
            );
        }
    }

    let output = SensitivityOutput {
        model: args.model,
        var1_name: var1.name,
        var2_name: var2.map(|v| v.name),
        results,
    };

    Ok(serde_json::to_value(output)?)
}
