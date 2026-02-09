# Domain-Driven Design Document (DDD)

## Corp Finance MCP Server & CLI

**Product:** corp-finance-mcp  
**Version:** 0.1.0  
**Author:** Robert Fall — Rob-otix AI Ltd  
**Date:** February 2026  
**Status:** Draft

---

## 1. Strategic Design

### 1.1 Domain Overview

The domain is **corporate finance computation** — the quantitative analysis performed by investment bankers, equity research analysts, credit analysts, portfolio managers, and PE/VC professionals. This is distinct from:

- **Market data** (fetching prices, fundamentals) — external concern, not our domain
- **Financial reasoning** (selecting methodology, interpreting results) — LLM concern
- **Reporting** (generating memos, presentations) — presentation concern

Our bounded context is: **given structured financial inputs, produce deterministic, auditable quantitative outputs.**

### 1.2 Ubiquitous Language

These terms have precise meanings within this domain. All code, documentation, APIs, and communication use these definitions consistently.

| Term | Definition | Example |
|------|-----------|---------|
| **Enterprise Value (EV)** | Market cap + net debt + minority interest + preferred equity - associates | EV = £500m |
| **EBITDA** | Earnings before interest, taxes, depreciation, and amortisation | EBITDA = £80m |
| **Free Cash Flow to Firm (FCFF)** | EBIT(1-t) + D&A - Capex - ΔWC | FCFF = £45m |
| **Free Cash Flow to Equity (FCFE)** | FCFF - interest(1-t) - net debt repayment | FCFE = £30m |
| **WACC** | Weighted average cost of capital — blended cost of debt and equity | WACC = 9.5% |
| **Terminal Value** | Value of all cash flows beyond the explicit forecast period | TV = £800m |
| **IRR** | Internal rate of return — discount rate that makes NPV = 0 | IRR = 22% |
| **XIRR** | IRR with irregular cash flow dates | XIRR = 18.5% |
| **MOIC** | Multiple on invested capital — total value / invested capital | MOIC = 2.8x |
| **Net Debt** | Total debt - cash and cash equivalents | Net debt = £200m |
| **Leverage** | Net Debt / EBITDA (or similar ratio) | 2.5x levered |
| **Coverage** | EBITDA / Interest expense (or similar) | 6.0x coverage |
| **Duration** | Weighted average time to receive bond cash flows (modified: price sensitivity to yield) | Duration = 4.2 years |
| **Convexity** | Rate of change of duration — second-order price sensitivity | Convexity = 25.3 |
| **VaR** | Value at Risk — maximum loss at given confidence level over time horizon | 1-day 95% VaR = £2.1m |
| **CVaR** | Conditional VaR / Expected Shortfall — average loss beyond VaR | CVaR = £3.4m |
| **Alpha** | Return above benchmark after adjusting for risk factor exposures | Alpha = 1.2% annualised |
| **Beta** | Sensitivity of asset returns to market returns | β = 1.15 |
| **Tranche** | A slice of debt with specific seniority, rate, and terms | Senior Term Loan A |
| **Hurdle Rate** | Minimum return threshold before carried interest applies | 8% preferred return |
| **Catch-up** | GP receives disproportionate share after hurdle until carry split equalises | 100% catch-up |
| **Waterfall** | Sequential distribution of proceeds through priority tiers | Return of capital → preferred return → catch-up → carried interest |
| **Accretion/Dilution** | Whether a merger increases or decreases acquirer's EPS | 5% accretive |
| **Covenant** | Contractual constraint in a loan agreement | Max leverage 3.5x |
| **Equalisation** | Mechanism to ensure fair performance fee allocation for investors entering at different times | Series accounting, equalisation shares |
| **High-Water Mark (HWM)** | Previous peak NAV per share — performance fees only charged on gains above HWM | HWM = £112.50 |
| **Crystallisation** | Point at which accrued performance fees become payable | Annual crystallisation |
| **UBTI** | Unearned Business Taxable Income — income that triggers tax for US tax-exempt investors | Leveraged real estate income |
| **ECI** | Effectively Connected Income — US-source income taxable to foreign investors | Operating business income |
| **Blocker** | Entity (typically C-corp) interposed to convert UBTI/ECI to non-taxable form | Cayman blocker corporation |
| **WHT** | Withholding tax — tax deducted at source on cross-border payments | 15% US dividend WHT |
| **FATCA** | Foreign Account Tax Compliance Act — US law requiring FFIs to report US person accounts | GIIN registration |
| **CRS** | Common Reporting Standard — OECD multilateral automatic exchange of tax information | Annual CRS filing |
| **CIMA** | Cayman Islands Monetary Authority — financial services regulator | CIMA registered fund |
| **NAV** | Net Asset Value — total assets minus total liabilities | Fund NAV = $500M |
| **DPI** | Distributions to Paid-In — actual cash returned relative to capital called | DPI = 1.4x |
| **RVPI** | Residual Value to Paid-In — unrealised value relative to capital called | RVPI = 0.8x |
| **TVPI** | Total Value to Paid-In — DPI + RVPI | TVPI = 2.2x |
| **Side Pocket** | Segregated portion of fund NAV holding illiquid/hard-to-value assets | Side pocket for distressed position |
| **Gate** | Mechanism limiting redemptions per period to protect remaining investors | 25% quarterly gate |

### 1.3 Bounded Contexts

