use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Working Capital Analysis — Input / Output types
// ---------------------------------------------------------------------------

/// Input for working capital analysis across multiple periods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingCapitalInput {
    /// Company identifier
    pub company_name: String,
    /// Multiple periods for trend analysis (chronological order)
    pub periods: Vec<WcPeriod>,
    /// Optional industry benchmarks for peer comparison
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry_benchmarks: Option<IndustryBenchmarks>,
    /// Weighted average cost of capital for financing savings
    pub cost_of_capital: Rate,
}

/// A single period of working capital data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcPeriod {
    /// Period label, e.g. "Q1 2024"
    pub period_name: String,
    /// Total revenue for the period
    pub revenue: Money,
    /// Cost of goods sold
    pub cogs: Money,
    /// Accounts receivable balance
    pub accounts_receivable: Money,
    /// Inventory balance
    pub inventory: Money,
    /// Accounts payable balance
    pub accounts_payable: Money,
    /// Prepaid expenses, other current assets
    pub other_current_assets: Money,
    /// Accrued expenses, other current liabilities
    pub other_current_liabilities: Money,
    /// Number of days in the period (90 for quarter, 365 for year)
    pub days_in_period: u32,
}

/// Industry benchmark medians for peer comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryBenchmarks {
    pub dso_median: Decimal,
    pub dio_median: Decimal,
    pub dpo_median: Decimal,
    pub ccc_median: Decimal,
}

/// Full output of working capital analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingCapitalOutput {
    /// Per-period metrics
    pub period_metrics: Vec<WcMetrics>,
    /// Trend analysis across periods
    pub trend_analysis: TrendAnalysis,
    /// Optimization opportunities
    pub optimization: WcOptimization,
    /// Benchmark comparison (if benchmarks provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_comparison: Option<BenchmarkComparison>,
}

/// Computed metrics for a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcMetrics {
    pub period_name: String,
    /// Days Sales Outstanding = AR / (Revenue / days)
    pub dso: Decimal,
    /// Days Inventory Outstanding = Inventory / (COGS / days)
    pub dio: Decimal,
    /// Days Payable Outstanding = AP / (COGS / days)
    pub dpo: Decimal,
    /// Cash Conversion Cycle = DSO + DIO - DPO
    pub ccc: Decimal,
    /// Current assets minus current liabilities
    pub net_working_capital: Money,
    /// NWC as percentage of revenue
    pub nwc_as_pct_revenue: Rate,
    /// Current assets / current liabilities
    pub current_ratio: Decimal,
    /// (Current assets - inventory) / current liabilities
    pub quick_ratio: Decimal,
}

/// Trend analysis comparing first and last periods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// "Improving", "Deteriorating", or "Stable"
    pub dso_trend: String,
    pub dio_trend: String,
    pub dpo_trend: String,
    pub ccc_trend: String,
    /// Absolute change from first to last period
    pub dso_change: Decimal,
    pub dio_change: Decimal,
    pub dpo_change: Decimal,
    pub ccc_change: Decimal,
}

/// Working capital optimization opportunities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcOptimization {
    /// Cash freed if DSO reduced by 5 days (revenue/365 * 5)
    pub cash_freed_if_dso_reduced_5d: Money,
    /// Cash freed if DIO reduced by 5 days (cogs/365 * 5)
    pub cash_freed_if_dio_reduced_5d: Money,
    /// Cash cost if DPO reduced by 5 days (cogs/365 * 5)
    pub cash_cost_if_dpo_reduced_5d: Money,
    /// Sum of DSO + DIO savings
    pub total_optimization_opportunity: Money,
    /// Opportunity * cost_of_capital
    pub annual_financing_savings: Money,
    /// Actionable recommendations
    pub recommendations: Vec<String>,
}

/// Comparison against industry benchmarks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    /// Days above/below DSO median (positive = worse)
    pub dso_vs_median: Decimal,
    /// Days above/below DIO median (positive = worse)
    pub dio_vs_median: Decimal,
    /// Days above/below DPO median (positive = worse, i.e., paying faster than peers)
    pub dpo_vs_median: Decimal,
    /// Days above/below CCC median (positive = worse)
    pub ccc_vs_median: Decimal,
    /// "Better than peers", "In-line", or "Worse than peers"
    pub overall_position: String,
}

// ---------------------------------------------------------------------------
// Rolling Forecast — Input / Output types
// ---------------------------------------------------------------------------

/// Input for building a rolling financial forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingForecastInput {
    pub company_name: String,
    /// At least 4 periods of historical data
    pub historical_periods: Vec<ForecastPeriod>,
    /// Number of periods to forecast forward
    pub forecast_periods: u32,
    /// Assumed revenue growth rate per period
    pub revenue_growth_rate: Rate,
    /// Driver assumptions (overrides or derived from history)
    pub drivers: ForecastDrivers,
}

/// A single historical period for the rolling forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPeriod {
    pub period_name: String,
    pub revenue: Money,
    pub cogs: Money,
    pub operating_expenses: Money,
    pub capex: Money,
    pub depreciation: Money,
}

