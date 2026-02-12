---
name: "Corp Finance Tools - Core"
description: "Use the corp-finance-mcp server tools for core corporate finance calculations. Invoke when performing valuations (DCF, WACC, comps), credit analysis (metrics, debt capacity, covenants, Altman Z-score), PE/M&A (LBO models, IRR, MOIC, debt schedules, waterfall distributions, merger accretion/dilution), portfolio analytics (Sharpe, VaR, Kelly), fund economics (fee calculator, GP/LP splits, GP economics, investor net returns), jurisdiction (GAAP/IFRS reconciliation, withholding tax, NAV with equalisation, UBTI/ECI screening), three-statement financial modelling, Monte Carlo simulation (DCF, generic), scenario/sensitivity analysis. All computation uses 128-bit decimal precision."
---

# Corp Finance Tools - Core

You have access to 29 core corporate finance MCP tools for fundamental valuation, credit, PE/M&A, portfolio, fund economics, jurisdiction, three-statement modelling, Monte Carlo, and scenario analysis. All tools return structured JSON with `result`, `methodology`, `assumptions`, `warnings`, and `metadata` fields. All monetary math uses `rust_decimal` (128-bit fixed-point) — never floating-point (except Monte Carlo which uses f64 for performance).

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
| `returns_calculator` | IRR, XIRR, MOIC, Cash-on-Cash | entry_equity, exit_equity, cash_flows, dated_cash_flows |
| `debt_schedule` | Multi-tranche amortisation | name, amount, interest_rate, amortisation type, maturity_years, PIK, seniority |
| `sources_uses` | Transaction financing summary | enterprise_value, equity_contribution, debt tranches, fees |
| `lbo_model` | Full LBO with multi-tranche debt | entry_ev, entry_ebitda, tranches, equity, revenue_growth, ebitda_margin, exit_year, exit_multiple, cash_sweep_pct |
| `waterfall_calculator` | GP/LP distribution waterfall | total_proceeds, total_invested, tiers (ROC, pref, catch-up, carry), gp_commitment_pct |

### M&A

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `merger_model` | Accretion/dilution analysis | acquirer/target financials, offer_price, consideration type (cash/stock/mixed), synergies, financing rates |