```
┌─────────────────────────────────────────────────────────┐
│                  CORP FINANCE COMPUTATION                 │
│                  (Our Core Domain)                        │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Valuation   │  │   Credit     │  │   Private    │  │
│  │  Context     │  │   Context    │  │   Equity     │  │
│  │              │  │              │  │   Context    │  │
│  │  DCF         │  │  Metrics     │  │              │  │
│  │  WACC        │  │  Capacity    │  │  LBO         │  │
│  │  Comps       │  │  Covenants   │  │  Returns     │  │
│  │  SOTP        │  │  Rating      │  │  Waterfall   │  │
│  │  DDM         │  │  Altman      │  │  Debt Sched  │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                  │                  │          │
│  ┌──────┴──────────────────┴──────────────────┴───────┐  │
│  │              Shared Kernel                          │  │
│  │  Money, Rate, Multiple, Currency, TimeValue,        │  │
│  │  CashFlowSeries, ProjectionPeriod                   │  │
│  └────────────────────────────────────────────────────┘  │
│         │                  │                  │          │
│  ┌──────┴───────┐  ┌──────┴───────┐  ┌──────┴───────┐  │
│  │  Portfolio    │  │   Fixed      │  │  Scenarios   │  │
│  │  Context     │  │   Income     │  │  Context     │  │
│  │              │  │   Context    │  │              │  │
│  │  Risk        │  │              │  │  Sensitivity │  │
│  │  Attribution │  │  Bonds       │  │  Monte Carlo │  │
│  │  Returns     │  │  Yield Curve │  │  Stress      │  │
│  │  Sizing      │  │  Spreads     │  │  Scenarios   │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │           Three-Statement Model Context             │  │
│  │  Income Statement, Balance Sheet, Cash Flow,        │  │
│  │  Circular Reference Solver                          │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │           Jurisdiction & Fund Context               │  │
│  │  GAAP/IFRS Reconciliation, WHT Calculator,          │  │
│  │  NAV with Equalisation, Fund Fee Calculator,        │  │
│  │  GP Economics, Investor Net Returns                 │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘

┌──────────────────┐  ┌──────────────────┐
│  MCP Interface   │  │  CLI Interface   │
│  (Anti-Corruption│  │  (Anti-Corruption│
│   Layer)         │  │   Layer)         │
│                  │  │                  │
│  Zod validation  │  │  clap parsing    │
│  JSON marshall   │  │  Output format   │
│  MCP protocol    │  │  Pipe support    │
└──────────────────┘  └──────────────────┘
```

### 1.4 Context Map

```
Valuation ←──uses──→ Credit       (WACC needs credit spread; credit needs EV for leverage)
Valuation ←──uses──→ Scenarios    (DCF feeds sensitivity; sensitivity varies DCF inputs)
PE        ←──uses──→ Valuation    (LBO entry uses EV; exit uses multiple)
PE        ←──uses──→ Credit       (LBO debt sizing uses coverage constraints)
MA        ←──uses──→ Valuation    (Merger uses DCF for synergy value)
Portfolio ←──uses──→ Scenarios    (Risk metrics use Monte Carlo)
Fixed Inc ←──uses──→ Scenarios    (Duration hedging uses sensitivity)
3-Stmt    ←──uses──→ Valuation    (Projections feed DCF)
3-Stmt    ←──uses──→ Credit       (Projected ratios feed covenant testing)
Jurisd    ←──uses──→ Valuation    (GAAP/IFRS adjustments affect inputs)
Jurisd    ←──uses──→ PE           (Waterfall variants, fund fee calculations)
Jurisd    ←──uses──→ Portfolio    (WHT-adjusted returns for cross-border)
Jurisd    ←──uses──→ Credit       (Rating agency adjustments differ by standard)
```

**Relationships are Shared Kernel** — contexts share types through the common `types.rs` module, not through published events or APIs. This is appropriate because all contexts live in the same crate and are maintained by the same team.

---

## 2. Tactical Design

### 2.1 Shared Kernel — Core Value Objects

Value objects are immutable, compared by value, and carry no identity.

```rust
// src/types.rs

use rust_decimal::Decimal;
use chrono::NaiveDate;
use serde::{Serialize, Deserialize};

/// All monetary values. Wraps Decimal to prevent accidental f64 usage.
pub type Money = Decimal;

/// Rates expressed as decimals (0.05 = 5%). Never as percentages.
pub type Rate = Decimal;

/// Multiples (e.g., 8.5x EV/EBITDA)
pub type Multiple = Decimal;

/// Year fractions or counts
pub type Years = Decimal;

/// Currency code
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    GBP, USD, EUR, CHF, JPY, CAD, AUD, HKD, SGD,
    #[serde(other)]
    Other(String),
}

/// A single cash flow at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlow {
    pub date: NaiveDate,
    pub amount: Money,
    pub label: Option<String>,
}

/// A series of cash flows — the fundamental building block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowSeries {
    pub flows: Vec<CashFlow>,
    pub currency: Currency,
}

/// A single period in a financial projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionPeriod {
    pub year: i32,
    pub label: String,           // "FY2025", "Year 3", etc.
    pub is_terminal: bool,
}

/// Sensitivity variable specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityVariable {
    pub name: String,
    pub min: Decimal,
    pub max: Decimal,
    pub step: Decimal,
}

/// Scenario definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,            // "Bear", "Base", "Bull"
    pub probability: Rate,       // Must sum to 1.0 across scenarios
    pub overrides: serde_json::Value, // Parameter overrides for this scenario
}
```

### 2.2 Valuation Context

#### Aggregates

**DCF Model** — the primary aggregate. Orchestrates WACC, projections, and terminal value.