/// Driver assumptions for the forecast model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastDrivers {
    /// COGS as % of revenue (overrides historical average if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cogs_pct_revenue: Option<Rate>,
    /// Operating expenses as % of revenue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opex_pct_revenue: Option<Rate>,
    /// Capital expenditures as % of revenue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capex_pct_revenue: Option<Rate>,
    /// Depreciation as % of PP&E (not used in simple model; included for extensibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depreciation_pct_ppe: Option<Rate>,
    /// Corporate tax rate
    pub tax_rate: Rate,
}

/// Full rolling forecast output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingForecastOutput {
    /// Historical periods (echoed back with computed fields)
    pub historical: Vec<ForecastRow>,
    /// Forecasted periods
    pub forecast: Vec<ForecastRow>,
    /// Driver assumptions used
    pub driver_assumptions: DriverAssumptions,
    /// Summary statistics
    pub summary: ForecastSummary,
}

/// A single row in the forecast (historical or projected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastRow {
    pub period_name: String,
    pub revenue: Money,
    pub cogs: Money,
    pub gross_profit: Money,
    pub gross_margin: Rate,
    pub operating_expenses: Money,
    pub ebitda: Money,
    pub ebitda_margin: Rate,
    pub depreciation: Money,
    pub ebit: Money,
    pub tax: Money,
    pub net_income: Money,
    pub capex: Money,
    pub free_cash_flow: Money,
    pub is_forecast: bool,
}

/// The driver assumptions that were actually used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverAssumptions {
    pub cogs_pct: Rate,
    pub opex_pct: Rate,
    pub capex_pct: Rate,
    pub source: String,
}

/// Summary statistics across the forecast horizon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastSummary {
    /// Revenue CAGR across the forecast periods
    pub forecast_revenue_cagr: Rate,
    /// Average EBITDA margin across forecast periods
    pub avg_forecast_ebitda_margin: Rate,
    /// Cumulative free cash flow across forecast periods
    pub cumulative_fcf: Money,
    /// Revenue in the final forecast period
    pub terminal_revenue: Money,
}

// ---------------------------------------------------------------------------
// Public API — Working Capital Analysis
// ---------------------------------------------------------------------------

