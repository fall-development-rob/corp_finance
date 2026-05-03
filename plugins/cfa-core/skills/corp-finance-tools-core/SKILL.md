---
name: Corp Finance Tools - Core
description: Use the corp-finance-mcp server tools for core corporate finance calculations. Invoke when performing valuations (DCF, WACC, comps), credit analysis (metrics, debt capacity, covenants, Altman Z-score), PE/M&A (LBO models, IRR, MOIC, debt schedules, waterfall distributions, merger accretion/dilution), portfolio analytics (Sharpe, VaR, Kelly), fund economics (fee calculator, GP/LP splits, GP economics, investor net returns), jurisdiction (GAAP/IFRS reconciliation, withholding tax, NAV with equalisation, UBTI/ECI screening), three-statement financial modelling, Monte Carlo simulation (DCF, generic), scenario/sensitivity analysis, earnings quality (Beneish M-Score, Piotroski F-Score, accrual quality, revenue quality, composite scoring), dividend policy (H-Model DDM, multi-stage DDM, buyback analysis, payout sustainability, total shareholder return), financial forensics (Benford's Law, DuPont analysis, Z-score models, peer benchmarking, red flag scoring). All computation uses 128-bit decimal precision.
requires_tools:
  - altman_zscore
  - analyze_benfords_law
  - analyze_buyback
  - analyze_merger
  - analyze_payout_sustainability
  - build_debt_schedule
  - build_lbo
  - build_sensitivity_grid
  - build_three_statement
  - calculate_accrual_quality
  - calculate_beneish_mscore
  - calculate_dupont
  - calculate_earnings_quality_composite
  - calculate_fund_fees
  - calculate_gp_economics
  - calculate_h_model_ddm
  - calculate_investor_net_returns
  - calculate_multistage_ddm
  - calculate_nav
  - calculate_peer_benchmarking
  - calculate_piotroski_fscore
  - calculate_portfolio_wht
  - calculate_red_flag_scoring
  - calculate_returns
  - calculate_revenue_quality
  - calculate_total_shareholder_return
  - calculate_waterfall
  - calculate_wht
  - calculate_zscore_models
  - comps_analysis
  - covenant_compliance
  - credit_metrics
  - dcf_model
  - debt_capacity
  - kelly_sizing
  - reconcile_accounting
  - risk_adjusted_returns
  - risk_metrics
  - run_mc_dcf
  - run_monte_carlo
  - screen_ubti_eci
  - sources_and_uses
  - wacc_calculator
---
# Corp Finance Tools - Core

You have access to 44 core corporate finance MCP tools for fundamental valuation, credit, PE/M&A, portfolio, fund economics, jurisdiction, three-statement modelling, Monte Carlo, scenario analysis, earnings quality, dividend policy, and financial forensics. All tools return structured JSON with `result`, `methodology`, `assumptions`, `warnings`, and `metadata` fields. All monetary math uses `rust_decimal` (128-bit fixed-point) — never floating-point (except Monte Carlo which uses f64 for performance).

## Tool Reference

### Valuation

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `wacc_calculator` | CAPM-based WACC | risk_free_rate, equity_risk_premium, beta, cost_of_debt, tax_rate, debt_weight, equity_weight |
| `dcf_model` | FCFF discounted cash flow | base_revenue, revenue_growth_rates, ebitda_margin, wacc, terminal_method, terminal_growth_rate |
| `comps_analysis` | Trading comparables | target metrics, comparable companies, multiple types (EV/EBITDA, P/E, etc.) |

### Credit

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `credit_metrics` | Full credit ratio suite + synthetic rating | revenue, ebitda, ebit, interest_expense, total_debt, cash, and 10+ balance sheet items |
| `debt_capacity` | Maximum debt sizing from constraints | ebitda, interest_rate, max_leverage, min_interest_coverage, min_dscr, min_ffo_to_debt |
| `covenant_compliance` | Test actuals vs covenant thresholds | covenants (metric, threshold, direction), actuals (CreditMetricsOutput) |
| `altman_zscore` | Altman Z-Score bankruptcy prediction | working_capital, total_assets, retained_earnings, ebit, revenue, total_liabilities, market_cap, book_equity, is_public, is_manufacturing |

### Private Equity

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `calculate_returns` | IRR, XIRR, MOIC, Cash-on-Cash | entry_equity, exit_equity, cash_flows, dated_cash_flows |
| `build_debt_schedule` | Multi-tranche amortisation | name, amount, interest_rate, amortisation type, maturity_years, PIK, seniority |
| `sources_and_uses` | Transaction financing summary | enterprise_value, equity_contribution, debt tranches, fees |
| `build_lbo` | Full LBO with multi-tranche debt | entry_ev, entry_ebitda, tranches, equity, revenue_growth, ebitda_margin, exit_year, exit_multiple, cash_sweep_pct |
| `calculate_waterfall` | GP/LP distribution waterfall | total_proceeds, total_invested, tiers (ROC, pref, catch-up, carry), gp_commitment_pct |

### M&A

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_merger` | Accretion/dilution analysis | acquirer/target financials, offer_price, consideration type (cash/stock/mixed), synergies, financing rates |

### Fund Economics & Jurisdiction

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `calculate_fund_fees` | Fund fee modelling + LP net returns | fund_size, mgmt_fee_rate, perf_fee_rate, hurdle, catch_up, waterfall_type (European/American), gp_commitment, fund_life |
| `reconcile_accounting` | GAAP/IFRS accounting reconciliation | source/target standard, revenue, ebitda, total_assets, lease payments, lifo_reserve, dev costs, revaluation surplus |
| `calculate_wht` | Withholding tax with treaty rates | source/investor jurisdiction, income_type, gross_income, is_tax_exempt |
| `calculate_portfolio_wht` | Portfolio-level WHT analysis | holdings array (each with jurisdiction, income_type, gross_income) |
| `calculate_nav` | NAV with equalisation & multi-class | share_classes (per-class HWM, fees, crystallisation), gross_return, equalisation_method |
| `calculate_gp_economics` | GP economics: fees, carry, break-even | fund_size, fee_rates, carry_rate, hurdle, gp_commitment, fund_life, professionals |
| `calculate_investor_net_returns` | Gross-to-net after all fees/WHT/blocker | gross_moic, gross_irr, holding_period, fee_rates, wht_rate, blocker_cost |
| `screen_ubti_eci` | UBTI/ECI income classification | investor_type, vehicle_structure, income_items, has_debt_financing |

### Portfolio Analytics

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `risk_adjusted_returns` | Sharpe, Sortino, Calmar, IR, Treynor | returns series, frequency, risk_free_rate, benchmark_returns |
| `risk_metrics` | VaR, CVaR, drawdown, skewness, kurtosis | returns series, confidence_level, frequency |
| `kelly_sizing` | Kelly criterion position sizing | win_probability, win_loss_ratio, kelly_fraction, max_position_pct |

### Scenarios

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `build_sensitivity_grid` | 2-way sensitivity grid + Bear/Base/Bull scenario weighting | model, variable_1, variable_2, base_inputs (or scenarios with name/probability/overrides) |

### Three-Statement Model

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `build_three_statement` | Linked 3-statement financial projection (IS/BS/CF) | base_revenue, revenue_growth_rates, cost percentages, working capital days, capex_pct, base balance sheet items |

### Monte Carlo

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `run_monte_carlo` | Generic MC simulation with statistical output | variables (name, distribution), num_simulations, seed |
| `run_mc_dcf` | Stochastic DCF valuation with confidence intervals | base_fcf, projection_years, distributions for growth/margin/wacc/terminal_growth |

### Earnings Quality

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `calculate_beneish_mscore` | Beneish M-Score: 8-variable earnings manipulation model | dsri, gmi, aqi, sgi, depi, sgai, lvgi, tata (or raw financials for 2 periods to compute indices) |
| `calculate_piotroski_fscore` | Piotroski F-Score: 9-signal fundamental strength score | net_income, operating_cash_flow, roa, roa_prior, leverage, leverage_prior, current_ratio, current_ratio_prior, shares_outstanding, shares_prior, gross_margin, gross_margin_prior, asset_turnover, asset_turnover_prior |
| `calculate_accrual_quality` | Accrual quality analysis: Sloan ratio, Dechow-Dichev, Jones, modified Jones | total_accruals, cash_from_operations, net_income, total_assets, delta_revenue, ppe, delta_receivables, prior period data |
| `calculate_revenue_quality` | Revenue quality assessment: receivables, deferred revenue, concentration | revenue, receivables, revenue_prior, receivables_prior, deferred_revenue, deferred_revenue_prior, allowance, segment_revenues |
| `calculate_earnings_quality_composite` | Composite earnings quality score with traffic-light rating | beneish_mscore_output, piotroski_fscore_output, accrual_quality_output, revenue_quality_output, weights (optional) |

### Dividend Policy

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `calculate_h_model_ddm` | H-Model DDM: Fuller & Hsia declining growth valuation | current_dividend, short_term_growth, long_term_growth, half_life_years, required_return |
| `calculate_multistage_ddm` | Multi-stage DDM: N-stage with terminal Gordon Growth | current_dividend, growth_stages (rate, years per stage), terminal_growth, required_return |
| `analyze_buyback` | Share buyback analysis: EPS accretion/dilution, P/E breakeven | shares_outstanding, current_eps, share_price, buyback_amount, funding_source, cost_of_debt, tax_rate, dividend_per_share |
| `analyze_payout_sustainability` | Payout sustainability: payout ratio, FCF coverage, Lintner smoothing | dividends_paid, net_income, free_cash_flow, total_debt, ebitda, prior_dividend, target_payout_ratio, speed_of_adjustment |
| `calculate_total_shareholder_return` | Total shareholder return: price + dividend + buyback attribution | price_begin, price_end, dividends_per_share, shares_repurchased, shares_outstanding, period_years |

### Financial Forensics

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_benfords_law` | Benford's Law: digit distribution conformity testing | data_series, test_type (first_digit, second_digit, first_two_digits), significance_level |
| `calculate_dupont` | DuPont decomposition: 3-step and 5-step ROE breakdown | net_income, revenue, total_assets, total_equity, interest_expense, pretax_income, tax_expense, prior_period (optional for trend) |
| `calculate_zscore_models` | Z-Score models: Altman, Ohlson, Zmijewski, Springate | working_capital, total_assets, retained_earnings, ebit, revenue, total_liabilities, market_cap, book_equity, net_income, current_assets, current_liabilities, total_debt, cash_from_operations, is_public, is_manufacturing |
| `calculate_peer_benchmarking` | Peer benchmarking: percentile ranking, z-score normalisation | target_metrics, peer_metrics (array of peer company metrics), metric_definitions (name, direction: higher_better/lower_better) |
| `calculate_red_flag_scoring` | Red flag scoring: composite fraud/distress risk assessment | beneish_output, altman_output, piotroski_output, financial_ratios, audit_indicators (auditor_changes, restatements, late_filings, going_concern) |

---

## Response Envelope

Every tool returns this structure:

```json
{
  "result": { },
  "methodology": "DCF (FCFF, 2-stage)",
  "assumptions": { },
  "warnings": ["Terminal growth (3.5%) above long-term GDP"],
  "metadata": {
    "version": "0.1.0",
    "computation_time_us": 1200,
    "precision": "rust_decimal_128bit"
  }
}
```

Always check `warnings` — they flag suspicious inputs (beta > 3, ERP > 10%, WACC > 20%, too few comps, etc.).

---

## Tool Chaining Workflows

### Full Valuation

1. `wacc_calculator` — compute discount rate
2. `dcf_model` — build DCF using that WACC (or pass `wacc_input` directly)
3. `comps_analysis` — cross-check with trading multiples
4. `build_sensitivity_grid` — vary WACC and terminal growth to show range

### Credit Assessment

1. `credit_metrics` — compute all leverage, coverage, cash flow, and liquidity ratios
2. `debt_capacity` — size maximum debt from constraint analysis
3. `covenant_compliance` — test actuals against loan covenants

### LBO Deal Analysis

1. `build_lbo` — full LBO with projections, debt service, cash sweep, exit returns
2. Or build manually: `sources_and_uses` → `build_debt_schedule` → `calculate_returns`
3. `build_sensitivity_grid` — sensitivity on exit multiple vs EBITDA
4. `altman_zscore` — check bankruptcy risk at entry leverage

### Merger Analysis

1. `analyze_merger` — accretion/dilution with consideration structure and synergies
2. `build_sensitivity_grid` — vary synergies vs offer premium
3. `credit_metrics` — assess combined entity credit profile

### Waterfall Distribution

1. `calculate_waterfall` — GP/LP splits with hurdle, catch-up, carry
2. `calculate_fund_fees` — full fund economics over fund life

### Credit Assessment

1. `credit_metrics` — compute all leverage, coverage, cash flow, and liquidity ratios
2. `altman_zscore` — bankruptcy prediction (Z, Z', Z'' variants)
3. `debt_capacity` — size maximum debt from constraint analysis
4. `covenant_compliance` — test actuals against loan covenants

### Portfolio Review

1. `risk_adjusted_returns` — Sharpe, Sortino, and peer-relative metrics
2. `risk_metrics` — VaR, CVaR, drawdown profile
3. `kelly_sizing` — optimal position sizing
4. `build_sensitivity_grid` — stress test across bear/base/bull

### GAAP/IFRS Reconciliation

1. `reconcile_accounting` — reconcile between US GAAP and IFRS
   - Adjustments: lease capitalisation (IFRS 16), LIFO→FIFO, dev cost capitalisation (IAS 38), revaluation strip
   - Returns adjusted EBITDA, EBIT, net income, debt, equity, assets + materiality flag

### Withholding Tax Analysis

1. `calculate_wht` — single holding WHT with treaty rate lookup
2. `calculate_portfolio_wht` — portfolio-level WHT with optimisation suggestions
   - Covers 15+ jurisdictions, 10+ bilateral tax treaties
   - Provides blocker recommendations for tax-neutral investors

### NAV & Fund Administration

1. `calculate_nav` — multi-class NAV with equalisation
   - Per-class: management fee accrual, performance fee (HWM-based), net NAV, FX conversion
   - Equalisation methods: equalisation shares, series accounting, depreciation deposit
   - Crystallisation: monthly, quarterly, semi-annual, annual, on redemption

### GP Economics & Investor Returns

1. `calculate_gp_economics` — GP revenue decomposition over fund life
   - Management fees, carried interest, co-invest returns, breakeven AUM
   - Per-professional economics, fee holiday, successor fund offset
2. `calculate_investor_net_returns` — gross-to-net after all fee layers
   - Management fees, carry, fund expenses, WHT, blocker cost, org costs
   - Fee drag in bps, fee breakdown as % of gross

### UBTI/ECI Screening

1. `screen_ubti_eci` — classify income for US tax-exempt investors
   - Classifies: interest, dividend, capital gain, rental, operating business, partnership, royalty, CFC
   - Risk assessment: None/Low/Medium/High
   - Blocker analysis: cost-benefit of US C-corp blocker (21% corp vs 37% trust rate)

### Three-Statement Financial Modelling

1. `build_three_statement` — build linked IS/BS/CF projections
   - Revenue growth, cost structure, working capital (DSO/DIO/DPO), capex, debt service
   - Circular reference resolution (5-iteration convergence on interest expense)
   - Revolver draw / excess cash paydown logic
   - Warnings: leverage > 6x, interest coverage < 2x, negative FCF

### Monte Carlo Simulation

1. `run_monte_carlo` — generic MC with configurable distributions
   - Normal, LogNormal, Triangular, Uniform distributions
   - Returns: mean, median, percentiles (P5-P95), histogram, skewness, kurtosis
   - Reproducible with optional seed
2. `run_mc_dcf` — stochastic DCF valuation
   - Vary revenue growth, EBITDA margin, WACC, terminal growth simultaneously
   - Returns: EV percentiles, 90% confidence interval, probability above thresholds

### Earnings Quality Assessment

1. `calculate_beneish_mscore` — compute M-Score for earnings manipulation detection
   - 8 variables: DSRI, GMI, AQI, SGI, DEPI, SGAI, LVGI, TATA
   - M-Score > -1.78 suggests possible manipulation
2. `calculate_piotroski_fscore` — assess fundamental strength (0-9)
   - Profitability (4 signals), leverage (3 signals), operating efficiency (2 signals)
   - F-Score >= 8 = strong fundamentals; <= 2 = weak
3. `calculate_accrual_quality` — analyse earnings persistence
   - Sloan ratio, Dechow-Dichev, Jones model for discretionary accruals
4. `calculate_revenue_quality` — assess revenue recognition quality
   - Receivables vs revenue growth, deferred revenue trends, concentration
5. `calculate_earnings_quality_composite` — weighted composite score with traffic-light rating

### Dividend Policy Analysis

1. `calculate_h_model_ddm` — H-Model for declining growth assumptions
   - Fuller & Hsia: V = D0(1+gL)/(r-gL) + D0*H*(gS-gL)/(r-gL)
2. `calculate_multistage_ddm` — explicit growth periods + terminal value
   - N-stage with Gordon Growth terminal value
3. `analyze_buyback` — compare buyback vs dividend alternatives
   - EPS accretion/dilution, P/E breakeven, tax efficiency
4. `analyze_payout_sustainability` — assess dividend safety
   - Payout ratio, FCF coverage, Lintner smoothing model
5. `calculate_total_shareholder_return` — component attribution of TSR
   - Price + dividend yield + buyback yield

### Financial Forensics Workflow

1. `analyze_benfords_law` — test data for manipulation indicators
   - First/second/first-two digit distribution vs Benford expected
   - Chi-squared and MAD conformity tests
2. `calculate_dupont` — decompose ROE drivers (3-way and 5-way)
   - 3-way: margin x turnover x leverage
   - 5-way: tax burden x interest burden x operating margin x turnover x leverage
   - Trend analysis with prior period comparison
3. `calculate_zscore_models` — comprehensive distress scoring
   - 5 models: Altman (original/revised/private), Ohlson O-Score, Zmijewski, Springate
   - Composite weighted distress score
4. `calculate_peer_benchmarking` — relative performance analysis
   - Percentile ranking, z-score normalization across peer group
5. `calculate_red_flag_scoring` — composite fraud/distress risk assessment
   - Integrates Beneish, Altman, Piotroski, financial ratios, audit indicators

---

## CLI Equivalent

The same calculations are available via the `cfa` binary:

```bash
cfa wacc --risk-free-rate 0.04 --equity-risk-premium 0.055 --beta 1.2 \
         --cost-of-debt 0.06 --tax-rate 0.25 --debt-weight 0.3 --equity-weight 0.7

cfa credit-metrics --input financials.json --output table

cfa returns --entry-equity 50000000 --exit-equity 140000000 --output json

cfa sensitivity --model wacc --var1 beta:0.8:1.6:0.1 \
                --var2 equity_risk_premium:0.04:0.07:0.005 --input base.json

cfa lbo --input deal.json --output table

cfa waterfall --input distribution.json

cfa merger --input merger.json

cfa altman-zscore --input financials.json --output table

cfa fund-fees --input fund.json --output table

cfa gaap-ifrs --input reconciliation.json --output table

cfa wht --input wht.json --output json

cfa nav --input nav.json --output table

cfa gp-economics --input gp.json --output table

cfa investor-net-returns --input investor.json --output json

cfa ubti-screening --input ubti.json --output table

cfa three-statement --input model.json --output table

cfa monte-carlo --input mc.json --output json

cfa mc-dcf --input mc_dcf.json --output json

cfa beneish --input mscore.json --output table

cfa piotroski --input fscore.json --output table

cfa accrual-quality --input accrual.json --output json

cfa revenue-quality --input revenue_quality.json --output table

cfa earnings-quality-composite --input eq_composite.json --output json

cfa h-model-ddm --input ddm.json --output table

cfa multistage-ddm --input ddm_multi.json --output json

cfa buyback --input buyback.json --output table

cfa payout-sustainability --input payout.json --output json

cfa total-shareholder-return --input tsr.json --output table

cfa benfords-law --input benfords.json --output table

cfa dupont-analysis --input dupont.json --output json

cfa zscore-models --input zscore.json --output table

cfa peer-benchmarking --input peers.json --output json

cfa red-flag-scoring --input redflags.json --output table
```

Output formats: `--output json` (default), `--output table`, `--output csv`, `--output minimal`.

Pipe support: `cat data.json | cfa credit-metrics --output table`

---

## Input Conventions

- **Rates as decimals**: 5% = `0.05`, never `5`
- **Money as raw numbers**: $1M = `1000000`, not `"$1M"`
- **Currency**: specify with `currency` field (default: USD)
- **Dates**: ISO 8601 format (`"2026-01-15"`)
- **Weights must sum to 1.0**: `debt_weight + equity_weight = 1.0`

## Error Handling

Tools return structured errors for:
- **InvalidInput**: field-level validation (e.g., negative beta, weights not summing to 1.0)
- **FinancialImpossibility**: terminal growth >= WACC, negative enterprise value
- **ConvergenceFailure**: IRR/XIRR Newton-Raphson didn't converge (reports iterations and last delta)
- **InsufficientData**: too few data points for statistical calculations
- **DivisionByZero**: zero interest expense for coverage ratios, etc.

Always validate tool error responses and report them clearly to the user.