```rust
// src/valuation/dcf.rs

/// DCF input — everything needed to build a discounted cash flow model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcfInput {
    // Revenue build
    pub base_revenue: Money,
    pub revenue_growth_rates: Vec<Rate>,     // Per year of explicit forecast
    
    // Margins
    pub ebitda_margin: Rate,                 // Or provide absolute EBITDA
    pub ebit_margin: Option<Rate>,           // If D&A needs separate treatment
    pub da_as_pct_revenue: Option<Rate>,
    
    // Capital requirements
    pub capex_as_pct_revenue: Rate,
    pub nwc_as_pct_revenue: Rate,            // Net working capital
    pub tax_rate: Rate,
    
    // Discount rate
    pub wacc: Rate,                          // Or provide WaccInput to calculate
    pub wacc_input: Option<WaccInput>,
    
    // Terminal value
    pub terminal_method: TerminalMethod,
    pub terminal_growth_rate: Option<Rate>,  // For Gordon Growth
    pub terminal_exit_multiple: Option<Multiple>, // For exit multiple
    
    // Optional
    pub currency: Currency,
    pub forecast_years: u32,                 // Default 10
    pub mid_year_convention: bool,           // Default true
    
    // Existing claims
    pub net_debt: Option<Money>,
    pub minority_interest: Option<Money>,
    pub shares_outstanding: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalMethod {
    GordonGrowth,
    ExitMultiple,
    Both,  // Calculate both, report both
}

/// DCF output — full model with all intermediate values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcfOutput {
    pub projections: Vec<DcfYearProjection>,
    
    pub terminal_value_gordon: Option<Money>,
    pub terminal_value_exit: Option<Money>,
    pub terminal_value_used: Money,
    
    pub pv_of_fcff: Money,
    pub pv_of_terminal: Money,
    pub enterprise_value: Money,
    
    pub equity_value: Option<Money>,        // EV - net debt - minority
    pub equity_value_per_share: Option<Money>,
    
    pub implied_exit_multiple: Multiple,     // EV / terminal year EBITDA
    pub terminal_value_pct: Rate,            // TV as % of total EV
    
    pub wacc_used: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcfYearProjection {
    pub period: ProjectionPeriod,
    pub revenue: Money,
    pub ebitda: Money,
    pub ebit: Money,
    pub nopat: Money,                        // EBIT × (1 - tax)
    pub plus_da: Money,
    pub less_capex: Money,
    pub less_nwc_change: Money,
    pub fcff: Money,
    pub discount_factor: Rate,
    pub pv_fcff: Money,
}
```

**WACC** — value object, often computed inline by DCF.

```rust
// src/valuation/wacc.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaccInput {
    pub risk_free_rate: Rate,
    pub equity_risk_premium: Rate,
    pub beta: Decimal,                       // Levered beta
    pub cost_of_debt: Rate,                  // Pre-tax
    pub tax_rate: Rate,
    pub debt_weight: Rate,                   // D / (D+E)
    pub equity_weight: Rate,                 // E / (D+E)
    
    // Optional adjustments
    pub size_premium: Option<Rate>,
    pub country_risk_premium: Option<Rate>,
    pub specific_risk_premium: Option<Rate>,
    
    // Beta adjustment
    pub unlevered_beta: Option<Decimal>,     // If provided, re-lever to target structure
    pub target_debt_equity: Option<Decimal>, // For re-levering
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaccOutput {
    pub wacc: Rate,
    pub cost_of_equity: Rate,
    pub after_tax_cost_of_debt: Rate,
    pub cost_of_debt_pretax: Rate,
    pub levered_beta: Decimal,
    pub unlevered_beta: Option<Decimal>,
}
```

**Comparables Analysis** — value object.

```rust
// src/valuation/comps.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompsInput {
    pub target: CompanyMetrics,
    pub comparables: Vec<ComparableCompany>,
    pub multiples: Vec<MultipleType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparableCompany {
    pub name: String,
    pub metrics: CompanyMetrics,
    pub include: bool,                       // For outlier exclusion
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyMetrics {
    pub enterprise_value: Option<Money>,
    pub market_cap: Option<Money>,
    pub revenue: Option<Money>,
    pub ebitda: Option<Money>,
    pub ebit: Option<Money>,
    pub net_income: Option<Money>,
    pub book_value: Option<Money>,
    pub earnings_growth: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultipleType {
    EvEbitda,
    EvRevenue,
    EvEbit,
    PriceEarnings,
    PriceBook,
    PegRatio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompsOutput {
    pub multiples_table: Vec<ComparableMultiples>,
    pub statistics: Vec<MultipleStatistics>,
    pub implied_valuations: Vec<ImpliedValuation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipleStatistics {
    pub multiple_type: MultipleType,
    pub mean: Multiple,
    pub median: Multiple,
    pub high: Multiple,
    pub low: Multiple,
    pub std_dev: Multiple,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedValuation {
    pub multiple_type: MultipleType,
    pub at_mean: Money,
    pub at_median: Money,
    pub at_low: Money,
    pub at_high: Money,
}
```

### 2.3 Credit Context

```rust
// src/credit/metrics.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditMetricsInput {
    // Income statement
    pub revenue: Money,
    pub ebitda: Money,
    pub ebit: Money,
    pub interest_expense: Money,
    pub depreciation_amortisation: Money,
    
    // Balance sheet
    pub total_debt: Money,
    pub cash: Money,
    pub total_assets: Money,
    pub current_assets: Money,
    pub current_liabilities: Money,
    pub total_equity: Money,
    pub retained_earnings: Money,
    pub working_capital: Money,
    
    // Cash flow
    pub operating_cash_flow: Money,
    pub capex: Money,
    pub funds_from_operations: Option<Money>, // FFO if available
    
    // Optional
    pub lease_payments: Option<Money>,        // For fixed charge coverage
    pub preferred_dividends: Option<Money>,
    pub market_cap: Option<Money>,            // For Altman Z-score market variant
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditMetricsOutput {
    // Leverage
    pub net_debt: Money,
    pub net_debt_to_ebitda: Multiple,
    pub total_debt_to_ebitda: Multiple,
    pub debt_to_equity: Multiple,
    pub debt_to_assets: Rate,
    pub net_debt_to_ev: Option<Rate>,
    
    // Coverage
    pub interest_coverage: Multiple,          // EBITDA / Interest
    pub ebit_coverage: Multiple,              // EBIT / Interest
    pub fixed_charge_coverage: Option<Multiple>,
    pub dscr: Multiple,                       // Debt service coverage ratio
    
    // Cash flow
    pub ffo_to_debt: Option<Rate>,
    pub ocf_to_debt: Rate,
    pub fcf_to_debt: Rate,
    pub fcf: Money,
    pub cash_conversion: Rate,                // OCF / EBITDA
    
    // Liquidity
    pub current_ratio: Multiple,
    pub quick_ratio: Multiple,
    pub cash_to_debt: Rate,
    
    // Synthetic rating
    pub implied_rating: CreditRating,
    pub rating_rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CreditRating {
    AAA, AAm, AAp, AA, Am, Ap, A,
    BBBp, BBB, BBBm,
    BBp, BB, BBm,
    Bp, B, Bm,
    CCCp, CCC, CCCm,
    CC, C, D,
}
```