/// Analyse working capital efficiency across multiple periods, computing
/// DSO/DIO/DPO/CCC, trends, optimisation opportunities, and optional
/// industry benchmark comparison.
pub fn analyze_working_capital(
    input: &WorkingCapitalInput,
) -> CorpFinanceResult<ComputationOutput<WorkingCapitalOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    validate_wc_input(input)?;

    // -- Compute per-period metrics ------------------------------------------
    let period_metrics: Vec<WcMetrics> = input
        .periods
        .iter()
        .map(|p| compute_period_metrics(p, &mut warnings))
        .collect::<CorpFinanceResult<Vec<_>>>()?;

    // -- Trend analysis ------------------------------------------------------
    let trend_analysis = compute_trend(&period_metrics);

    // -- Optimization (based on last period) ---------------------------------
    let last_period = input.periods.last().unwrap(); // validated non-empty
    let last_metrics = period_metrics.last().unwrap();
    let optimization = compute_optimization(
        last_period,
        last_metrics,
        input.cost_of_capital,
        &mut warnings,
    );

    // -- Benchmark comparison ------------------------------------------------
    let benchmark_comparison = input
        .industry_benchmarks
        .as_ref()
        .map(|bench| compute_benchmark(last_metrics, bench));

    let output = WorkingCapitalOutput {
        period_metrics,
        trend_analysis,
        optimization,
        benchmark_comparison,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Working Capital Analysis (DSO/DIO/DPO/CCC)",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Public API — Rolling Forecast
// ---------------------------------------------------------------------------

/// Build a rolling financial forecast from historical data and growth
/// assumptions, projecting revenue, margins, and free cash flow.
pub fn build_rolling_forecast(
    input: &RollingForecastInput,
) -> CorpFinanceResult<ComputationOutput<RollingForecastOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    validate_forecast_input(input)?;

    // -- Derive driver assumptions -------------------------------------------
    let (cogs_pct, cogs_source) = derive_driver(
        input.drivers.cogs_pct_revenue,
        &input.historical_periods,
        |p| {
            if p.revenue.is_zero() {
                Decimal::ZERO
            } else {
                p.cogs / p.revenue
            }
        },
        "cogs_pct_revenue",
    );

    let (opex_pct, opex_source) = derive_driver(
        input.drivers.opex_pct_revenue,
        &input.historical_periods,
        |p| {
            if p.revenue.is_zero() {
                Decimal::ZERO
            } else {
                p.operating_expenses / p.revenue
            }
        },
        "opex_pct_revenue",
    );

    let (capex_pct, capex_source) = derive_driver(
        input.drivers.capex_pct_revenue,
        &input.historical_periods,
        |p| {
            if p.revenue.is_zero() {
                Decimal::ZERO
            } else {
                p.capex / p.revenue
            }
        },
        "capex_pct_revenue",
    );

    let source = if cogs_source == "User override"
        && opex_source == "User override"
        && capex_source == "User override"
    {
        "User override".to_string()
    } else if cogs_source == "Historical average"
        && opex_source == "Historical average"
        && capex_source == "Historical average"
    {
        "Historical average".to_string()
    } else {
        format!(
            "Mixed (COGS: {}, OpEx: {}, CapEx: {})",
            cogs_source, opex_source, capex_source
        )
    };

    let driver_assumptions = DriverAssumptions {
        cogs_pct,
        opex_pct,
        capex_pct,
        source,
    };

    // -- Build historical rows -----------------------------------------------
    let historical: Vec<ForecastRow> = input
        .historical_periods
        .iter()
        .map(|p| build_historical_row(p, input.drivers.tax_rate))
        .collect();

    // -- Build forecast rows -------------------------------------------------
    let last_revenue = input.historical_periods.last().unwrap().revenue;
    let last_depreciation = input.historical_periods.last().unwrap().depreciation;
    let mut forecast: Vec<ForecastRow> = Vec::with_capacity(input.forecast_periods as usize);
    let mut prev_revenue = last_revenue;
    let mut prev_depreciation = last_depreciation;

    for i in 0..input.forecast_periods {
        let revenue = prev_revenue * (Decimal::ONE + input.revenue_growth_rate);
        let cogs = revenue * cogs_pct;
        let gross_profit = revenue - cogs;
        let gross_margin = if revenue.is_zero() {
            Decimal::ZERO
        } else {
            gross_profit / revenue
        };
        let opex = revenue * opex_pct;
        let depreciation = if let Some(dep_pct) = input.drivers.depreciation_pct_ppe {
            // Simple approach: grow depreciation with revenue
            prev_depreciation * (Decimal::ONE + input.revenue_growth_rate) * dep_pct
                / dep_pct.max(dec!(0.01))
            // This simplification just uses the prior depreciation scaled
        } else {
            // Use historical average depreciation-to-revenue ratio
            let avg_dep_ratio = compute_avg(&input.historical_periods, |p| {
                if p.revenue.is_zero() {
                    Decimal::ZERO
                } else {
                    p.depreciation / p.revenue
                }
            });
            revenue * avg_dep_ratio
        };
        let ebitda = gross_profit - opex;
        let ebitda_margin = if revenue.is_zero() {
            Decimal::ZERO
        } else {
            ebitda / revenue
        };
        let ebit = ebitda - depreciation;
        let tax = if ebit > Decimal::ZERO {
            ebit * input.drivers.tax_rate
        } else {
            Decimal::ZERO
        };
        let net_income = ebit - tax;
        let capex = revenue * capex_pct;
        let fcf = net_income + depreciation - capex;

        let period_name = format!("Forecast {}", i + 1);
        forecast.push(ForecastRow {
            period_name,
            revenue,
            cogs,
            gross_profit,
            gross_margin,
            operating_expenses: opex,
            ebitda,
            ebitda_margin,
            depreciation,
            ebit,
            tax,
            net_income,
            capex,
            free_cash_flow: fcf,
            is_forecast: true,
        });

        prev_revenue = revenue;
        prev_depreciation = depreciation;
    }

    // -- Summary statistics --------------------------------------------------
    let terminal_revenue = if forecast.is_empty() {
        last_revenue
    } else {
        forecast.last().unwrap().revenue
    };

    let forecast_revenue_cagr = input.revenue_growth_rate; // by construction

    let avg_forecast_ebitda_margin = if forecast.is_empty() {
        Decimal::ZERO
    } else {
        let sum: Decimal = forecast.iter().map(|r| r.ebitda_margin).sum();
        sum / Decimal::from(forecast.len() as u32)
    };

    let cumulative_fcf: Money = forecast.iter().map(|r| r.free_cash_flow).sum();

    if cumulative_fcf < Decimal::ZERO {
        warnings.push("Cumulative free cash flow is negative over the forecast horizon.".into());
    }

    let summary = ForecastSummary {
        forecast_revenue_cagr,
        avg_forecast_ebitda_margin,
        cumulative_fcf,
        terminal_revenue,
    };

    let output = RollingForecastOutput {
        historical,
        forecast,
        driver_assumptions,
        summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Rolling Financial Forecast",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers — Working Capital
// ---------------------------------------------------------------------------

fn validate_wc_input(input: &WorkingCapitalInput) -> CorpFinanceResult<()> {
    if input.periods.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one period is required for working capital analysis.".into(),
        ));
    }
    if input.cost_of_capital < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cost_of_capital".into(),
            reason: "Cost of capital cannot be negative.".into(),
        });
    }
    for p in &input.periods {
        if p.revenue < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "revenue".into(),
                reason: format!("Revenue cannot be negative in period '{}'.", p.period_name),
            });
        }
        if p.days_in_period == 0 {
            return Err(CorpFinanceError::InvalidInput {
                field: "days_in_period".into(),
                reason: format!(
                    "Days in period must be positive in period '{}'.",
                    p.period_name
                ),
            });
        }
    }
    Ok(())
}

