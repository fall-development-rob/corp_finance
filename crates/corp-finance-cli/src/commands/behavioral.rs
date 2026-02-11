use clap::Args;
use serde_json::Value;

use corp_finance_core::behavioral::prospect_theory::{self, ProspectTheoryInput};
use corp_finance_core::behavioral::sentiment::{self, SentimentInput};

use crate::input;

/// Arguments for prospect theory and behavioral bias analysis
#[derive(Args)]
pub struct ProspectTheoryArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

/// Arguments for market sentiment analysis
#[derive(Args)]
pub struct SentimentArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_prospect_theory(args: ProspectTheoryArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let pt_input: ProspectTheoryInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for prospect theory analysis".into());
    };
    let result = prospect_theory::analyze_prospect_theory(&pt_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_sentiment(args: SentimentArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let sentiment_input: SentimentInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for sentiment analysis".into());
    };
    let result = sentiment::analyze_sentiment(&sentiment_input)?;
    Ok(serde_json::to_value(result)?)
}
