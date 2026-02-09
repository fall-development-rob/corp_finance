use clap::Args;
use rust_decimal::Decimal;
use serde_json::Value;

use corp_finance_core::pe::returns::{self, ReturnsInput};

use crate::input;

/// Arguments for PE returns calculation
#[derive(Args)]
pub struct ReturnsArgs {
    /// Path to JSON input file (overrides individual flags)
    #[arg(long)]
    pub input: Option<String>,

    /// Equity invested at entry
    #[arg(long)]
    pub entry_equity: Option<Decimal>,

    /// Equity received at exit
    #[arg(long)]
    pub exit_equity: Option<Decimal>,

    /// Holding period in years
    #[arg(long)]
    pub holding_years: Option<Decimal>,

    /// Periodic cash flows (comma-separated, e.g. "-100,30,30,130")
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    pub cash_flows: Option<Vec<Decimal>>,
}

pub fn run_returns(args: ReturnsArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let returns_input: ReturnsInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        let entry = args
            .entry_equity
            .ok_or("--entry-equity is required (or provide --input)")?;
        let exit = args
            .exit_equity
            .ok_or("--exit-equity is required (or provide --input)")?;

        let cash_flows = args.cash_flows.unwrap_or_default();

        ReturnsInput {
            cash_flows,
            dated_cash_flows: None,
            entry_equity: entry,
            exit_equity: exit,
            holding_period_years: args.holding_years,
            dates: None,
        }
    };

    let result = returns::calculate_returns(&returns_input)?;
    Ok(serde_json::to_value(result)?)
}