/// Compute WcMetrics for a single period.
fn compute_period_metrics(
    p: &WcPeriod,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<WcMetrics> {
    let days = Decimal::from(p.days_in_period);

    // DSO = AR / (Revenue / days). If revenue is zero, DSO = 0.
    let dso = if p.revenue.is_zero() {
        if !p.accounts_receivable.is_zero() {
            warnings.push(format!(
                "Period '{}': revenue is zero but AR is non-zero; DSO set to 0.",
                p.period_name
            ));
        }
        Decimal::ZERO
    } else {
        p.accounts_receivable / (p.revenue / days)
    };

    // DIO = Inventory / (COGS / days). If COGS is zero, DIO = 0.
    let dio = if p.cogs.is_zero() {
        if !p.inventory.is_zero() {
            warnings.push(format!(
                "Period '{}': COGS is zero but inventory is non-zero; DIO set to 0.",
                p.period_name
            ));
        }
        Decimal::ZERO
    } else {
        p.inventory / (p.cogs / days)
    };

    // DPO = AP / (COGS / days). If COGS is zero, DPO = 0.
    let dpo = if p.cogs.is_zero() {
        if !p.accounts_payable.is_zero() {
            warnings.push(format!(
                "Period '{}': COGS is zero but AP is non-zero; DPO set to 0.",
                p.period_name
            ));
        }
        Decimal::ZERO
    } else {
        p.accounts_payable / (p.cogs / days)
    };

    let ccc = dso + dio - dpo;

    let current_assets = p.accounts_receivable + p.inventory + p.other_current_assets;
    let current_liabilities = p.accounts_payable + p.other_current_liabilities;
    let net_working_capital = current_assets - current_liabilities;

    let nwc_as_pct_revenue = if p.revenue.is_zero() {
        Decimal::ZERO
    } else {
        net_working_capital / p.revenue
    };

    let current_ratio = if current_liabilities.is_zero() {
        Decimal::ZERO
    } else {
        current_assets / current_liabilities
    };

    let quick_assets = current_assets - p.inventory;
    let quick_ratio = if current_liabilities.is_zero() {
        Decimal::ZERO
    } else {
        quick_assets / current_liabilities
    };

    Ok(WcMetrics {
        period_name: p.period_name.clone(),
        dso,
        dio,
        dpo,
        ccc,
        net_working_capital,
        nwc_as_pct_revenue,
        current_ratio,
        quick_ratio,
    })
}

/// Compute trend analysis from first to last period.
fn compute_trend(metrics: &[WcMetrics]) -> TrendAnalysis {
    if metrics.len() < 2 {
        // Single period: everything is stable, zero change
        let zero = Decimal::ZERO;
        return TrendAnalysis {
            dso_trend: "Stable".to_string(),
            dio_trend: "Stable".to_string(),
            dpo_trend: "Stable".to_string(),
            ccc_trend: "Stable".to_string(),
            dso_change: zero,
            dio_change: zero,
            dpo_change: zero,
            ccc_change: zero,
        };
    }

    let first = &metrics[0];
    let last = &metrics[metrics.len() - 1];
    let threshold = dec!(5);

    let dso_change = last.dso - first.dso;
    let dio_change = last.dio - first.dio;
    let dpo_change = last.dpo - first.dpo;
    let ccc_change = last.ccc - first.ccc;

    // For DSO/DIO/CCC: negative change = improving (fewer days)
    let dso_trend = classify_trend_lower_is_better(dso_change, threshold);
    let dio_trend = classify_trend_lower_is_better(dio_change, threshold);
    // For DPO: higher = better (paying suppliers later = more favourable)
    let dpo_trend = classify_trend_higher_is_better(dpo_change, threshold);
    let ccc_trend = classify_trend_lower_is_better(ccc_change, threshold);

    TrendAnalysis {
        dso_trend,
        dio_trend,
        dpo_trend,
        ccc_trend,
        dso_change,
        dio_change,
        dpo_change,
        ccc_change,
    }
}

/// For metrics where lower is better (DSO, DIO, CCC):
/// negative change > threshold = "Improving"
/// positive change > threshold = "Deteriorating"
fn classify_trend_lower_is_better(change: Decimal, threshold: Decimal) -> String {
    if change < -threshold {
        "Improving".to_string()
    } else if change > threshold {
        "Deteriorating".to_string()
    } else {
        "Stable".to_string()
    }
}

/// For metrics where higher is better (DPO):
/// positive change > threshold = "Improving"
/// negative change > threshold = "Deteriorating"
fn classify_trend_higher_is_better(change: Decimal, threshold: Decimal) -> String {
    if change > threshold {
        "Improving".to_string()
    } else if change < -threshold {
        "Deteriorating".to_string()
    } else {
        "Stable".to_string()
    }
}

/// Compute optimization opportunities from the last period.
fn compute_optimization(
    period: &WcPeriod,
    metrics: &WcMetrics,
    cost_of_capital: Rate,
    _warnings: &mut Vec<String>,
) -> WcOptimization {
    let days_365 = dec!(365);
    let five_days = dec!(5);

    // Cash freed from reducing DSO by 5 days = daily_revenue * 5
    let daily_revenue = if days_365.is_zero() {
        Decimal::ZERO
    } else {
        period.revenue / days_365
    };
    let cash_freed_dso = daily_revenue * five_days;

    // Cash freed from reducing DIO by 5 days = daily_cogs * 5
    let daily_cogs = if days_365.is_zero() {
        Decimal::ZERO
    } else {
        period.cogs / days_365
    };
    let cash_freed_dio = daily_cogs * five_days;

    // Cash cost if DPO reduced by 5 days = daily_cogs * 5
    let cash_cost_dpo = daily_cogs * five_days;

    // Total opportunity = DSO savings + DIO savings (DPO reduction is a cost, not added)
    let total_optimization = cash_freed_dso + cash_freed_dio;

    let annual_financing_savings = total_optimization * cost_of_capital;

    let mut recommendations = Vec::new();

    if metrics.dso > dec!(45) {
        recommendations.push(
            "DSO exceeds 45 days. Consider tightening credit terms, \
             offering early-payment discounts, or improving collections processes."
                .to_string(),
        );
    }
    if metrics.dio > dec!(60) {
        recommendations.push(
            "DIO exceeds 60 days. Evaluate inventory management practices, \
             consider JIT procurement or SKU rationalization."
                .to_string(),
        );
    }
    if metrics.dpo < dec!(30) {
        recommendations.push(
            "DPO is below 30 days. Negotiate extended payment terms with suppliers \
             to improve cash flow."
                .to_string(),
        );
    }
    if metrics.ccc > dec!(60) {
        recommendations.push(
            "Cash conversion cycle exceeds 60 days. Prioritize a holistic working \
             capital programme addressing receivables, inventory, and payables."
                .to_string(),
        );
    }
    if recommendations.is_empty() {
        recommendations.push(
            "Working capital metrics are within acceptable ranges. \
             Continue monitoring for seasonal variations."
                .to_string(),
        );
    }

    WcOptimization {
        cash_freed_if_dso_reduced_5d: cash_freed_dso,
        cash_freed_if_dio_reduced_5d: cash_freed_dio,
        cash_cost_if_dpo_reduced_5d: cash_cost_dpo,
        total_optimization_opportunity: total_optimization,
        annual_financing_savings,
        recommendations,
    }
}

/// Compare last-period metrics against industry benchmarks.
fn compute_benchmark(metrics: &WcMetrics, bench: &IndustryBenchmarks) -> BenchmarkComparison {
    let dso_vs = metrics.dso - bench.dso_median;
    let dio_vs = metrics.dio - bench.dio_median;
    // For DPO, negative vs_median means company pays faster than peers (worse)
    let dpo_vs = bench.dpo_median - metrics.dpo;
    let ccc_vs = metrics.ccc - bench.ccc_median;

    // Overall: count how many metrics are worse. Positive = worse for all.
    let worse_count = [dso_vs, dio_vs, dpo_vs, ccc_vs]
        .iter()
        .filter(|v| **v > dec!(5))
        .count();
    let better_count = [dso_vs, dio_vs, dpo_vs, ccc_vs]
        .iter()
        .filter(|v| **v < dec!(-5))
        .count();

    let overall_position = if better_count >= 3 {
        "Better than peers".to_string()
    } else if worse_count >= 3 {
        "Worse than peers".to_string()
    } else {
        "In-line".to_string()
    };

    BenchmarkComparison {
        dso_vs_median: dso_vs,
        dio_vs_median: dio_vs,
        dpo_vs_median: dpo_vs,
        ccc_vs_median: ccc_vs,
        overall_position,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers — Rolling Forecast
// ---------------------------------------------------------------------------

fn validate_forecast_input(input: &RollingForecastInput) -> CorpFinanceResult<()> {
    if input.historical_periods.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one historical period is required for a rolling forecast.".into(),
        ));
    }
    if input.forecast_periods == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "forecast_periods".into(),
            reason: "Must forecast at least one period.".into(),
        });
    }
    if input.drivers.tax_rate < Decimal::ZERO || input.drivers.tax_rate > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "tax_rate".into(),
            reason: "Tax rate must be between 0 and 1.".into(),
        });
    }
    Ok(())
}

