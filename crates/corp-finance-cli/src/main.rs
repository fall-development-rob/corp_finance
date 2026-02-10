mod commands;
mod input;
mod output;

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::process;

use commands::credit::{AltmanArgs, CovenantArgs, CreditArgs, DebtCapacityArgs};
use commands::derivatives::{
    BasisAnalysisArgs, CurrencySwapArgs, ForwardPositionArgs, ForwardPriceArgs, ImpliedVolArgs,
    IrsArgs, OptionPriceArgs, StrategyArgs,
};
use commands::fixed_income::{
    BondPricingArgs, BondYieldArgs, BootstrapArgs, CreditSpreadArgs, DurationArgs, NelsonSiegelArgs,
};
use commands::jurisdiction::{
    FundFeesArgs, GaapIfrsArgs, GpEconomicsArgs, InvestorNetReturnsArgs, NavArgs,
    UbtiScreeningArgs, WhtArgs,
};
use commands::ma::MergerArgs;
use commands::pe::{LboArgs, ReturnsArgs, WaterfallArgs};
use commands::portfolio::{KellyArgs, RiskArgs, SharpeArgs};
use commands::scenarios::SensitivityArgs;
use commands::trading::{TradingAnalyticsArgs, TradingDayArgs};
use commands::valuation::{CompsArgs, DcfArgs, WaccArgs};

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
    /// Build a full LBO model with multi-tranche debt
    Lbo(LboArgs),
    /// Calculate GP/LP distribution waterfall
    Waterfall(WaterfallArgs),
    /// Merger accretion/dilution analysis
    Merger(MergerArgs),
    /// Altman Z-Score bankruptcy prediction
    AltmanZscore(AltmanArgs),
    /// Fund fee modelling (management + performance fees)
    FundFees(FundFeesArgs),
    /// GAAP/IFRS accounting reconciliation
    GaapIfrs(GaapIfrsArgs),
    /// Withholding tax calculator
    Wht(WhtArgs),
    /// NAV calculator with equalisation
    Nav(NavArgs),
    /// GP economics model
    GpEconomics(GpEconomicsArgs),
    /// Investor net returns calculator
    InvestorNetReturns(InvestorNetReturnsArgs),
    /// UBTI/ECI screening
    UbtiScreening(UbtiScreeningArgs),
    /// Analyze a single trading day
    TradingDay(TradingDayArgs),
    /// Multi-day trading performance analytics
    TradingAnalytics(TradingAnalyticsArgs),
    /// Bond pricing (clean/dirty, accrued interest)
    BondPricing(BondPricingArgs),
    /// Bond yield calculator (YTM, BEY, effective yield)
    BondYield(BondYieldArgs),
    /// Bootstrap spot rate curve from par instruments
    Bootstrap(BootstrapArgs),
    /// Nelson-Siegel yield curve fitting
    NelsonSiegel(NelsonSiegelArgs),
    /// Bond duration & convexity
    Duration(DurationArgs),
    /// Credit spread analysis (Z-spread, OAS, I-spread)
    CreditSpread(CreditSpreadArgs),
    /// Option pricing (Black-Scholes & binomial)
    OptionPrice(OptionPriceArgs),
    /// Implied volatility solver
    ImpliedVol(ImpliedVolArgs),
    /// Forward/futures pricing
    ForwardPrice(ForwardPriceArgs),
    /// Forward position valuation
    ForwardPosition(ForwardPositionArgs),
    /// Futures basis analysis
    BasisAnalysis(BasisAnalysisArgs),
    /// Interest rate swap valuation
    Irs(IrsArgs),
    /// Currency swap valuation
    CurrencySwap(CurrencySwapArgs),
    /// Option strategy analysis
    Strategy(StrategyArgs),
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
        Commands::Lbo(args) => commands::pe::run_lbo(args),
        Commands::Waterfall(args) => commands::pe::run_waterfall(args),
        Commands::Merger(args) => commands::ma::run_merger(args),
        Commands::AltmanZscore(args) => commands::credit::run_altman(args),
        Commands::FundFees(args) => commands::jurisdiction::run_fund_fees(args),
        Commands::GaapIfrs(args) => commands::jurisdiction::run_gaap_ifrs(args),
        Commands::Wht(args) => commands::jurisdiction::run_wht(args),
        Commands::Nav(args) => commands::jurisdiction::run_nav(args),
        Commands::GpEconomics(args) => commands::jurisdiction::run_gp_economics(args),
        Commands::InvestorNetReturns(args) => {
            commands::jurisdiction::run_investor_net_returns(args)
        }
        Commands::UbtiScreening(args) => commands::jurisdiction::run_ubti_screening(args),
        Commands::TradingDay(args) => commands::trading::run_trading_day(args),
        Commands::TradingAnalytics(args) => commands::trading::run_trading_analytics(args),
        Commands::BondPricing(args) => commands::fixed_income::run_bond_pricing(args),
        Commands::BondYield(args) => commands::fixed_income::run_bond_yield(args),
        Commands::Bootstrap(args) => commands::fixed_income::run_bootstrap(args),
        Commands::NelsonSiegel(args) => commands::fixed_income::run_nelson_siegel(args),
        Commands::Duration(args) => commands::fixed_income::run_duration(args),
        Commands::CreditSpread(args) => commands::fixed_income::run_credit_spreads(args),
        Commands::OptionPrice(args) => commands::derivatives::run_option_price(args),
        Commands::ImpliedVol(args) => commands::derivatives::run_implied_vol(args),
        Commands::ForwardPrice(args) => commands::derivatives::run_forward_price(args),
        Commands::ForwardPosition(args) => commands::derivatives::run_forward_position(args),
        Commands::BasisAnalysis(args) => commands::derivatives::run_basis_analysis(args),
        Commands::Irs(args) => commands::derivatives::run_irs(args),
        Commands::CurrencySwap(args) => commands::derivatives::run_currency_swap(args),
        Commands::Strategy(args) => commands::derivatives::run_strategy(args),
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