```rust
// src/credit/capacity.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtCapacityInput {
    pub ebitda: Money,
    pub interest_rate: Rate,
    
    // Constraints (any combination)
    pub max_leverage: Option<Multiple>,        // e.g., 4.0x
    pub min_interest_coverage: Option<Multiple>, // e.g., 3.0x
    pub min_dscr: Option<Multiple>,            // e.g., 1.5x
    pub min_ffo_to_debt: Option<Rate>,         // e.g., 15%
    
    // Optional
    pub existing_debt: Option<Money>,
    pub annual_amortisation: Option<Money>,
    pub ffo: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtCapacityOutput {
    pub max_debt_by_leverage: Option<Money>,
    pub max_debt_by_coverage: Option<Money>,
    pub max_debt_by_dscr: Option<Money>,
    pub max_debt_by_ffo: Option<Money>,
    pub binding_constraint: String,
    pub max_incremental_debt: Money,
    pub implied_leverage_at_max: Multiple,
    pub implied_coverage_at_max: Multiple,
}
```

```rust
// src/credit/covenants.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantTestInput {
    pub covenants: Vec<Covenant>,
    pub actuals: CreditMetricsOutput,        // Output from credit metrics
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Covenant {
    pub name: String,
    pub metric: CovenantMetric,
    pub threshold: Decimal,
    pub direction: CovenantDirection,         // MaxOf or MinOf
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CovenantMetric {
    NetDebtToEbitda,
    InterestCoverage,
    Dscr,
    DebtToEquity,
    MinCash,
    MaxCapex,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CovenantDirection {
    MaxOf,    // Actual must be ≤ threshold (e.g., max leverage 3.5x)
    MinOf,    // Actual must be ≥ threshold (e.g., min coverage 3.0x)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantTestOutput {
    pub results: Vec<CovenantResult>,
    pub all_passing: bool,
    pub headroom_summary: Vec<CovenantHeadroom>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantResult {
    pub covenant: String,
    pub threshold: Decimal,
    pub actual: Decimal,
    pub passing: bool,
    pub headroom: Decimal,                   // How much room before breach
    pub headroom_pct: Rate,                  // As percentage of threshold
}
```

### 2.4 Private Equity Context

```rust
// src/pe/lbo.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LboInput {
    // Entry
    pub entry_ev: Money,
    pub entry_multiple: Multiple,            // Or derive from EV / EBITDA
    pub entry_ebitda: Money,
    
    // Operating projections
    pub revenue_growth: Vec<Rate>,
    pub ebitda_margin: Vec<Rate>,            // Or absolute EBITDA per year
    pub capex_as_pct_revenue: Rate,
    pub nwc_as_pct_revenue: Rate,
    pub tax_rate: Rate,
    pub da_as_pct_revenue: Rate,
    
    // Debt structure
    pub tranches: Vec<DebtTrancheInput>,
    pub equity_contribution: Money,
    pub cash_sweep_pct: Option<Rate>,        // % of excess cash to mandatory repayment
    
    // Exit
    pub exit_year: u32,
    pub exit_multiple: Multiple,
    
    // Optional
    pub transaction_fees: Option<Money>,
    pub financing_fees: Option<Money>,
    pub management_rollover: Option<Money>,
    pub currency: Currency,
    
    // Waterfall (optional)
    pub waterfall: Option<WaterfallInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtTrancheInput {
    pub name: String,                        // "Senior Term Loan A"
    pub amount: Money,
    pub interest_rate: Rate,
    pub is_floating: bool,
    pub base_rate: Option<Rate>,             // SOFR/SONIA if floating
    pub spread: Option<Rate>,
    pub amortisation: AmortisationType,
    pub maturity_years: u32,
    pub pik_rate: Option<Rate>,              // Payment-in-kind interest
    pub seniority: u32,                      // 1 = most senior
    pub commitment_fee: Option<Rate>,        // For revolvers
    pub is_revolver: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AmortisationType {
    Bullet,                                  // Full repayment at maturity
    StraightLine(Rate),                      // Fixed % per year
    Custom(Vec<Money>),                      // Specific amounts per year
    CashSweep(Rate),                         // % of excess cash flow
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LboOutput {
    // Year-by-year projections
    pub projections: Vec<LboYearProjection>,
    
    // Debt schedule per tranche
    pub debt_schedules: Vec<DebtScheduleOutput>,
    
    // Sources & Uses
    pub sources_uses: SourcesUsesOutput,
    
    // Exit
    pub exit_ev: Money,
    pub exit_equity_value: Money,
    pub exit_net_debt: Money,
    
    // Returns
    pub irr: Rate,
    pub moic: Multiple,
    pub cash_on_cash: Multiple,
    
    // Credit metrics at entry and each year
    pub credit_profile: Vec<CreditMetricsOutput>,
    
    // Waterfall (if specified)
    pub waterfall: Option<WaterfallOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LboYearProjection {
    pub period: ProjectionPeriod,
    pub revenue: Money,
    pub ebitda: Money,
    pub ebit: Money,
    pub total_interest: Money,
    pub ebt: Money,
    pub tax: Money,
    pub net_income: Money,
    pub fcf_before_debt: Money,
    pub mandatory_repayment: Money,
    pub optional_repayment: Money,
    pub total_debt: Money,
    pub net_debt: Money,
    pub cash_balance: Money,
    pub equity_value: Money,
}
```

