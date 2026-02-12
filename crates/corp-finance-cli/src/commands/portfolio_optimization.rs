use clap::Args;
use serde_json::Value;

use corp_finance_core::portfolio_optimization::black_litterman_portfolio::{
    self, BlackLittermanInput,
};
use corp_finance_core::portfolio_optimization::mean_variance::{self, MeanVarianceInput};

use crate::input;

#[derive(Args)]
pub struct MeanVarianceArgs {
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Args)]
pub struct BlackLittermanPortfolioArgs {
    #[arg(long)]
    pub input: Option<String>,
}

pub fn run_mean_variance(args: MeanVarianceArgs) -> Result<Value, Box<dyn std::error::Error>> {
    let mv_input: MeanVarianceInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for mean-variance optimization".into());
    };
    let result = mean_variance::optimize_mean_variance(&mv_input)?;
    Ok(serde_json::to_value(result)?)
}

pub fn run_black_litterman_portfolio(
    args: BlackLittermanPortfolioArgs,
) -> Result<Value, Box<dyn std::error::Error>> {
    let bl_input: BlackLittermanInput = if let Some(ref path) = args.input {
        input::file::read_json(path)?
    } else if let Some(data) = input::stdin::read_stdin()? {
        serde_json::from_value(data)?
    } else {
        return Err("--input <file.json> or stdin required for Black-Litterman portfolio".into());
    };
    let result = black_litterman_portfolio::optimize_black_litterman(&bl_input)?;
    Ok(serde_json::to_value(result)?)
}
