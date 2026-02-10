use clap::Args;
use serde_json::Value;

use corp_finance_core::venture::instruments::{self, ConvertibleNoteInput, SafeInput};
use corp_finance_core::venture::returns::{self, VentureFundInput};
use corp_finance_core::venture::valuation::{self, DilutionInput, FundingRoundInput};

use crate::input;

/// Arguments for funding round modelling
#[derive(Args)]
pub struct FundingRoundArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for multi-round dilution analysis
#[derive(Args)]
pub struct DilutionArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for convertible note conversion
#[derive(Args)]
pub struct ConvertibleNoteArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for SAFE conversion
#[derive(Args)]
pub struct SafeArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for venture fund returns modelling
#[derive(Args)]
pub struct VentureFundArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_funding_round(args: FundingRoundArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let fr_input: FundingRoundInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for funding round model".into());
    };
    let result = valuation::model_funding_round(&fr_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_dilution(args: DilutionArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let dil_input: DilutionInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for dilution analysis".into());
    };
    let result = valuation::analyze_dilution(&dil_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_convertible_note(
    args: ConvertibleNoteArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let cn_input: ConvertibleNoteInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for convertible note".into());
    };
    let result = instruments::convert_note(&cn_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_safe(args: SafeArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let safe_input: SafeInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for SAFE conversion".into());
    };
    let result = instruments::convert_safe(&safe_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_venture_fund(args: VentureFundArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let vf_input: VentureFundInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for venture fund model".into());
    };
    let result = returns::model_venture_fund(&vf_input)?;
    Ok(serde_json::to_value(result)?)
}