```rust
// src/pe/waterfall.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallInput {
    pub total_proceeds: Money,
    pub total_invested: Money,
    pub tiers: Vec<WaterfallTier>,
    pub gp_commitment_pct: Rate,             // GP's share of fund (typically 1-5%)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallTier {
    pub name: String,                        // "Return of Capital", "Preferred Return", etc.
    pub tier_type: WaterfallTierType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaterfallTierType {
    ReturnOfCapital,
    PreferredReturn { rate: Rate },
    CatchUp { gp_share: Rate },              // GP catch-up (typically 100% to GP)
    CarriedInterest { gp_share: Rate },      // Ongoing split (typically 80/20)
    Residual { gp_share: Rate },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallOutput {
    pub tiers: Vec<WaterfallTierResult>,
    pub total_to_gp: Money,
    pub total_to_lp: Money,
    pub gp_pct_of_total: Rate,
    pub lp_pct_of_total: Rate,
    pub lp_net_irr: Rate,
    pub lp_net_moic: Multiple,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallTierResult {
    pub tier_name: String,
    pub amount: Money,
    pub to_gp: Money,
    pub to_lp: Money,
    pub remaining: Money,                    // Proceeds remaining after this tier
}
```

### 2.5 Portfolio Context

```rust
// src/portfolio/risk.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetricsInput {
    pub returns: Vec<Decimal>,               // Period returns (daily, weekly, monthly)
    pub frequency: ReturnFrequency,
    pub confidence_level: Rate,              // 0.95 or 0.99 for VaR
    pub benchmark_returns: Option<Vec<Decimal>>,
    pub risk_free_rate: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReturnFrequency {
    Daily, Weekly, Monthly, Quarterly, Annual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetricsOutput {
    // Return statistics
    pub annualised_return: Rate,
    pub annualised_volatility: Rate,
    pub skewness: Decimal,
    pub kurtosis: Decimal,
    
    // Risk metrics
    pub var_parametric: Money,
    pub var_historical: Money,
    pub cvar: Money,
    pub max_drawdown: Rate,
    pub max_drawdown_duration_periods: u32,
    pub downside_deviation: Rate,
    
    // Risk-adjusted
    pub sharpe_ratio: Decimal,
    pub sortino_ratio: Decimal,
    pub calmar_ratio: Decimal,
    pub information_ratio: Option<Decimal>,
    pub treynor_ratio: Option<Decimal>,
    
    // Relative (if benchmark provided)
    pub tracking_error: Option<Rate>,
    pub beta: Option<Decimal>,
    pub alpha: Option<Rate>,
    pub upside_capture: Option<Rate>,
    pub downside_capture: Option<Rate>,
}
```

```rust
// src/portfolio/sizing.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyInput {
    pub win_probability: Rate,
    pub win_loss_ratio: Decimal,             // Average win / average loss
    pub kelly_fraction: Rate,                // Fractional Kelly (0.25 - 0.50 typical)
    pub portfolio_value: Option<Money>,
    pub max_position_pct: Option<Rate>,      // Hard cap
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyOutput {
    pub full_kelly_pct: Rate,
    pub fractional_kelly_pct: Rate,
    pub recommended_position: Option<Money>,
    pub edge: Rate,                          // Expected value per unit risked
    pub growth_rate: Rate,                   // Expected log growth rate
}
```

### 2.6 Fixed Income Context

```rust
// src/fixed_income/bond.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondInput {
    pub face_value: Money,
    pub coupon_rate: Rate,
    pub coupon_frequency: u32,               // 1 = annual, 2 = semi-annual
    pub maturity_date: NaiveDate,
    pub settlement_date: NaiveDate,
    pub price: Option<Money>,                // If provided, calculate yield
    pub yield_to_maturity: Option<Rate>,     // If provided, calculate price
    pub call_schedule: Option<Vec<CallDate>>,
    pub day_count: DayCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DayCount {
    Actual360,
    Actual365,
    ActualActual,
    ThirtyThreeSixty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallDate {
    pub date: NaiveDate,
    pub call_price: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondOutput {
    pub clean_price: Money,
    pub dirty_price: Money,
    pub accrued_interest: Money,
    pub yield_to_maturity: Rate,
    pub yield_to_worst: Option<Rate>,
    pub current_yield: Rate,
    pub modified_duration: Decimal,
    pub macaulay_duration: Decimal,
    pub convexity: Decimal,
    pub dv01: Money,                         // Dollar value of 1bp
    pub remaining_life: Years,
    pub cash_flows: Vec<BondCashFlow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondCashFlow {
    pub date: NaiveDate,
    pub coupon: Money,
    pub principal: Money,
    pub total: Money,
    pub pv: Money,
}
```

### 2.7 Scenarios Context

```rust
// src/scenarios/sensitivity.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityInput {
    pub model: SensitivityModel,
    pub variable_1: SensitivityVariable,
    pub variable_2: SensitivityVariable,
    pub base_inputs: serde_json::Value,      // Full model inputs as baseline
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensitivityModel {
    Dcf,
    Lbo,
    Bond,
    CreditMetrics,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityOutput {
    pub variable_1_name: String,
    pub variable_2_name: String,
    pub variable_1_values: Vec<Decimal>,
    pub variable_2_values: Vec<Decimal>,
    pub output_metric: String,               // e.g., "enterprise_value", "irr"
    pub matrix: Vec<Vec<Decimal>>,           // [row][col] = output at (var1, var2)
    pub base_case_value: Decimal,
    pub base_case_position: (usize, usize),  // Row, col of base case in matrix
}
```

