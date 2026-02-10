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
use commands::esg::{CarbonFootprintArgs, EsgScoreArgs, GreenBondArgs, SllArgs};
use commands::fixed_income::{
    BondPricingArgs, BondYieldArgs, BootstrapArgs, CreditSpreadArgs, DurationArgs, NelsonSiegelArgs,
};
use commands::fx_commodities::{
    CommodityCurveArgs, CommodityForwardArgs, CrossRateArgs, FxForwardArgs,
};
use commands::jurisdiction::{
    FundFeesArgs, GaapIfrsArgs, GpEconomicsArgs, InvestorNetReturnsArgs, NavArgs,
    UbtiScreeningArgs, WhtArgs,
};
use commands::ma::MergerArgs;
use commands::monte_carlo::{McDcfArgs, MonteCarloArgs};
use commands::pe::{LboArgs, ReturnsArgs, WaterfallArgs};
use commands::portfolio::{KellyArgs, RiskArgs, SharpeArgs};
use commands::quant_risk::{BlackLittermanArgs, FactorModelArgs, RiskParityArgs, StressTestArgs};
use commands::real_assets::{ProjectFinanceArgs, PropertyValuationArgs};
use commands::regulatory::{AlmArgs, LcrArgs, NsfrArgs, RegulatoryCapitalArgs};
use commands::restructuring::{DistressedDebtArgs, RecoveryArgs};
use commands::scenarios::SensitivityArgs;
use commands::securitization::{AbsMbsArgs, TranchingArgs};
use commands::three_statement::ThreeStatementArgs;
use commands::valuation::{CompsArgs, DcfArgs, WaccArgs};
use commands::venture::{
    ConvertibleNoteArgs, DilutionArgs, FundingRoundArgs, SafeArgs, VentureFundArgs,
};

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
    /// Build a linked three-statement financial model (IS, BS, CF)
    ThreeStatement(ThreeStatementArgs),
    /// Run a generic Monte Carlo simulation
    MonteCarlo(MonteCarloArgs),
    /// Monte Carlo DCF valuation
    McDcf(McDcfArgs),
    /// Factor model regression (CAPM, Fama-French, Carhart)
    FactorModel(FactorModelArgs),
    /// Black-Litterman portfolio optimisation
    BlackLitterman(BlackLittermanArgs),
    /// Risk-parity portfolio construction
    RiskParity(RiskParityArgs),
    /// Portfolio stress testing across multiple scenarios
    StressTest(StressTestArgs),
    /// Restructuring recovery analysis (APR waterfall)
    Recovery(RecoveryArgs),
    /// Distressed debt analysis and restructuring plan
    DistressedDebt(DistressedDebtArgs),
    /// Property valuation (direct cap, DCF, GRM)
    PropertyValuation(PropertyValuationArgs),
    /// Project finance model (infrastructure / PPP)
    ProjectFinance(ProjectFinanceArgs),
    /// FX forward pricing (covered interest rate parity)
    FxForward(FxForwardArgs),
    /// Cross rate calculation from two currency pairs
    CrossRate(CrossRateArgs),
    /// Commodity forward pricing (cost-of-carry model)
    CommodityForward(CommodityForwardArgs),
    /// Commodity term structure and curve analysis
    CommodityCurve(CommodityCurveArgs),
    /// ABS/MBS cash flow modelling (CPR/PSA/CDR/SDA)
    AbsMbs(AbsMbsArgs),
    /// CDO/CLO tranching and waterfall analysis
    Tranching(TranchingArgs),
    /// VC funding round modelling with option pool shuffle
    FundingRound(FundingRoundArgs),
    /// Multi-round dilution analysis
    Dilution(DilutionArgs),
    /// Convertible note conversion mechanics
    ConvertibleNote(ConvertibleNoteArgs),
    /// SAFE conversion mechanics (pre-money / post-money)
    Safe(SafeArgs),
    /// Venture fund returns modelling (J-curve, DPI, TVPI)
    VentureFund(VentureFundArgs),
    /// ESG scoring with pillar weighting and peer benchmarking
    EsgScore(EsgScoreArgs),
    /// Carbon footprint analysis (Scope 1/2/3)
    CarbonFootprint(CarbonFootprintArgs),
    /// Green bond premium (greenium) analysis
    GreenBond(GreenBondArgs),
    /// Sustainability-linked loan covenant testing
    Sll(SllArgs),
    /// Basel III/IV regulatory capital and RWA
    RegulatoryCapital(RegulatoryCapitalArgs),
    /// Basel III Liquidity Coverage Ratio (LCR)
    Lcr(LcrArgs),
    /// Basel III Net Stable Funding Ratio (NSFR)
    Nsfr(NsfrArgs),
    /// Asset-Liability Management (ALM / IRRBB)
    Alm(AlmArgs),
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
        Commands::ThreeStatement(args) => commands::three_statement::run_three_statement(args),
        Commands::MonteCarlo(args) => commands::monte_carlo::run_monte_carlo(args),
        Commands::McDcf(args) => commands::monte_carlo::run_mc_dcf(args),
        Commands::FactorModel(args) => commands::quant_risk::run_factor_model(args),
        Commands::BlackLitterman(args) => commands::quant_risk::run_black_litterman(args),
        Commands::RiskParity(args) => commands::quant_risk::run_risk_parity(args),
        Commands::StressTest(args) => commands::quant_risk::run_stress_test(args),
        Commands::Recovery(args) => commands::restructuring::run_recovery(args),
        Commands::DistressedDebt(args) => commands::restructuring::run_distressed_debt(args),
        Commands::PropertyValuation(args) => commands::real_assets::run_property_valuation(args),
        Commands::ProjectFinance(args) => commands::real_assets::run_project_finance(args),
        Commands::FxForward(args) => commands::fx_commodities::run_fx_forward(args),
        Commands::CrossRate(args) => commands::fx_commodities::run_cross_rate(args),
        Commands::CommodityForward(args) => commands::fx_commodities::run_commodity_forward(args),
        Commands::CommodityCurve(args) => commands::fx_commodities::run_commodity_curve(args),
        Commands::AbsMbs(args) => commands::securitization::run_abs_mbs(args),
        Commands::Tranching(args) => commands::securitization::run_tranching(args),
        Commands::FundingRound(args) => commands::venture::run_funding_round(args),
        Commands::Dilution(args) => commands::venture::run_dilution(args),
        Commands::ConvertibleNote(args) => commands::venture::run_convertible_note(args),
        Commands::Safe(args) => commands::venture::run_safe(args),
        Commands::VentureFund(args) => commands::venture::run_venture_fund(args),
        Commands::EsgScore(args) => commands::esg::run_esg_score(args),
        Commands::CarbonFootprint(args) => commands::esg::run_carbon_footprint(args),
        Commands::GreenBond(args) => commands::esg::run_green_bond(args),
        Commands::Sll(args) => commands::esg::run_sll(args),
        Commands::RegulatoryCapital(args) => commands::regulatory::run_regulatory_capital(args),
        Commands::Lcr(args) => commands::regulatory::run_lcr(args),
        Commands::Nsfr(args) => commands::regulatory::run_nsfr(args),
        Commands::Alm(args) => commands::regulatory::run_alm(args),
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