### Fund Economics & Jurisdiction

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fund_fee_calculator` | Fund fee modelling + LP net returns | fund_size, mgmt_fee_rate, perf_fee_rate, hurdle, catch_up, waterfall_type (European/American), gp_commitment, fund_life |
| `gaap_ifrs_reconciliation` | GAAP/IFRS accounting reconciliation | source/target standard, revenue, ebitda, total_assets, lease payments, lifo_reserve, dev costs, revaluation surplus |
| `withholding_tax_calculator` | Withholding tax with treaty rates | source/investor jurisdiction, income_type, gross_income, is_tax_exempt |
| `portfolio_wht_calculator` | Portfolio-level WHT analysis | holdings array (each with jurisdiction, income_type, gross_income) |
| `nav_calculator` | NAV with equalisation & multi-class | share_classes (per-class HWM, fees, crystallisation), gross_return, equalisation_method |
| `gp_economics_model` | GP economics: fees, carry, break-even | fund_size, fee_rates, carry_rate, hurdle, gp_commitment, fund_life, professionals |
| `investor_net_returns` | Gross-to-net after all fees/WHT/blocker | gross_moic, gross_irr, holding_period, fee_rates, wht_rate, blocker_cost |
| `ubti_eci_screening` | UBTI/ECI income classification | investor_type, vehicle_structure, income_items, has_debt_financing |

### Portfolio Analytics

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `risk_adjusted_returns` | Sharpe, Sortino, Calmar, IR, Treynor | returns series, frequency, risk_free_rate, benchmark_returns |
| `risk_metrics` | VaR, CVaR, drawdown, skewness, kurtosis | returns series, confidence_level, frequency |
| `kelly_sizing` | Kelly criterion position sizing | win_probability, win_loss_ratio, kelly_fraction, max_position_pct |

### Scenarios

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `sensitivity_matrix` | 2-way sensitivity grid | model, variable_1, variable_2, base_inputs |
| `scenario_analysis` | Bear/Base/Bull with probability weights | scenarios (name, probability, overrides), output_values, base_case_value |

### Three-Statement Model

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `three_statement_model` | Linked 3-statement financial projection (IS/BS/CF) | base_revenue, revenue_growth_rates, cost percentages, working capital days, capex_pct, base balance sheet items |

### Monte Carlo

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `monte_carlo_simulation` | Generic MC simulation with statistical output | variables (name, distribution), num_simulations, seed |
| `monte_carlo_dcf` | Stochastic DCF valuation with confidence intervals | base_fcf, projection_years, distributions for growth/margin/wacc/terminal_growth |

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
4. `sensitivity_matrix` — vary WACC and terminal growth to show range

### Credit Assessment

1. `credit_metrics` — compute all leverage, coverage, cash flow, and liquidity ratios
2. `debt_capacity` — size maximum debt from constraint analysis
3. `covenant_compliance` — test actuals against loan covenants

### LBO Deal Analysis

1. `lbo_model` — full LBO with projections, debt service, cash sweep, exit returns
2. Or build manually: `sources_uses` → `debt_schedule` → `returns_calculator`
3. `sensitivity_matrix` — sensitivity on exit multiple vs EBITDA
4. `altman_zscore` — check bankruptcy risk at entry leverage

### Merger Analysis

1. `merger_model` — accretion/dilution with consideration structure and synergies
2. `sensitivity_matrix` — vary synergies vs offer premium
3. `credit_metrics` — assess combined entity credit profile

### Waterfall Distribution

1. `waterfall_calculator` — GP/LP splits with hurdle, catch-up, carry
2. `fund_fee_calculator` — full fund economics over fund life

### Credit Assessment

1. `credit_metrics` — compute all leverage, coverage, cash flow, and liquidity ratios
2. `altman_zscore` — bankruptcy prediction (Z, Z', Z'' variants)
3. `debt_capacity` — size maximum debt from constraint analysis
4. `covenant_compliance` — test actuals against loan covenants

### Portfolio Review

1. `risk_adjusted_returns` — Sharpe, Sortino, and peer-relative metrics
2. `risk_metrics` — VaR, CVaR, drawdown profile
3. `kelly_sizing` — optimal position sizing
4. `scenario_analysis` — stress test across bear/base/bull

### GAAP/IFRS Reconciliation

1. `gaap_ifrs_reconciliation` — reconcile between US GAAP and IFRS
   - Adjustments: lease capitalisation (IFRS 16), LIFO→FIFO, dev cost capitalisation (IAS 38), revaluation strip
   - Returns adjusted EBITDA, EBIT, net income, debt, equity, assets + materiality flag

### Withholding Tax Analysis

1. `withholding_tax_calculator` — single holding WHT with treaty rate lookup
2. `portfolio_wht_calculator` — portfolio-level WHT with optimisation suggestions
   - Covers 15+ jurisdictions, 10+ bilateral tax treaties
   - Provides blocker recommendations for tax-neutral investors

### NAV & Fund Administration

1. `nav_calculator` — multi-class NAV with equalisation
   - Per-class: management fee accrual, performance fee (HWM-based), net NAV, FX conversion
   - Equalisation methods: equalisation shares, series accounting, depreciation deposit
   - Crystallisation: monthly, quarterly, semi-annual, annual, on redemption

### GP Economics & Investor Returns

1. `gp_economics_model` — GP revenue decomposition over fund life
   - Management fees, carried interest, co-invest returns, breakeven AUM
   - Per-professional economics, fee holiday, successor fund offset
2. `investor_net_returns` — gross-to-net after all fee layers
   - Management fees, carry, fund expenses, WHT, blocker cost, org costs
   - Fee drag in bps, fee breakdown as % of gross

### UBTI/ECI Screening

1. `ubti_eci_screening` — classify income for US tax-exempt investors
   - Classifies: interest, dividend, capital gain, rental, operating business, partnership, royalty, CFC
   - Risk assessment: None/Low/Medium/High
   - Blocker analysis: cost-benefit of US C-corp blocker (21% corp vs 37% trust rate)

### Three-Statement Financial Modelling

1. `three_statement_model` — build linked IS/BS/CF projections
   - Revenue growth, cost structure, working capital (DSO/DIO/DPO), capex, debt service
   - Circular reference resolution (5-iteration convergence on interest expense)
   - Revolver draw / excess cash paydown logic
   - Warnings: leverage > 6x, interest coverage < 2x, negative FCF

### Monte Carlo Simulation

1. `monte_carlo_simulation` — generic MC with configurable distributions
   - Normal, LogNormal, Triangular, Uniform distributions
   - Returns: mean, median, percentiles (P5-P95), histogram, skewness, kurtosis
   - Reproducible with optional seed
2. `monte_carlo_dcf` — stochastic DCF valuation
   - Vary revenue growth, EBITDA margin, WACC, terminal growth simultaneously
   - Returns: EV percentiles, 90% confidence interval, probability above thresholds

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