```rust
// src/scenarios/monte_carlo.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloInput {
    pub model: SensitivityModel,
    pub base_inputs: serde_json::Value,
    pub variables: Vec<MonteCarloVariable>,
    pub num_simulations: u32,                // Default 10,000
    pub seed: Option<u64>,                   // For reproducibility
    pub correlation_matrix: Option<Vec<Vec<f64>>>, // Between variables
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloVariable {
    pub name: String,
    pub distribution: Distribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Distribution {
    Normal { mean: f64, std_dev: f64 },
    LogNormal { mean: f64, std_dev: f64 },
    Uniform { min: f64, max: f64 },
    Triangular { min: f64, mode: f64, max: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloOutput {
    pub output_metric: String,
    pub num_simulations: u32,
    pub mean: Decimal,
    pub median: Decimal,
    pub std_dev: Decimal,
    pub percentiles: MonteCarloPercentiles,
    pub probability_below_zero: Rate,
    pub probability_above_target: Option<Rate>,
    pub histogram_bins: Vec<HistogramBin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloPercentiles {
    pub p5: Decimal,
    pub p10: Decimal,
    pub p25: Decimal,
    pub p50: Decimal,
    pub p75: Decimal,
    pub p90: Decimal,
    pub p95: Decimal,
}
```

### 2.8 Three-Statement Model Context

```rust
// src/three_statement/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeStatementInput {
    // Historical (for trend analysis)
    pub historical_years: Vec<HistoricalYear>,
    
    // Projection assumptions
    pub projection_years: u32,
    pub revenue_growth: Vec<Rate>,
    pub cogs_as_pct_revenue: Rate,
    pub sga_as_pct_revenue: Rate,
    pub da_as_pct_revenue: Rate,
    pub interest_rate_on_debt: Rate,
    pub interest_income_rate: Rate,
    pub tax_rate: Rate,
    pub capex_as_pct_revenue: Rate,
    pub dividend_payout_ratio: Option<Rate>,
    
    // Working capital drivers
    pub dso: Decimal,                        // Days sales outstanding
    pub dio: Decimal,                        // Days inventory outstanding
    pub dpo: Decimal,                        // Days payable outstanding
    
    // Debt assumptions
    pub debt_schedule: Option<Vec<DebtTrancheInput>>,
    pub target_min_cash: Option<Money>,
    pub revolver_available: Option<Money>,
    
    // Opening balance sheet
    pub opening_balance: BalanceSheet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeStatementOutput {
    pub income_statements: Vec<IncomeStatement>,
    pub balance_sheets: Vec<BalanceSheet>,
    pub cash_flow_statements: Vec<CashFlowStatement>,
    pub key_metrics: Vec<PeriodMetrics>,
    pub solver_iterations: Option<u32>,      // If circular ref needed iteration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub period: ProjectionPeriod,
    pub revenue: Money,
    pub cogs: Money,
    pub gross_profit: Money,
    pub gross_margin: Rate,
    pub sga: Money,
    pub ebitda: Money,
    pub ebitda_margin: Rate,
    pub depreciation: Money,
    pub amortisation: Money,
    pub ebit: Money,
    pub interest_expense: Money,
    pub interest_income: Money,
    pub ebt: Money,
    pub tax: Money,
    pub net_income: Money,
    pub net_margin: Rate,
    pub eps: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub period: ProjectionPeriod,
    // Assets
    pub cash: Money,
    pub accounts_receivable: Money,
    pub inventory: Money,
    pub other_current_assets: Money,
    pub total_current_assets: Money,
    pub ppe_net: Money,
    pub intangibles: Money,
    pub other_non_current: Money,
    pub total_assets: Money,
    // Liabilities
    pub accounts_payable: Money,
    pub accrued_liabilities: Money,
    pub current_debt: Money,
    pub total_current_liabilities: Money,
    pub long_term_debt: Money,
    pub other_non_current_liabilities: Money,
    pub total_liabilities: Money,
    // Equity
    pub share_capital: Money,
    pub retained_earnings: Money,
    pub total_equity: Money,
    // Check
    pub balance_check: Money,                // Should be zero
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowStatement {
    pub period: ProjectionPeriod,
    // Operating
    pub net_income: Money,
    pub depreciation: Money,
    pub amortisation: Money,
    pub change_receivables: Money,
    pub change_inventory: Money,
    pub change_payables: Money,
    pub other_operating: Money,
    pub cash_from_operations: Money,
    // Investing
    pub capex: Money,
    pub acquisitions: Money,
    pub other_investing: Money,
    pub cash_from_investing: Money,
    // Financing
    pub debt_issued: Money,
    pub debt_repaid: Money,
    pub dividends: Money,
    pub share_repurchase: Money,
    pub other_financing: Money,
    pub cash_from_financing: Money,
    // Net
    pub net_change_in_cash: Money,
    pub opening_cash: Money,
    pub closing_cash: Money,
}
```

---

## 2.8 Jurisdiction & Fund Context

#### GAAP/IFRS Reconciliation

```rust
// src/jurisdiction/reconciliation.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountingStandard {
    UsGaap,
    Ifrs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationInput {
    pub source_standard: AccountingStandard,
    pub target_standard: AccountingStandard,
    
    // Core financials
    pub revenue: Money,
    pub ebitda: Money,
    pub ebit: Money,
    pub net_income: Money,
    pub total_assets: Money,
    pub total_debt: Money,
    pub total_equity: Money,
    pub inventory: Money,
    pub ppe_net: Money,
    
    // Adjustment inputs (provide what's available)
    pub operating_lease_payments: Option<Money>,
    pub operating_lease_remaining_years: Option<u32>,
    pub lifo_reserve: Option<Money>,
    pub capitalised_dev_costs: Option<Money>,
    pub dev_cost_amortisation: Option<Money>,
    pub revaluation_surplus: Option<Money>,
    pub discount_rate_for_leases: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationOutput {
    pub adjusted_ebitda: Money,
    pub adjusted_ebit: Money,
    pub adjusted_total_debt: Money,
    pub adjusted_total_equity: Money,
    pub adjusted_total_assets: Money,
    pub adjustments: Vec<ReconciliationAdjustment>,
    pub materiality_flag: bool,              // True if adjustments > 2% of EV
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationAdjustment {
    pub name: String,
    pub category: AdjustmentCategory,
    pub impact_ebitda: Money,
    pub impact_debt: Money,
    pub impact_assets: Money,
    pub impact_equity: Money,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdjustmentCategory {
    LeaseCapitalisation,
    LifoAdjustment,
    DevelopmentCosts,
    RevaluationStrip,
    ContingencyRecognition,
    PensionNormalisation,
    OtherGaapDifference,
}
```

