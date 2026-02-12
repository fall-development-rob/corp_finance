use clap::Args;
use serde_json::Value;

use corp_finance_core::financial_forensics::benfords_law::{self, BenfordsLawInput};
use corp_finance_core::financial_forensics::dupont_analysis::{self, DupontInput};
use corp_finance_core::financial_forensics::peer_benchmarking::{self, PeerBenchmarkingInput};
use corp_finance_core::financial_forensics::red_flag_scoring::{self, RedFlagScoringInput};
use corp_finance_core::financial_forensics::zscore_models::{self, ZScoreModelsInput};

use crate::input;

#[derive(Args)]
pub struct BenfordsLawArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct DupontAnalysisArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct ZscoreModelsArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct PeerBenchmarkingArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct RedFlagScoringArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_benfords_law(args: BenfordsLawArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: BenfordsLawInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = benfords_law::analyze_benfords_law(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_dupont_analysis(args: DupontAnalysisArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: DupontInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = dupont_analysis::calculate_dupont(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_zscore_models(args: ZscoreModelsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: ZScoreModelsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = zscore_models::calculate_zscore_models(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_peer_benchmarking(
    args: PeerBenchmarkingArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: PeerBenchmarkingInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = peer_benchmarking::calculate_peer_benchmarking(&input_data)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_red_flag_scoring(args: RedFlagScoringArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let input_data: RedFlagScoringInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required".into());
    };
    let result = red_flag_scoring::calculate_red_flag_scoring(&input_data)?;
    Ok(serde_json::to_value(result)?)
}
