mod commands;
mod input;
mod output;

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::process;

use commands::credit::{CreditArgs, DebtCapacityArgs, CovenantArgs};
use commands::pe::ReturnsArgs;
use commands::portfolio::{SharpeArgs, RiskArgs, KellyArgs};
use commands::scenarios::SensitivityArgs;
use commands::valuation::{WaccArgs, DcfArgs, CompsArgs};

/// Institutional-grade corporate finance calculations
#[derive(Parser)]
#[command(
    name = "cfa",
    version,
    about = "Institutional-grade corporate finance calculations",
    long_about = "A CLI for performing institutional-grade corporate finance calculations \
                  with decimal precision. Supports WACC, DCF, comps, credit metrics, \
                  PE returns, portfolio analytics, and sensitivity analysis."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, default_value = "json", global = true)]
    output: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Calculate Weighted Average Cost of Capital (CAPM build-up)
    Wacc(WaccArgs),
    /// Run a Discounted Cash Flow valuation
    Dcf(DcfArgs),
    /// Comparable company analysis
    Comps(CompsArgs),
    /// Calculate credit metrics from financial statements
    CreditMetrics(CreditArgs),
    /// Estimate debt capacity
    DebtCapacity(DebtCapacityArgs),
    /// Run covenant compliance tests
    CovenantTest(CovenantArgs),
    /// Calculate PE fund returns (IRR, MOIC, Cash-on-Cash)
    Returns(ReturnsArgs),
    /// Run sensitivity analysis on any model
    Sensitivity(SensitivityArgs),
    /// Calculate Sharpe ratio
    Sharpe(SharpeArgs),
    /// Portfolio risk metrics (VaR, CVaR)
    Risk(RiskArgs),
    /// Kelly criterion position sizing
    Kelly(KellyArgs),
    /// Print version information
    Version,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Csv,
    Minimal,
}

fn main() {
    let cli = Cli::parse();

    let result: Result<serde_json::Value, Box<dyn std::error::Error>> = match cli.command {
        Commands::Wacc(args) => commands::valuation::run_wacc(args),
        Commands::Dcf(args) => commands::valuation::run_dcf(args),
        Commands::Comps(args) => commands::valuation::run_comps(args),
        Commands::CreditMetrics(args) => commands::credit::run_credit_metrics(args),
        Commands::DebtCapacity(args) => commands::credit::run_debt_capacity(args),
        Commands::CovenantTest(args) => commands::credit::run_covenant_test(args),
        Commands::Returns(args) => commands::pe::run_returns(args),
        Commands::Sensitivity(args) => commands::scenarios::run_sensitivity(args),
        Commands::Sharpe(args) => commands::portfolio::run_sharpe(args),
        Commands::Risk(args) => commands::portfolio::run_risk(args),
        Commands::Kelly(args) => commands::portfolio::run_kelly(args),
        Commands::Version => {
            println!("cfa {}", env!("CARGO_PKG_VERSION"));
            return;
        }
    };

    match result {
        Ok(value) => {
            output::format_output(&cli.output, &value);
            process::exit(0);
        }
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            process::exit(1);
        }
    }
}