#### Withholding Tax Calculator

```rust
// src/jurisdiction/withholding_tax.rs

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Jurisdiction {
    US, UK, Cayman, Ireland, Luxembourg, Jersey, Guernsey, BVI,
    Germany, France, Netherlands, Switzerland, Singapore, HongKong,
    Japan, Australia, Canada,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncomeType {
    Dividend, Interest, Royalty, RentalIncome, CapitalGain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhtInput {
    pub source_jurisdiction: Jurisdiction,
    pub investor_jurisdiction: Jurisdiction,
    pub fund_jurisdiction: Option<Jurisdiction>,
    pub income_type: IncomeType,
    pub gross_income: Money,
    pub is_tax_exempt_investor: bool,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhtOutput {
    pub statutory_rate: Rate,
    pub treaty_rate: Option<Rate>,
    pub effective_rate: Rate,
    pub withholding_amount: Money,
    pub net_income: Money,
    pub treaty_name: Option<String>,
    pub notes: Vec<String>,
    pub blocker_recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioWhtOutput {
    pub total_gross_income: Money,
    pub total_wht: Money,
    pub total_net_income: Money,
    pub effective_wht_rate: Rate,
    pub wht_drag_on_return: Rate,
    pub per_holding: Vec<WhtOutput>,
    pub optimisation_suggestions: Vec<String>,
}
```

#### NAV Calculator with Equalisation

```rust
// src/jurisdiction/nav.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EqualisationMethod {
    EqualisationShares,
    SeriesAccounting,
    DepreciationDeposit,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareClassInput {
    pub class_name: String,
    pub currency: Currency,
    pub shares_outstanding: Decimal,
    pub nav_per_share_opening: Money,
    pub high_water_mark: Money,
    pub management_fee_rate: Rate,
    pub performance_fee_rate: Rate,
    pub hurdle_rate: Option<Rate>,
    pub crystallisation_frequency: CrystallisationFrequency,
    pub fx_rate_to_base: Option<Decimal>,
    pub fx_hedging_cost: Option<Rate>,
    pub subscriptions: Vec<Subscription>,
    pub redemptions: Vec<Redemption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrystallisationFrequency {
    Monthly, Quarterly, SemiAnnually, Annually, OnRedemption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareClassNavOutput {
    pub class_name: String,
    pub currency: Currency,
    pub gross_nav_per_share: Money,
    pub management_fee_accrual: Money,
    pub performance_fee_accrual: Money,
    pub net_nav_per_share: Money,
    pub high_water_mark: Money,
    pub hwm_distance: Rate,
    pub shares_outstanding: Decimal,
    pub class_total_nav: Money,
    pub gross_return: Rate,
    pub net_return: Rate,
}
```

#### Fund Fee Calculator

```rust
// src/jurisdiction/fund_fees.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ManagementFeeBasis {
    CommittedCapital,
    InvestedCapital,
    NetAssetValue,
    GrossAssetValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaterfallType {
    European,        // Whole-fund carry
    American,        // Deal-by-deal carry
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundFeeInput {
    pub fund_size: Money,
    pub management_fee_rate: Rate,
    pub management_fee_basis: ManagementFeeBasis,
    pub performance_fee_rate: Rate,
    pub hurdle_rate: Rate,
    pub catch_up_rate: Rate,
    pub waterfall_type: WaterfallType,
    pub gp_commitment_pct: Rate,
    pub clawback: bool,
    pub fund_life_years: u32,
    pub investment_period_years: u32,
    pub gross_irr_assumption: Rate,
    pub gross_moic_assumption: Multiple,
    pub annual_fund_expenses: Money,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundFeeOutput {
    pub projections: Vec<FundYearProjection>,
    pub total_management_fees: Money,
    pub total_carried_interest: Money,
    pub total_gp_income: Money,
    pub lp_net_irr: Rate,
    pub lp_net_moic: Multiple,
    pub lp_dpi: Multiple,
    pub total_fee_drag: Rate,
    pub total_fee_drag_dollars: Money,
    pub gp_breakeven_aum: Money,
    pub waterfall_detail: Vec<WaterfallTierResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundYearProjection {
    pub year: u32,
    pub invested_capital: Money,
    pub nav: Money,
    pub distributions: Money,
    pub management_fee: Money,
    pub carry_accrual: Money,
    pub fund_expenses: Money,
    pub dpi: Multiple,
    pub rvpi: Multiple,
    pub tvpi: Multiple,
}
```

#### Investor Net Returns Calculator

```rust
// src/jurisdiction/investor_returns.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorNetReturnsInput {
    pub gross_return: Rate,
    pub investment_amount: Money,
    pub holding_period_years: Years,
    pub management_fee: Rate,
    pub performance_fee: Rate,
    pub hurdle_rate: Option<Rate>,
    pub fund_expenses_pct: Rate,
    pub fof_management_fee: Option<Rate>,
    pub fof_performance_fee: Option<Rate>,
    pub wht_drag: Rate,
    pub blocker_cost: Option<Rate>,
    pub investor_tax_rate: Option<Rate>,
    pub fund_currency: Currency,
    pub investor_currency: Currency,
    pub fx_hedging_cost: Option<Rate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorNetReturnsOutput {
    pub gross_return: Rate,
    pub after_management_fee: Rate,
    pub after_performance_fee: Rate,
    pub after_fund_expenses: Rate,
    pub after_wht: Rate,
    pub after_fx_hedging: Option<Rate>,
    pub net_return: Rate,
    pub total_fee_drag: Rate,
    pub gross_amount: Money,
    pub net_amount: Money,
    pub fees_paid: Money,
    pub cost_breakdown: Vec<CostLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostLayer {
    pub name: String,
    pub annual_rate: Rate,
    pub total_cost: Money,
    pub pct_of_total_drag: Rate,
}
```

---

## 3. Domain Services