/// Derive a driver percentage: use override if provided, otherwise historical average.
fn derive_driver<F>(
    override_val: Option<Rate>,
    periods: &[ForecastPeriod],
    ratio_fn: F,
    _name: &str,
) -> (Rate, String)
where
    F: Fn(&ForecastPeriod) -> Decimal,
{
    match override_val {
        Some(val) => (val, "User override".to_string()),
        None => {
            let avg = compute_avg(periods, ratio_fn);
            (avg, "Historical average".to_string())
        }
    }
}

/// Compute the simple average of a derived value across forecast periods.
fn compute_avg<F>(periods: &[ForecastPeriod], f: F) -> Decimal
where
    F: Fn(&ForecastPeriod) -> Decimal,
{
    if periods.is_empty() {
        return Decimal::ZERO;
    }
    let sum: Decimal = periods.iter().map(&f).sum();
    sum / Decimal::from(periods.len() as u32)
}

/// Build a ForecastRow from a historical period.
fn build_historical_row(p: &ForecastPeriod, tax_rate: Rate) -> ForecastRow {
    let gross_profit = p.revenue - p.cogs;
    let gross_margin = if p.revenue.is_zero() {
        Decimal::ZERO
    } else {
        gross_profit / p.revenue
    };
    let ebitda = gross_profit - p.operating_expenses;
    let ebitda_margin = if p.revenue.is_zero() {
        Decimal::ZERO
    } else {
        ebitda / p.revenue
    };
    let ebit = ebitda - p.depreciation;
    let tax = if ebit > Decimal::ZERO {
        ebit * tax_rate
    } else {
        Decimal::ZERO
    };
    let net_income = ebit - tax;
    let fcf = net_income + p.depreciation - p.capex;

    ForecastRow {
        period_name: p.period_name.clone(),
        revenue: p.revenue,
        cogs: p.cogs,
        gross_profit,
        gross_margin,
        operating_expenses: p.operating_expenses,
        ebitda,
        ebitda_margin,
        depreciation: p.depreciation,
        ebit,
        tax,
        net_income,
        capex: p.capex,
        free_cash_flow: fcf,
        is_forecast: false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -- Test helpers --------------------------------------------------------

    fn sample_period(name: &str) -> WcPeriod {
        WcPeriod {
            period_name: name.to_string(),
            revenue: dec!(1_000_000),
            cogs: dec!(600_000),
            accounts_receivable: dec!(150_000),
            inventory: dec!(100_000),
            accounts_payable: dec!(80_000),
            other_current_assets: dec!(50_000),
            other_current_liabilities: dec!(30_000),
            days_in_period: 365,
        }
    }

    fn sample_wc_input() -> WorkingCapitalInput {
        WorkingCapitalInput {
            company_name: "TestCo".to_string(),
            periods: vec![sample_period("FY2023")],
            industry_benchmarks: None,
            cost_of_capital: dec!(0.10),
        }
    }

    fn multi_period_input(periods: Vec<WcPeriod>) -> WorkingCapitalInput {
        WorkingCapitalInput {
            company_name: "TrendCo".to_string(),
            periods,
            industry_benchmarks: None,
            cost_of_capital: dec!(0.08),
        }
    }

    fn sample_forecast_input() -> RollingForecastInput {
        RollingForecastInput {
            company_name: "ForecastCo".to_string(),
            historical_periods: vec![
                ForecastPeriod {
                    period_name: "Y1".to_string(),
                    revenue: dec!(1_000_000),
                    cogs: dec!(600_000),
                    operating_expenses: dec!(200_000),
                    capex: dec!(50_000),
                    depreciation: dec!(30_000),
                },
                ForecastPeriod {
                    period_name: "Y2".to_string(),
                    revenue: dec!(1_100_000),
                    cogs: dec!(660_000),
                    operating_expenses: dec!(220_000),
                    capex: dec!(55_000),
                    depreciation: dec!(33_000),
                },
                ForecastPeriod {
                    period_name: "Y3".to_string(),
                    revenue: dec!(1_200_000),
                    cogs: dec!(720_000),
                    operating_expenses: dec!(240_000),
                    capex: dec!(60_000),
                    depreciation: dec!(36_000),
                },
                ForecastPeriod {
                    period_name: "Y4".to_string(),
                    revenue: dec!(1_300_000),
                    cogs: dec!(780_000),
                    operating_expenses: dec!(260_000),
                    capex: dec!(65_000),
                    depreciation: dec!(39_000),
                },
            ],
            forecast_periods: 3,
            revenue_growth_rate: dec!(0.10),
            drivers: ForecastDrivers {
                cogs_pct_revenue: None,
                opex_pct_revenue: None,
                capex_pct_revenue: None,
                depreciation_pct_ppe: None,
                tax_rate: dec!(0.25),
            },
        }
    }

    // -- Working Capital Tests -----------------------------------------------

    #[test]
    fn test_dso_calculation() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let m = &result.result.period_metrics[0];
        // DSO = 150_000 / (1_000_000 / 365) = 150_000 * 365 / 1_000_000 = 54.75
        let expected_dso = dec!(150_000) / (dec!(1_000_000) / dec!(365));
        assert_eq!(m.dso, expected_dso, "DSO mismatch");
    }

    #[test]
    fn test_dio_calculation() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let m = &result.result.period_metrics[0];
        // DIO = 100_000 / (600_000 / 365)
        let expected_dio = dec!(100_000) / (dec!(600_000) / dec!(365));
        assert_eq!(m.dio, expected_dio, "DIO mismatch");
    }

    #[test]
    fn test_dpo_calculation() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let m = &result.result.period_metrics[0];
        // DPO = 80_000 / (600_000 / 365)
        let expected_dpo = dec!(80_000) / (dec!(600_000) / dec!(365));
        assert_eq!(m.dpo, expected_dpo, "DPO mismatch");
    }

    #[test]
    fn test_ccc_equals_dso_plus_dio_minus_dpo() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let m = &result.result.period_metrics[0];
        let expected_ccc = m.dso + m.dio - m.dpo;
        assert_eq!(m.ccc, expected_ccc, "CCC should equal DSO + DIO - DPO");
    }

    #[test]
    fn test_nwc_calculation() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let m = &result.result.period_metrics[0];
        // current_assets = AR + inventory + other_current_assets = 150k + 100k + 50k = 300k
        // current_liabilities = AP + other_current_liabilities = 80k + 30k = 110k
        // NWC = 300k - 110k = 190k
        assert_eq!(
            m.net_working_capital,
            dec!(190_000),
            "NWC = current assets - current liabilities"
        );
    }

    #[test]
    fn test_current_ratio_and_quick_ratio() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let m = &result.result.period_metrics[0];
        // current_assets = 300k, current_liabilities = 110k
        let expected_current = dec!(300_000) / dec!(110_000);
        assert_eq!(m.current_ratio, expected_current, "Current ratio mismatch");

        // quick_assets = 300k - 100k = 200k
        let expected_quick = dec!(200_000) / dec!(110_000);
        assert_eq!(m.quick_ratio, expected_quick, "Quick ratio mismatch");
    }

    #[test]
    fn test_multi_period_trend_improving_dso() {
        // DSO decreasing by more than 5 days => "Improving"
        let mut p1 = sample_period("Q1");
        let mut p2 = sample_period("Q2");
        // p1: AR=150k => DSO ~54.75; p2: AR=100k => DSO ~36.5
        p1.accounts_receivable = dec!(150_000);
        p2.accounts_receivable = dec!(100_000);

        let input = multi_period_input(vec![p1, p2]);
        let result = analyze_working_capital(&input).unwrap();
        assert_eq!(
            result.result.trend_analysis.dso_trend, "Improving",
            "DSO should be improving when it decreases significantly"
        );
        assert!(
            result.result.trend_analysis.dso_change < Decimal::ZERO,
            "DSO change should be negative when improving"
        );
    }

    #[test]
    fn test_multi_period_trend_deteriorating_ccc() {
        // CCC increasing by more than 5 days => "Deteriorating"
        let p1 = sample_period("Q1");
        let mut p2 = sample_period("Q2");
        // Increase AR and inventory significantly to raise CCC
        p2.accounts_receivable = dec!(250_000);
        p2.inventory = dec!(200_000);

        let input = multi_period_input(vec![p1, p2]);
        let result = analyze_working_capital(&input).unwrap();
        assert_eq!(
            result.result.trend_analysis.ccc_trend, "Deteriorating",
            "CCC should be deteriorating when it increases significantly"
        );
    }

    #[test]
    fn test_optimization_cash_freed_from_dso_reduction() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let opt = &result.result.optimization;
        // Cash freed = revenue / 365 * 5 = 1_000_000 / 365 * 5
        let expected = dec!(1_000_000) / dec!(365) * dec!(5);
        assert_eq!(opt.cash_freed_if_dso_reduced_5d, expected);
    }

    #[test]
    fn test_annual_financing_savings() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let opt = &result.result.optimization;
        // total_optimization * cost_of_capital
        let expected = opt.total_optimization_opportunity * dec!(0.10);
        assert_eq!(opt.annual_financing_savings, expected);
    }

    #[test]
    fn test_benchmark_better_than_peers() {
        let mut input = sample_wc_input();
        // Our DSO ~54.75, DIO ~60.83, DPO ~48.67, CCC ~66.92
        // Set benchmarks so our company is much better (lower DSO/DIO/CCC, higher DPO)
        input.industry_benchmarks = Some(IndustryBenchmarks {
            dso_median: dec!(70),
            dio_median: dec!(80),
            dpo_median: dec!(40),
            ccc_median: dec!(90),
        });
        let result = analyze_working_capital(&input).unwrap();
        let bench = result.result.benchmark_comparison.as_ref().unwrap();
        assert_eq!(
            bench.overall_position, "Better than peers",
            "Company with lower DSO/DIO/CCC and higher DPO should be better than peers"
        );
    }

    #[test]
    fn test_benchmark_worse_than_peers() {
        let mut input = sample_wc_input();
        // Set benchmarks so our company is much worse
        input.industry_benchmarks = Some(IndustryBenchmarks {
            dso_median: dec!(30),
            dio_median: dec!(30),
            dpo_median: dec!(70),
            ccc_median: dec!(20),
        });
        let result = analyze_working_capital(&input).unwrap();
        let bench = result.result.benchmark_comparison.as_ref().unwrap();
        assert_eq!(
            bench.overall_position, "Worse than peers",
            "Company with higher DSO/DIO/CCC and lower DPO should be worse than peers"
        );
    }

    #[test]
    fn test_recommendations_generated() {
        let mut input = sample_wc_input();
        // Set high DSO, DIO, low DPO to trigger multiple recommendations
        input.periods[0].accounts_receivable = dec!(200_000); // DSO ~73 days
        input.periods[0].inventory = dec!(200_000); // DIO ~121 days
        input.periods[0].accounts_payable = dec!(40_000); // DPO ~24 days

        let result = analyze_working_capital(&input).unwrap();
        let recs = &result.result.optimization.recommendations;
        assert!(
            recs.len() >= 3,
            "Should generate recommendations for high DSO, DIO, and low DPO. Got: {:?}",
            recs
        );
    }

    // -- Rolling Forecast Tests ----------------------------------------------

    #[test]
    fn test_forecast_revenue_growth_applied() {
        let input = sample_forecast_input();
        let result = build_rolling_forecast(&input).unwrap();
        let forecast = &result.result.forecast;
        assert_eq!(forecast.len(), 3, "Should have 3 forecast periods");

        // First forecast period revenue = 1_300_000 * 1.10 = 1_430_000
        let expected_rev1 = dec!(1_300_000) * dec!(1.10);
        assert_eq!(
            forecast[0].revenue, expected_rev1,
            "Revenue growth mismatch"
        );

        // Second forecast period
        let expected_rev2 = expected_rev1 * dec!(1.10);
        assert_eq!(
            forecast[1].revenue, expected_rev2,
            "Compounding revenue growth mismatch"
        );
    }

    #[test]
    fn test_forecast_cogs_from_historical_average() {
        let input = sample_forecast_input();
        let result = build_rolling_forecast(&input).unwrap();

        // Historical COGS/Revenue: 600/1000=0.6, 660/1100=0.6, 720/1200=0.6, 780/1300=0.6
        // Average = 0.6
        assert_eq!(
            result.result.driver_assumptions.cogs_pct,
            dec!(0.6),
            "COGS pct should be historical average of 0.6"
        );
        assert!(
            result
                .result
                .driver_assumptions
                .source
                .contains("Historical average"),
            "Source should indicate historical average"
        );

        // Verify COGS in first forecast period
        let rev1 = result.result.forecast[0].revenue;
        let cogs1 = result.result.forecast[0].cogs;
        assert_eq!(cogs1, rev1 * dec!(0.6), "COGS should be 60% of revenue");
    }

    #[test]
    fn test_forecast_user_driver_override() {
        let mut input = sample_forecast_input();
        input.drivers.cogs_pct_revenue = Some(dec!(0.55));
        input.drivers.opex_pct_revenue = Some(dec!(0.15));
        input.drivers.capex_pct_revenue = Some(dec!(0.04));

        let result = build_rolling_forecast(&input).unwrap();
        assert_eq!(
            result.result.driver_assumptions.cogs_pct,
            dec!(0.55),
            "COGS pct should reflect user override"
        );
        assert_eq!(
            result.result.driver_assumptions.opex_pct,
            dec!(0.15),
            "OpEx pct should reflect user override"
        );
        assert_eq!(
            result.result.driver_assumptions.source, "User override",
            "Source should be 'User override'"
        );

        let rev1 = result.result.forecast[0].revenue;
        let cogs1 = result.result.forecast[0].cogs;
        assert_eq!(cogs1, rev1 * dec!(0.55));
    }

    #[test]
    fn test_fcf_calculation_in_forecast() {
        let input = sample_forecast_input();
        let result = build_rolling_forecast(&input).unwrap();

        for row in &result.result.forecast {
            let expected_fcf = row.net_income + row.depreciation - row.capex;
            assert_eq!(
                row.free_cash_flow, expected_fcf,
                "FCF = net_income + depreciation - capex for period '{}'",
                row.period_name
            );
        }
    }

    #[test]
    fn test_cumulative_fcf_over_forecast() {
        let input = sample_forecast_input();
        let result = build_rolling_forecast(&input).unwrap();

        let manual_sum: Decimal = result
            .result
            .forecast
            .iter()
            .map(|r| r.free_cash_flow)
            .sum();
        assert_eq!(
            result.result.summary.cumulative_fcf, manual_sum,
            "Cumulative FCF should be sum of all forecast FCFs"
        );
    }

    #[test]
    fn test_ebitda_margin_consistency() {
        let input = sample_forecast_input();
        let result = build_rolling_forecast(&input).unwrap();

        for row in &result.result.forecast {
            if !row.revenue.is_zero() {
                let expected_margin = row.ebitda / row.revenue;
                assert_eq!(
                    row.ebitda_margin, expected_margin,
                    "EBITDA margin mismatch in period '{}'",
                    row.period_name
                );
            }
        }
    }

    #[test]
    fn test_edge_zero_cogs_dio_dpo() {
        let mut input = sample_wc_input();
        input.periods[0].cogs = Decimal::ZERO;

        let result = analyze_working_capital(&input).unwrap();
        let m = &result.result.period_metrics[0];
        assert_eq!(m.dio, Decimal::ZERO, "DIO should be 0 when COGS is zero");
        assert_eq!(m.dpo, Decimal::ZERO, "DPO should be 0 when COGS is zero");
    }

    #[test]
    fn test_edge_single_period_no_trend() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let trend = &result.result.trend_analysis;
        assert_eq!(trend.dso_trend, "Stable");
        assert_eq!(trend.dio_trend, "Stable");
        assert_eq!(trend.dpo_trend, "Stable");
        assert_eq!(trend.ccc_trend, "Stable");
        assert_eq!(trend.dso_change, Decimal::ZERO);
        assert_eq!(trend.ccc_change, Decimal::ZERO);
    }

    #[test]
    fn test_nwc_as_pct_revenue() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        let m = &result.result.period_metrics[0];
        // NWC = 190_000, Revenue = 1_000_000 => 0.19
        let expected = dec!(190_000) / dec!(1_000_000);
        assert_eq!(
            m.nwc_as_pct_revenue, expected,
            "NWC as % of revenue mismatch"
        );
    }

    // -- Additional edge case and structural tests ---------------------------

    #[test]
    fn test_historical_rows_marked_not_forecast() {
        let input = sample_forecast_input();
        let result = build_rolling_forecast(&input).unwrap();
        for row in &result.result.historical {
            assert!(
                !row.is_forecast,
                "Historical rows should have is_forecast=false"
            );
        }
        for row in &result.result.forecast {
            assert!(
                row.is_forecast,
                "Forecast rows should have is_forecast=true"
            );
        }
    }

    #[test]
    fn test_terminal_revenue() {
        let input = sample_forecast_input();
        let result = build_rolling_forecast(&input).unwrap();
        let terminal = result.result.summary.terminal_revenue;
        let last_forecast_rev = result.result.forecast.last().unwrap().revenue;
        assert_eq!(
            terminal, last_forecast_rev,
            "Terminal revenue should match last forecast period"
        );
    }

    #[test]
    fn test_methodology_string_wc() {
        let input = sample_wc_input();
        let result = analyze_working_capital(&input).unwrap();
        assert_eq!(
            result.methodology,
            "Working Capital Analysis (DSO/DIO/DPO/CCC)"
        );
    }

    #[test]
    fn test_methodology_string_forecast() {
        let input = sample_forecast_input();
        let result = build_rolling_forecast(&input).unwrap();
        assert_eq!(result.methodology, "Rolling Financial Forecast");
    }
}
