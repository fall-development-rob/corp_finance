use clap::Args;
use serde_json::Value;

use corp_finance_core::trading::analytics::{self, TradingAnalyticsInput};
use corp_finance_core::trading::diary::{self, TradingDayInput};

use crate::input;

/// Arguments for trading day analysis
#[derive(Args)]
pub struct TradingDayArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_trading_day(args: TradingDayArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let day_input: TradingDayInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for trading day analysis".into());
    };
    let result = diary::analyze_trading_day(&day_input)?;
    Ok(serde_json::to_value(result)?)
}

/// Arguments for trading performance analytics
#[derive(Args)]
pub struct TradingAnalyticsArgs {
    /// Path to JSON input file
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_trading_analytics(
    args: TradingAnalyticsArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let analytics_input: TradingAnalyticsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for trading analytics".into());
    };
    let result = analytics::analyze_trading_performance(&analytics_input)?;
    Ok(serde_json::to_value(result)?)
}