Domain services encapsulate operations that don't naturally belong to a single aggregate.

### 3.1 Time Value Service

```rust
// src/time_value.rs — Used by multiple contexts

pub fn npv(rate: Rate, cash_flows: &[Money]) -> CorpFinanceResult<Money>;
pub fn irr(cash_flows: &[Money], guess: Rate) -> CorpFinanceResult<Rate>;
pub fn xirr(dated_flows: &[(NaiveDate, Money)], guess: Rate) -> CorpFinanceResult<Rate>;
pub fn pv(rate: Rate, nper: u32, pmt: Money, fv: Money) -> CorpFinanceResult<Money>;
pub fn fv(rate: Rate, nper: u32, pmt: Money, pv: Money) -> CorpFinanceResult<Money>;
pub fn pmt(rate: Rate, nper: u32, pv: Money, fv: Money) -> CorpFinanceResult<Money>;
```

### 3.2 Computation Envelope Service

```rust
// Wraps any calculation with metadata, timing, and warnings

pub fn with_metadata<T: Serialize>(
    methodology: &str,
    assumptions: &impl Serialize,
    compute: impl FnOnce() -> CorpFinanceResult<T>,
) -> CorpFinanceResult<ComputationOutput<T>> {
    let start = std::time::Instant::now();
    let result = compute()?;
    let elapsed = start.elapsed();
    
    Ok(ComputationOutput {
        result,
        methodology: methodology.to_string(),
        assumptions: serde_json::to_value(assumptions)?,
        warnings: vec![], // Populated by calling code
        metadata: ComputationMetadata {
            version: env!("CARGO_PKG_VERSION"),
            computation_time_us: elapsed.as_micros() as u64,
            precision: "rust_decimal_128bit",
        },
    })
}
```

---

## 4. Anti-Corruption Layers

### 4.1 MCP Anti-Corruption Layer

The MCP server translates between MCP protocol concepts and domain concepts:

```
MCP Concept          →  Domain Concept
─────────────────       ──────────────
Tool name            →  Function identifier
Tool parameters      →  Input struct (via Zod → JSON → serde)
Tool response        →  ComputationOutput<T> (via serde → JSON → MCP text)
MCP error            →  CorpFinanceError (mapped to MCP error codes)
```

**The domain never knows about MCP.** All MCP-specific types stay in `packages/mcp-server/`.

### 4.2 CLI Anti-Corruption Layer

The CLI translates between shell concepts and domain concepts:

```
Shell Concept        →  Domain Concept
─────────────────       ──────────────
Subcommand           →  Function identifier
Flags (--arg)        →  Input struct fields
stdin (pipe)         →  Partial input (merged with flags)
--output format      →  Presentation concern (not domain)
Exit code            →  CorpFinanceError (0 = success, 1 = error)
```

**The domain never knows about the CLI.** All CLI-specific types stay in `crates/corp-finance-cli/`.

---

## 5. Invariants

These are rules that must ALWAYS hold. Violation is a bug.

### 5.1 Global Invariants

- All monetary calculations use `rust_decimal::Decimal`, never `f64` (except Monte Carlo)
- Every public function returns `Result<T, CorpFinanceError>`, never panics
- Every output includes the methodology and assumptions used
- No function performs I/O (network, file, database)
- No function maintains state between calls
- Weights that should sum to 1.0 are validated (debt_weight + equity_weight = 1.0)

### 5.2 Valuation Invariants

- Terminal growth rate < WACC (otherwise infinite value)
- WACC > 0 (negative WACC is meaningless)
- Discount factors are always positive
- Enterprise value = PV(FCFF) + PV(Terminal Value)
- Equity value = Enterprise Value - Net Debt - Minority Interest

### 5.3 Credit Invariants

- Net Debt = Total Debt - Cash
- Interest coverage = EBITDA / Interest (not EBITDA / 0)
- Covenant headroom = (threshold - actual) / threshold for MaxOf covenants
- Synthetic rating is monotonically related to leverage and coverage

### 5.4 LBO Invariants

- Sources = Uses (always balanced)
- Total debt + equity = enterprise value + fees
- Cash sweep cannot exceed available cash flow
- Senior debt serviced before junior (seniority respected)
- IRR sign convention: negative at entry, positive at exit

### 5.5 Bond Invariants

- Dirty price = Clean price + Accrued interest
- Duration > 0 for coupon-paying bonds
- Convexity > 0 for non-callable bonds
- YTM and price move inversely
- DV01 = Modified Duration × Price × 0.01

### 5.6 Jurisdiction & Fund Invariants

- Equalisation adjustments net to zero across all investors (zero-sum)
- NAV per share × shares outstanding = class total NAV (always balanced)
- High-water mark only moves up, never down
- Performance fee accrual ≥ 0 (can't be negative — no fee rebate for losses)
- Management fee basis must match fund lifecycle stage (committed during investment period, invested after)
- European waterfall: carry computed on total fund profit, not per-deal
- American waterfall: carry computed per realised investment
- Clawback obligation = max(carry received - carry entitled, 0)
- WHT rate ≤ statutory rate (treaties only reduce, never increase)
- WHT effective rate = min(statutory, treaty) where treaty exists
- GAAP/IFRS reconciliation adjustments must balance (asset adjustment = liability adjustment + equity adjustment)
- Sum of LP + GP distributions = total fund distributions (no leakage)
- TVPI = DPI + RVPI (always, by definition)
- GP commitment % of fund size × fund return = GP co-invest return

---

## 6. Domain Events (Future Consideration)

While the current architecture is stateless, future versions might emit domain events for observability:

```rust
pub enum CorpFinanceEvent {
    CalculationCompleted { tool: String, duration_us: u64 },
    WarningGenerated { tool: String, warning: String },
    ConvergenceAchieved { function: String, iterations: u32 },
    CovenantBreach { covenant: String, actual: Decimal, threshold: Decimal },
}
```

These would be opt-in, emitted via a trait, and consumed by telemetry/logging — never affecting computation.
