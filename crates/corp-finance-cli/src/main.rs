mod commands;
mod input;
mod output;

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::process;

use commands::aml_compliance::{KycRiskArgs, SanctionsScreeningArgs};
use commands::bank_analytics::{
    CamelsRatingArgs, CeclProvisioningArgs, DepositBetaArgs, LoanBookArgs, NimAnalysisArgs,
};
use commands::behavioral::{ProspectTheoryArgs, SentimentArgs};
use commands::capital_allocation::{
    EconomicCapitalArgs, EulerAllocationArgs, LimitManagementArgs, RarocArgs, ShapleyAllocationArgs,
};
use commands::carbon_markets::{
    CarbonPricingArgs, CbamArgs, EtsComplianceArgs, OffsetValuationArgs, ShadowCarbonArgs,
};
use commands::clo_analytics::{
    CloCoverageArgs, CloReinvestmentArgs, CloScenarioArgs, CloTrancheArgs, CloWaterfallArgs,
};
use commands::commodity_trading::{CommoditySpreadArgs, StorageEconomicsArgs};
use commands::compliance::{BestExecutionArgs, GipsReportArgs};
use commands::convertibles::{ConvertibleAnalysisArgs, ConvertiblePricingArgs};
use commands::credit::{AltmanArgs, CovenantArgs, CreditArgs, DebtCapacityArgs};
use commands::credit_derivatives::{CdsArgs, CvaArgs};
use commands::credit_portfolio::{MigrationArgs, PortfolioCreditRiskArgs};
use commands::credit_scoring::{
    CreditScorecardArgs, IntensityModelArgs, MertonPdArgs, PdCalibrationArgs, ScoringValidationArgs,
};
use commands::crypto::{DefiAnalysisArgs, TokenValuationArgs};
use commands::derivatives::{
    BasisAnalysisArgs, CurrencySwapArgs, ForwardPositionArgs, ForwardPriceArgs, ImpliedVolArgs,
    IrsArgs, OptionPriceArgs, StrategyArgs,
};
use commands::dividend_policy::{
    BuybackArgs, HModelDdmArgs, MultistageDdmArgs, PayoutSustainabilityArgs,
    TotalShareholderReturnArgs,
};
use commands::earnings_quality::{
    AccrualQualityArgs, BeneishArgs, EarningsQualityCompositeArgs, PiotroskiArgs,
    RevenueQualityArgs,
};
use commands::equity_research::{SotpArgs, TargetPriceArgs};
use commands::esg::{CarbonFootprintArgs, EsgScoreArgs, GreenBondArgs, SllArgs};
use commands::fatca_crs::{EntityClassificationArgs, FatcaCrsReportingArgs};
use commands::fixed_income::{
    BondPricingArgs, BondYieldArgs, BootstrapArgs, CreditSpreadArgs, DurationArgs, NelsonSiegelArgs,
};
use commands::fpa::{BreakevenArgs, RollingForecastArgs, VarianceArgs, WorkingCapitalArgs};
use commands::fund_of_funds::{
    CommitmentPacingArgs, FofPortfolioArgs, JCurveArgs, ManagerSelectionArgs,
    SecondariesPricingArgs,
};
use commands::fx_commodities::{
    CommodityCurveArgs, CommodityForwardArgs, CrossRateArgs, FxForwardArgs,
};
use commands::inflation_linked::{InflationDerivativeArgs, TipsAnalyticsArgs};
use commands::infrastructure::{ConcessionArgs, PppModelArgs};
use commands::insurance::{CombinedRatioArgs, PremiumPricingArgs, ReservingArgs, ScrArgs};
use commands::interest_rate_models::{ShortRateArgs, TermStructureFitArgs};
use commands::jurisdiction::{
    FundFeesArgs, GaapIfrsArgs, GpEconomicsArgs, InvestorNetReturnsArgs, NavArgs,
    UbtiScreeningArgs, WhtArgs,
};
use commands::lease_accounting::{LeaseClassificationArgs, SaleLeasebackArgs};
use commands::ma::MergerArgs;
use commands::macro_economics::{InternationalArgs, MonetaryPolicyArgs};
use commands::market_microstructure::{OptimalExecutionArgs, SpreadAnalysisArgs};
use commands::monte_carlo::{McDcfArgs, MonteCarloArgs};
use commands::mortgage_analytics::{MbsAnalyticsArgs, PrepaymentArgs};
use commands::municipal::{MuniAnalysisArgs, MuniBondArgs};
use commands::offshore_structures::{CaymanFundArgs, LuxFundArgs};
use commands::onshore_structures::{UkEuFundArgs, UsFundArgs};
use commands::pe::{LboArgs, ReturnsArgs, WaterfallArgs};
use commands::pension::{LdiStrategyArgs, PensionFundingArgs};
use commands::performance_attribution::{BrinsonArgs, FactorAttributionArgs};
use commands::portfolio::{KellyArgs, RiskArgs, SharpeArgs};
use commands::portfolio_optimization::{BlackLittermanPortfolioArgs, MeanVarianceArgs};
use commands::private_credit::{DirectLoanArgs, SyndicationArgs, UnitrancheArgs};
use commands::quant_risk::{BlackLittermanArgs, FactorModelArgs, RiskParityArgs, StressTestArgs};
use commands::quant_strategies::{MomentumArgs, PairsTradingArgs};
use commands::real_assets::{ProjectFinanceArgs, PropertyValuationArgs};
use commands::real_options::{DecisionTreeArgs, RealOptionArgs};
use commands::regulatory::{AlmArgs, LcrArgs, NsfrArgs, RegulatoryCapitalArgs};
use commands::regulatory_reporting::{AifmdReportingArgs, SecCftcReportingArgs};
use commands::repo_financing::{CollateralArgs, RepoAnalyticsArgs};
use commands::restructuring::{DistressedDebtArgs, RecoveryArgs};
use commands::risk_budgeting::{FactorRiskBudgetArgs, TailRiskArgs};
use commands::scenarios::SensitivityArgs;
use commands::securitization::{AbsMbsArgs, TranchingArgs};
use commands::sovereign::{CountryRiskArgs, SovereignBondArgs};
use commands::structured_products::{ExoticProductArgs, StructuredNoteArgs};
use commands::substance_requirements::{EconomicSubstanceArgs, JurisdictionSubstanceTestArgs};
use commands::tax_treaty::{TreatyNetworkArgs, TreatyOptArgs};
use commands::three_statement::ThreeStatementArgs;
use commands::trade_finance::{LetterOfCreditArgs, SupplyChainFinanceArgs};
use commands::transfer_pricing::{BepsArgs, IntercompanyArgs};
use commands::treasury::{CashManagementArgs, HedgingArgs};
use commands::valuation::{CompsArgs, DcfArgs, WaccArgs};
use commands::venture::{
    ConvertibleNoteArgs, DilutionArgs, FundingRoundArgs, SafeArgs, VentureFundArgs,
};
use commands::volatility_surface::{ImpliedVolSurfaceArgs, SabrCalibrationArgs};
use commands::wealth::{EstatePlanArgs, RetirementArgs, TlhArgs};

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
    /// Unitranche pricing (first-out / last-out split)
    Unitranche(UnitrancheArgs),
    /// Direct lending loan model (cash/PIK, delayed draw)
    DirectLoan(DirectLoanArgs),
    /// Loan syndication analysis
    Syndication(SyndicationArgs),
    /// Insurance loss reserve estimation (Chain-Ladder / Bornhuetter-Ferguson)
    Reserving(ReservingArgs),
    /// Insurance premium pricing (frequency x severity)
    PremiumPricing(PremiumPricingArgs),
    /// Insurance combined ratio analysis
    CombinedRatio(CombinedRatioArgs),
    /// Solvency II Standard Formula SCR
    Scr(ScrArgs),
    /// Budget-vs-actual variance analysis (price/volume/mix)
    Variance(VarianceArgs),
    /// Break-even and operating leverage analysis
    Breakeven(BreakevenArgs),
    /// Working capital analysis (DSO, DIO, DPO, CCC)
    WorkingCapital(WorkingCapitalArgs),
    /// Rolling financial forecast
    RollingForecast(RollingForecastArgs),
    /// Retirement planning projection
    Retirement(RetirementArgs),
    /// Tax-loss harvesting simulation
    Tlh(TlhArgs),
    /// Estate planning (gift tax, GST, trust analysis)
    EstatePlan(EstatePlanArgs),
    /// Token/protocol valuation using on-chain metrics
    TokenValuation(TokenValuationArgs),
    /// DeFi yield, impermanent loss, staking & LP analysis
    DefiAnalysis(DefiAnalysisArgs),
    /// Municipal bond pricing with tax-equivalent yield
    MuniBond(MuniBondArgs),
    /// Municipal credit analysis (GO, revenue, scoring, refunding)
    MuniAnalysis(MuniAnalysisArgs),
    /// Structured note pricing (capital-protected, yield enhancement, participation, credit-linked)
    StructuredNote(StructuredNoteArgs),
    /// Exotic product pricing (autocallable, barrier, digital options)
    ExoticProduct(ExoticProductArgs),
    /// Letter of credit pricing and risk assessment
    LetterOfCredit(LetterOfCreditArgs),
    /// Supply chain finance analysis (reverse factoring, dynamic discounting, forfaiting, export credit)
    SupplyChainFinance(SupplyChainFinanceArgs),
    /// Price a single-name credit default swap
    CdsPricing(CdsArgs),
    /// Credit Valuation Adjustment (CVA/DVA)
    CvaCalculation(CvaArgs),
    /// Price a convertible bond (CRR binomial tree)
    ConvertiblePricing(ConvertiblePricingArgs),
    /// Convertible bond scenario analysis
    ConvertibleAnalysis(ConvertibleAnalysisArgs),
    /// ASC 842 / IFRS 16 lease classification and measurement
    LeaseClassification(LeaseClassificationArgs),
    /// Sale-leaseback transaction analysis
    SaleLeaseback(SaleLeasebackArgs),
    /// Pension funding analysis (PBO, ABO, NPPC)
    PensionFunding(PensionFundingArgs),
    /// Liability-Driven Investing (LDI) strategy design
    LdiStrategy(LdiStrategyArgs),
    /// Sovereign bond analysis (yield decomposition, risk premium)
    SovereignBond(SovereignBondArgs),
    /// Country risk assessment (political, economic, financial)
    CountryRisk(CountryRiskArgs),
    /// Real option valuation (Black-Scholes, binomial, Monte Carlo)
    RealOption(RealOptionArgs),
    /// Decision tree analysis for investment decisions
    DecisionTree(DecisionTreeArgs),
    /// Sum-of-the-parts (SOTP) valuation
    Sotp(SotpArgs),
    /// Equity research target price calculation
    TargetPrice(TargetPriceArgs),
    /// Commodity spread analysis (calendar, crack, crush, spark)
    CommoditySpread(CommoditySpreadArgs),
    /// Storage economics analysis (carry trade, injection/withdrawal)
    StorageEconomics(StorageEconomicsArgs),
    /// Pairs trading analysis (cointegration, z-scores, backtest)
    PairsTrading(PairsTradingArgs),
    /// Momentum factor analysis and portfolio construction
    Momentum(MomentumArgs),
    /// Corporate cash management and liquidity analysis
    CashManagement(CashManagementArgs),
    /// Hedge effectiveness analysis (FX, IR hedging)
    HedgeEffectiveness(HedgingArgs),
    /// PPP/PFI project financial model
    PppModel(PppModelArgs),
    /// Concession valuation and analysis
    Concession(ConcessionArgs),
    /// Prospect theory and behavioral bias analysis
    ProspectTheory(ProspectTheoryArgs),
    /// Market sentiment analysis (Fear & Greed scoring)
    Sentiment(SentimentArgs),
    /// Brinson-Fachler performance attribution (allocation, selection, interaction)
    Brinson(BrinsonArgs),
    /// Factor-based return attribution and tracking error decomposition
    FactorAttribution(FactorAttributionArgs),
    /// Portfolio credit risk analysis (Gaussian copula VaR, concentration)
    PortfolioCreditRisk(PortfolioCreditRiskArgs),
    /// Rating migration analysis (transition matrices, mark-to-market VaR)
    CreditMigration(MigrationArgs),
    /// Monetary policy analysis (Taylor Rule, Phillips Curve, Okun's Law)
    MonetaryPolicy(MonetaryPolicyArgs),
    /// International economics (PPP, interest rate parity, balance of payments)
    International(InternationalArgs),
    /// MiFID II best execution and transaction cost analysis
    BestExecution(BestExecutionArgs),
    /// GIPS-compliant performance reporting
    GipsReport(GipsReportArgs),
    /// US onshore fund structure analysis (Delaware LP, REIT, MLP, BDC, QOZ)
    UsFund(UsFundArgs),
    /// UK/EU onshore fund structure analysis (LP, LLP, OEIC, SICAV, FCP, KG)
    UkEuFund(UkEuFundArgs),
    /// Cayman/BVI offshore fund structure analysis (Exempted LP, SPC, BVI BCA)
    CaymanFund(CaymanFundArgs),
    /// Luxembourg/Ireland fund structure analysis (SICAV-SIF, RAIF, SCSp, ICAV, QIAIF)
    LuxFund(LuxFundArgs),
    /// OECD BEPS compliance analysis (CbCR, Pillar Two, functional analysis)
    BepsCompliance(BepsArgs),
    /// Intercompany transfer pricing analysis (CUP, TNMM, Profit Split, CFC)
    Intercompany(IntercompanyArgs),
    /// Tax treaty network analysis (WHT optimization, conduit routing, anti-avoidance)
    TreatyNetwork(TreatyNetworkArgs),
    /// Multi-jurisdiction holding structure optimization (PE risk, substance)
    TreatyOptimization(TreatyOptArgs),
    /// FATCA/CRS reporting compliance analysis
    FatcaCrsReporting(FatcaCrsReportingArgs),
    /// FATCA/CRS entity classification (US person, FI, NFFE)
    EntityClassification(EntityClassificationArgs),
    /// Economic substance analysis (BEPS Action 5, EU ATAD)
    EconomicSubstance(EconomicSubstanceArgs),
    /// Jurisdiction substance test (single or comparative)
    JurisdictionSubstanceTest(JurisdictionSubstanceTestArgs),
    /// AIFMD Annex IV reporting
    AifmdReporting(AifmdReportingArgs),
    /// SEC/CFTC regulatory reporting (Form PF, Form ADV, Form CPO-PQR)
    SecCftcReporting(SecCftcReportingArgs),
    /// KYC risk assessment and scoring
    KycRisk(KycRiskArgs),
    /// Sanctions screening (OFAC, EU, UN)
    SanctionsScreening(SanctionsScreeningArgs),
    /// Build implied volatility surface with interpolation and arbitrage detection
    ImpliedVolSurface(ImpliedVolSurfaceArgs),
    /// SABR stochastic volatility model calibration
    SabrCalibration(SabrCalibrationArgs),
    /// Markowitz mean-variance portfolio optimization
    MeanVarianceOpt(MeanVarianceArgs),
    /// Black-Litterman portfolio optimization with investor views
    BlackLittermanPortfolio(BlackLittermanPortfolioArgs),
    /// Factor-based risk budgeting analysis
    FactorRiskBudget(FactorRiskBudgetArgs),
    /// Tail risk analysis (VaR, CVaR, stress testing)
    TailRisk(TailRiskArgs),
    /// Bid-ask spread decomposition and market quality analysis
    SpreadAnalysis(SpreadAnalysisArgs),
    /// Optimal trade execution (Almgren-Chriss, TWAP, VWAP, IS)
    OptimalExecution(OptimalExecutionArgs),
    /// Short rate models (Vasicek, CIR, Hull-White)
    ShortRate(ShortRateArgs),
    /// Yield curve fitting (Nelson-Siegel, Svensson, Bootstrap)
    TermStructureFit(TermStructureFitArgs),
    /// Mortgage prepayment analysis (PSA, CPR, Refinancing)
    Prepayment(PrepaymentArgs),
    /// MBS pass-through analytics (cash flows, OAS, duration)
    MbsAnalytics(MbsAnalyticsArgs),
    /// TIPS/inflation-linked bond analytics
    TipsAnalytics(TipsAnalyticsArgs),
    /// Inflation derivative pricing (ZCIS, YYIS, Cap/Floor)
    InflationDerivative(InflationDerivativeArgs),
    /// Repo rate and securities lending analytics
    RepoAnalytics(RepoAnalyticsArgs),
    /// Collateral management (haircuts, margin, rehypothecation)
    CollateralAnalytics(CollateralArgs),
    /// Credit scorecard (WoE, IV, Gini, KS)
    CreditScorecard(CreditScorecardArgs),
    /// Merton structural model (PD, distance-to-default, KMV EDF)
    MertonPd(MertonPdArgs),
    /// Reduced-form intensity model (hazard rates, survival)
    IntensityModel(IntensityModelArgs),
    /// PIT/TTC PD calibration (Vasicek single-factor)
    PdCalibration(PdCalibrationArgs),
    /// Credit model validation (AUC-ROC, Brier, Hosmer-Lemeshow)
    ScoringValidation(ScoringValidationArgs),
    /// Economic capital (VaR/ES, IRB, stress buffer)
    EconomicCapital(EconomicCapitalArgs),
    /// RAROC and risk-adjusted pricing
    Raroc(RarocArgs),
    /// Euler risk contribution allocation
    EulerAllocation(EulerAllocationArgs),
    /// Shapley value capital allocation
    ShapleyAllocation(ShapleyAllocationArgs),
    /// Risk limit management and breach detection
    LimitManagement(LimitManagementArgs),
    /// CLO waterfall engine
    CloWaterfall(CloWaterfallArgs),
    /// CLO OC/IC coverage tests
    CloCoverage(CloCoverageArgs),
    /// CLO reinvestment period analytics
    CloReinvestment(CloReinvestmentArgs),
    /// CLO tranche analytics (yield, WAL, breakeven CDR)
    CloTranche(CloTrancheArgs),
    /// CLO scenario analysis (stress testing)
    CloScenario(CloScenarioArgs),
    /// J-Curve fund lifecycle model
    JCurve(JCurveArgs),
    /// Commitment pacing and NAV projection
    CommitmentPacing(CommitmentPacingArgs),
    /// Manager due diligence and selection
    ManagerSelection(ManagerSelectionArgs),
    /// Secondaries pricing and IRR sensitivity
    SecondariesPricing(SecondariesPricingArgs),
    /// Fund of funds portfolio analytics
    FofPortfolio(FofPortfolioArgs),
    /// Beneish M-Score earnings manipulation detection
    Beneish(BeneishArgs),
    /// Piotroski F-Score fundamental strength
    Piotroski(PiotroskiArgs),
    /// Accrual quality analysis (Dechow-Dichev)
    AccrualQuality(AccrualQualityArgs),
    /// Revenue quality analysis (persistence, predictability)
    RevenueQuality(RevenueQualityArgs),
    /// Composite earnings quality score
    EarningsQualityComposite(EarningsQualityCompositeArgs),
    /// Net interest margin analysis
    NimAnalysis(NimAnalysisArgs),
    /// CAMELS bank rating system
    CamelsRating(CamelsRatingArgs),
    /// CECL expected credit loss provisioning
    CeclProvisioning(CeclProvisioningArgs),
    /// Deposit beta and funding cost analysis
    DepositBeta(DepositBetaArgs),
    /// Loan book analysis (concentration, migration, NPL)
    LoanBook(LoanBookArgs),
    /// H-Model dividend discount model
    HModelDdm(HModelDdmArgs),
    /// Multi-stage dividend discount model
    MultistageDdm(MultistageDdmArgs),
    /// Share buyback analysis (accretion, EPS impact)
    Buyback(BuybackArgs),
    /// Payout sustainability analysis
    PayoutSustainability(PayoutSustainabilityArgs),
    /// Total shareholder return decomposition
    TotalShareholderReturn(TotalShareholderReturnArgs),
    /// Carbon pricing analysis (EU ETS, CBAM, internal)
    CarbonPricing(CarbonPricingArgs),
    /// ETS compliance analysis (allowance, hedging, auction)
    EtsCompliance(EtsComplianceArgs),
    /// CBAM carbon border adjustment analysis
    Cbam(CbamArgs),
    /// Carbon offset valuation and portfolio analysis
    OffsetValuation(OffsetValuationArgs),
    /// Shadow carbon price and abatement cost analysis
    ShadowCarbon(ShadowCarbonArgs),
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
        Commands::Unitranche(args) => commands::private_credit::run_unitranche(args),
        Commands::DirectLoan(args) => commands::private_credit::run_direct_loan(args),
        Commands::Syndication(args) => commands::private_credit::run_syndication(args),
        Commands::Reserving(args) => commands::insurance::run_reserving(args),
        Commands::PremiumPricing(args) => commands::insurance::run_premium_pricing(args),
        Commands::CombinedRatio(args) => commands::insurance::run_combined_ratio(args),
        Commands::Scr(args) => commands::insurance::run_scr(args),
        Commands::Variance(args) => commands::fpa::run_variance(args),
        Commands::Breakeven(args) => commands::fpa::run_breakeven(args),
        Commands::WorkingCapital(args) => commands::fpa::run_working_capital(args),
        Commands::RollingForecast(args) => commands::fpa::run_rolling_forecast(args),
        Commands::Retirement(args) => commands::wealth::run_retirement(args),
        Commands::Tlh(args) => commands::wealth::run_tlh(args),
        Commands::EstatePlan(args) => commands::wealth::run_estate_plan(args),
        Commands::TokenValuation(args) => commands::crypto::run_token_valuation(args),
        Commands::DefiAnalysis(args) => commands::crypto::run_defi_analysis(args),
        Commands::MuniBond(args) => commands::municipal::run_muni_bond(args),
        Commands::MuniAnalysis(args) => commands::municipal::run_muni_analysis(args),
        Commands::StructuredNote(args) => commands::structured_products::run_structured_note(args),
        Commands::ExoticProduct(args) => commands::structured_products::run_exotic_product(args),
        Commands::LetterOfCredit(args) => commands::trade_finance::run_letter_of_credit(args),
        Commands::SupplyChainFinance(args) => {
            commands::trade_finance::run_supply_chain_finance(args)
        }
        Commands::CdsPricing(args) => commands::credit_derivatives::run_cds_pricing(args),
        Commands::CvaCalculation(args) => commands::credit_derivatives::run_cva_calculation(args),
        Commands::ConvertiblePricing(args) => commands::convertibles::run_convertible_pricing(args),
        Commands::ConvertibleAnalysis(args) => {
            commands::convertibles::run_convertible_analysis(args)
        }
        Commands::LeaseClassification(args) => {
            commands::lease_accounting::run_lease_classification(args)
        }
        Commands::SaleLeaseback(args) => commands::lease_accounting::run_sale_leaseback(args),
        Commands::PensionFunding(args) => commands::pension::run_pension_funding(args),
        Commands::LdiStrategy(args) => commands::pension::run_ldi_strategy(args),
        Commands::SovereignBond(args) => commands::sovereign::run_sovereign_bond(args),
        Commands::CountryRisk(args) => commands::sovereign::run_country_risk(args),
        Commands::RealOption(args) => commands::real_options::run_real_option(args),
        Commands::DecisionTree(args) => commands::real_options::run_decision_tree(args),
        Commands::Sotp(args) => commands::equity_research::run_sotp(args),
        Commands::TargetPrice(args) => commands::equity_research::run_target_price(args),
        Commands::CommoditySpread(args) => commands::commodity_trading::run_commodity_spread(args),
        Commands::StorageEconomics(args) => {
            commands::commodity_trading::run_storage_economics(args)
        }
        Commands::PairsTrading(args) => commands::quant_strategies::run_pairs_trading(args),
        Commands::Momentum(args) => commands::quant_strategies::run_momentum(args),
        Commands::CashManagement(args) => commands::treasury::run_cash_management(args),
        Commands::HedgeEffectiveness(args) => commands::treasury::run_hedging(args),
        Commands::PppModel(args) => commands::infrastructure::run_ppp_model(args),
        Commands::Concession(args) => commands::infrastructure::run_concession(args),
        Commands::ProspectTheory(args) => commands::behavioral::run_prospect_theory(args),
        Commands::Sentiment(args) => commands::behavioral::run_sentiment(args),
        Commands::Brinson(args) => commands::performance_attribution::run_brinson(args),
        Commands::FactorAttribution(args) => {
            commands::performance_attribution::run_factor_attribution(args)
        }
        Commands::PortfolioCreditRisk(args) => {
            commands::credit_portfolio::run_portfolio_credit_risk(args)
        }
        Commands::CreditMigration(args) => commands::credit_portfolio::run_migration(args),
        Commands::MonetaryPolicy(args) => commands::macro_economics::run_monetary_policy(args),
        Commands::International(args) => commands::macro_economics::run_international(args),
        Commands::BestExecution(args) => commands::compliance::run_best_execution(args),
        Commands::GipsReport(args) => commands::compliance::run_gips_report(args),
        Commands::UsFund(args) => commands::onshore_structures::run_us_fund(args),
        Commands::UkEuFund(args) => commands::onshore_structures::run_uk_eu_fund(args),
        Commands::CaymanFund(args) => commands::offshore_structures::run_cayman_fund(args),
        Commands::LuxFund(args) => commands::offshore_structures::run_lux_fund(args),
        Commands::BepsCompliance(args) => commands::transfer_pricing::run_beps(args),
        Commands::Intercompany(args) => commands::transfer_pricing::run_intercompany(args),
        Commands::TreatyNetwork(args) => commands::tax_treaty::run_treaty_network(args),
        Commands::TreatyOptimization(args) => commands::tax_treaty::run_treaty_optimization(args),
        Commands::FatcaCrsReporting(args) => commands::fatca_crs::run_fatca_crs_reporting(args),
        Commands::EntityClassification(args) => {
            commands::fatca_crs::run_entity_classification(args)
        }
        Commands::EconomicSubstance(args) => {
            commands::substance_requirements::run_economic_substance(args)
        }
        Commands::JurisdictionSubstanceTest(args) => {
            commands::substance_requirements::run_jurisdiction_substance_test(args)
        }
        Commands::AifmdReporting(args) => commands::regulatory_reporting::run_aifmd_reporting(args),
        Commands::SecCftcReporting(args) => {
            commands::regulatory_reporting::run_sec_cftc_reporting(args)
        }
        Commands::KycRisk(args) => commands::aml_compliance::run_kyc_risk(args),
        Commands::SanctionsScreening(args) => {
            commands::aml_compliance::run_sanctions_screening(args)
        }
        Commands::ImpliedVolSurface(args) => {
            commands::volatility_surface::run_implied_vol_surface(args)
        }
        Commands::SabrCalibration(args) => commands::volatility_surface::run_sabr_calibration(args),
        Commands::MeanVarianceOpt(args) => {
            commands::portfolio_optimization::run_mean_variance(args)
        }
        Commands::BlackLittermanPortfolio(args) => {
            commands::portfolio_optimization::run_black_litterman_portfolio(args)
        }
        Commands::FactorRiskBudget(args) => commands::risk_budgeting::run_factor_risk_budget(args),
        Commands::TailRisk(args) => commands::risk_budgeting::run_tail_risk(args),
        Commands::SpreadAnalysis(args) => {
            commands::market_microstructure::run_spread_analysis(args)
        }
        Commands::OptimalExecution(args) => {
            commands::market_microstructure::run_optimal_execution(args)
        }
        Commands::ShortRate(args) => commands::interest_rate_models::run_short_rate(args),
        Commands::TermStructureFit(args) => {
            commands::interest_rate_models::run_term_structure_fit(args)
        }
        Commands::Prepayment(args) => commands::mortgage_analytics::run_prepayment(args),
        Commands::MbsAnalytics(args) => commands::mortgage_analytics::run_mbs_analytics(args),
        Commands::TipsAnalytics(args) => commands::inflation_linked::run_tips_analytics(args),
        Commands::InflationDerivative(args) => {
            commands::inflation_linked::run_inflation_derivatives(args)
        }
        Commands::RepoAnalytics(args) => commands::repo_financing::run_repo_analytics(args),
        Commands::CollateralAnalytics(args) => {
            commands::repo_financing::run_collateral_analytics(args)
        }
        Commands::CreditScorecard(args) => commands::credit_scoring::run_credit_scorecard(args),
        Commands::MertonPd(args) => commands::credit_scoring::run_merton_pd(args),
        Commands::IntensityModel(args) => commands::credit_scoring::run_intensity_model(args),
        Commands::PdCalibration(args) => commands::credit_scoring::run_pd_calibration(args),
        Commands::ScoringValidation(args) => commands::credit_scoring::run_scoring_validation(args),
        Commands::EconomicCapital(args) => commands::capital_allocation::run_economic_capital(args),
        Commands::Raroc(args) => commands::capital_allocation::run_raroc(args),
        Commands::EulerAllocation(args) => commands::capital_allocation::run_euler_allocation(args),
        Commands::ShapleyAllocation(args) => {
            commands::capital_allocation::run_shapley_allocation(args)
        }
        Commands::LimitManagement(args) => commands::capital_allocation::run_limit_management(args),
        Commands::CloWaterfall(args) => commands::clo_analytics::run_clo_waterfall(args),
        Commands::CloCoverage(args) => commands::clo_analytics::run_clo_coverage(args),
        Commands::CloReinvestment(args) => commands::clo_analytics::run_clo_reinvestment(args),
        Commands::CloTranche(args) => commands::clo_analytics::run_clo_tranche(args),
        Commands::CloScenario(args) => commands::clo_analytics::run_clo_scenario(args),
        Commands::JCurve(args) => commands::fund_of_funds::run_j_curve(args),
        Commands::CommitmentPacing(args) => commands::fund_of_funds::run_commitment_pacing(args),
        Commands::ManagerSelection(args) => commands::fund_of_funds::run_manager_selection(args),
        Commands::SecondariesPricing(args) => {
            commands::fund_of_funds::run_secondaries_pricing(args)
        }
        Commands::FofPortfolio(args) => commands::fund_of_funds::run_fof_portfolio(args),
        Commands::Beneish(args) => commands::earnings_quality::run_beneish(args),
        Commands::Piotroski(args) => commands::earnings_quality::run_piotroski(args),
        Commands::AccrualQuality(args) => commands::earnings_quality::run_accrual_quality(args),
        Commands::RevenueQuality(args) => commands::earnings_quality::run_revenue_quality(args),
        Commands::EarningsQualityComposite(args) => {
            commands::earnings_quality::run_earnings_quality_composite(args)
        }
        Commands::NimAnalysis(args) => commands::bank_analytics::run_nim_analysis(args),
        Commands::CamelsRating(args) => commands::bank_analytics::run_camels_rating(args),
        Commands::CeclProvisioning(args) => commands::bank_analytics::run_cecl_provisioning(args),
        Commands::DepositBeta(args) => commands::bank_analytics::run_deposit_beta(args),
        Commands::LoanBook(args) => commands::bank_analytics::run_loan_book(args),
        Commands::HModelDdm(args) => commands::dividend_policy::run_h_model_ddm(args),
        Commands::MultistageDdm(args) => commands::dividend_policy::run_multistage_ddm(args),
        Commands::Buyback(args) => commands::dividend_policy::run_buyback(args),
        Commands::PayoutSustainability(args) => {
            commands::dividend_policy::run_payout_sustainability(args)
        }
        Commands::TotalShareholderReturn(args) => {
            commands::dividend_policy::run_total_shareholder_return(args)
        }
        Commands::CarbonPricing(args) => commands::carbon_markets::run_carbon_pricing(args),
        Commands::EtsCompliance(args) => commands::carbon_markets::run_ets_compliance(args),
        Commands::Cbam(args) => commands::carbon_markets::run_cbam(args),
        Commands::OffsetValuation(args) => commands::carbon_markets::run_offset_valuation(args),
        Commands::ShadowCarbon(args) => commands::carbon_markets::run_shadow_carbon(args),
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
