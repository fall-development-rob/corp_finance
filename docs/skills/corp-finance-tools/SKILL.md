---
name: "Corp Finance MCP Tools"
description: "Use the corp-finance-mcp server tools for institutional-grade financial calculations. Invoke when performing valuations (DCF, WACC, comps), credit analysis (metrics, debt capacity, covenants, Altman Z-score), PE/M&A (LBO models, IRR, MOIC, debt schedules, waterfall distributions, merger accretion/dilution), portfolio analytics (Sharpe, VaR, Kelly), fund economics (fee calculator, GP/LP splits, GP economics, investor net returns), jurisdiction (GAAP/IFRS reconciliation, withholding tax, NAV with equalisation, UBTI/ECI screening), fixed income (bond pricing, yield analysis, duration/convexity, credit spreads, yield curve bootstrapping, Nelson-Siegel fitting), derivatives (option pricing, implied volatility, forwards/futures, interest rate swaps, currency swaps, option strategies), three-statement financial modelling, Monte Carlo simulation (DCF, generic), quantitative risk (factor models, Black-Litterman, risk parity, stress testing), restructuring (recovery analysis, distressed debt), real assets (property valuation, project finance), FX (forwards, cross rates), commodities (forwards, term structure), scenario/sensitivity analysis, securitization (ABS/MBS, CDO tranching), venture capital (dilution, convertibles, fund returns), ESG (scoring, climate/carbon, green bonds, SLL), regulatory (Basel III capital, LCR/NSFR, ALM), private credit (unitranche, direct lending, syndication), insurance (loss reserving, premium pricing, combined ratio, Solvency II SCR), FP&A (variance analysis, break-even, working capital, rolling forecast), wealth management (retirement planning, tax-loss harvesting, estate planning). All computation uses 128-bit decimal precision."
---

# Corp Finance MCP Tools

You have access to 83 MCP tools for corporate finance computation. All tools return structured JSON with `result`, `methodology`, `assumptions`, `warnings`, and `metadata` fields. All monetary math uses `rust_decimal` (128-bit fixed-point) — never floating-point (except Monte Carlo which uses f64 for performance).

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

### Fixed Income

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `bond_pricer` | Bond pricing — clean/dirty price, accrued interest, day count conventions | face_value, coupon_rate, coupon_frequency, ytm, settlement_date, maturity_date, day_count |
| `bond_yield` | Bond yield calculator — YTM, BEY, effective annual yield | face_value, coupon_rate, coupon_frequency, market_price, years_to_maturity |
| `bootstrap_spot_curve` | Bootstrap spot rate curve from par instruments | par_instruments (maturity_years, par_rate, coupon_frequency) |
| `nelson_siegel_fit` | Nelson-Siegel yield curve fitting | observed_rates (maturity, rate), initial_lambda |
| `bond_duration` | Duration & convexity — Macaulay, modified, effective, DV01, key rate | face_value, coupon_rate, coupon_frequency, ytm, years_to_maturity |
| `credit_spreads` | Credit spread analysis — Z-spread, OAS, I-spread, G-spread | face_value, coupon_rate, market_price, years_to_maturity, benchmark_curve |

### Derivatives

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `option_pricer` | Option pricing — Black-Scholes, binomial, Greeks | spot_price, strike_price, time_to_expiry, risk_free_rate, volatility, option_type, exercise_style |
| `implied_volatility` | Implied volatility solver from market price | spot_price, strike_price, time_to_expiry, risk_free_rate, market_price, option_type |
| `forward_pricer` | Forward/futures pricing with cost of carry | spot_price, risk_free_rate, time_to_expiry, underlying_type, storage/dividend/convenience |
| `forward_position_value` | Mark-to-market existing forward position | original_forward_price, current_spot, risk_free_rate, remaining_time, is_long |
| `futures_basis_analysis` | Futures term structure and basis analysis | spot_price, futures_prices, risk_free_rate |
| `interest_rate_swap` | IRS valuation — fixed/floating legs, par rate, DV01 | notional, fixed_rate, payment_frequency, remaining_years, discount_curve |
| `currency_swap` | Cross-currency swap valuation | notional_domestic/foreign, rates, discount_curves, spot_fx_rate |
| `option_strategy` | Option strategy payoff analysis — 12 strategy types | strategy_type, underlying_price, legs |

### Three-Statement Model

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `three_statement_model` | Linked 3-statement financial projection (IS/BS/CF) | base_revenue, revenue_growth_rates, cost percentages, working capital days, capex_pct, base balance sheet items |

### Monte Carlo

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `monte_carlo_simulation` | Generic MC simulation with statistical output | variables (name, distribution), num_simulations, seed |
| `monte_carlo_dcf` | Stochastic DCF valuation with confidence intervals | base_fcf, projection_years, distributions for growth/margin/wacc/terminal_growth |

### Quantitative Risk

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `factor_model` | Multi-factor regression (CAPM, FF3, Carhart4, Custom) | asset_returns, factor_returns (MKT, SMB, HML, MOM), model_type |
| `black_litterman` | Black-Litterman portfolio optimisation with views | market_cap_weights, covariance_matrix, views (absolute/relative), risk_aversion, tau |
| `risk_parity` | Risk parity portfolio construction | assets, covariance_matrix, method (InverseVol/ERC/MinVariance), target_volatility |
| `stress_test` | Multi-scenario stress testing with 5 built-in historical | portfolio positions, scenarios (or use built-in GFC/COVID/etc.), correlation_adjustments |

### Restructuring

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `recovery_analysis` | APR waterfall recovery by claim priority | enterprise_value, claims (priority, secured, collateral), DIP facility, admin costs |
| `distressed_debt_analysis` | Restructuring plan analysis with fulcrum ID | enterprise_value, exit_ev, capital_structure, proposed_treatments, DIP terms |

### Real Assets

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `property_valuation` | Real estate valuation (direct cap, DCF, GRM) | gross_rent, vacancy, opex, cap_rate, holding_period, financing terms, comparables |
| `project_finance_model` | Infrastructure project finance with debt sculpting | total_cost, construction/operating periods, revenue, debt (level/sculpted/bullet), DSCR target |

### FX & Commodities

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fx_forward` | FX forward pricing via covered interest parity | spot_rate, domestic/foreign rates, time_to_expiry, notional |
| `cross_rate` | Cross rate derivation from two currency pairs | rate1, rate1_pair, rate2, rate2_pair, target_pair |
| `commodity_forward` | Commodity forward pricing (cost-of-carry) | spot_price, risk_free_rate, storage_cost, convenience_yield, commodity_type |
| `commodity_curve` | Futures term structure analysis | spot_price, futures_prices, risk_free_rate, storage_cost |

### Securitization

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `abs_mbs_cashflows` | ABS/MBS pool cash flow projection with prepayment/default models | pool_balance, wac, wam, prepayment_model (CPR/PSA/SMM), default_model (CDR/SDA), loss_severity, recovery_lag |
| `cdo_tranching` | CDO/CLO tranching waterfall analysis | collateral_balance, cashflow_periods, tranches (name, balance, coupon, seniority), loss_scenarios, OC/IC triggers, reserve_account |

### Venture Capital

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `venture_dilution` | Pre/post-money dilution and cap table modelling | rounds (pre_money, investment, option_pool_pct), founders_shares |
| `convertible_instrument` | SAFE and convertible note conversion analysis | instrument_type (SAFE/Note), investment, valuation_cap, discount_rate, interest_rate |
| `venture_fund_returns` | VC fund return analytics (IRR, TVPI, DPI, J-curve) | fund_size, investments (amount, entry/exit year, exit_multiple), management_fee, carry_rate, hurdle |

### ESG & Climate

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `esg_score` | ESG scoring with sector-specific materiality weights | company, sector, environmental/social/governance pillar scores |
| `carbon_footprint` | Carbon footprint analysis (Scope 1/2/3) | scope1/2/3 emissions, revenue, sector benchmarks |
| `green_bond` | Green bond framework analysis | proceeds_allocation, eligible_categories, impact_metrics |
| `sll_covenants` | Sustainability-linked loan covenant testing | kpi_targets, actual_performance, margin_adjustment |

### Regulatory Capital

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `basel_capital` | Basel III capital adequacy (CET1, Tier1, Total) with SA risk weights | exposures (asset_class, rating, amount), operational_risk, capital_buffers |
| `lcr_nsfr` | Liquidity coverage ratio and net stable funding ratio | hqla_assets, cash_outflows/inflows, available/required_stable_funding |
| `alm_analysis` | Asset-liability management (gap, NII sensitivity, EVE) | assets/liabilities by repricing bucket, rate scenarios, beta pass-through |

### Private Credit

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `unitranche_pricing` | Unitranche FOLO split pricing and blended yield | total_commitment, first_out_pct, spreads, OID, fees, borrower metrics |
| `direct_lending` | Direct loan modelling (PIK, delayed draw, amortisation) | loan_amount, base_rate, spread, pik_rate, amort_schedule, maturity, credit metrics |
| `syndication_analysis` | Loan syndication allocation and arranger economics | facility_size, arranger_hold, syndicate_members, arrangement_fee |

### Insurance & Actuarial

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `loss_reserving` | Chain-ladder and Bornhuetter-Ferguson loss reserving | claims_triangle, method (ChainLadder/BF/Both), earned_premium, expected_loss_ratio, tail_factor |
| `premium_pricing` | Insurance premium pricing (freq x severity + loadings) | expected_frequency, expected_severity, expense_loading, profit_loading, trend_rates |
| `combined_ratio` | Multi-period combined ratio and operating ratio analysis | periods (premium, losses, expenses, investment_income) |
| `solvency_scr` | Solvency II Standard Formula SCR calculation | premium_reserve_risk, operational_risk, correlation_matrix, MCR_floor |

### FP&A

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `variance_analysis` | Budget vs actual variance with price/volume/mix decomposition | budget/actual revenue_lines, cost_lines, prior_period (optional) |
| `breakeven_analysis` | Break-even, DOL, and target volume analysis | selling_price, variable_cost_per_unit, fixed_costs, current_volume, scenarios |
| `working_capital` | Working capital efficiency (DSO/DIO/DPO/CCC) and benchmarking | periods (revenue, cogs, receivables, inventory, payables), cost_of_capital |
| `rolling_forecast` | Rolling financial forecast with driver-based projections | historical_periods, forecast_periods, revenue_growth_rate, driver_overrides |

### Wealth Management

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `retirement_planning` | Retirement planning with 4 withdrawal strategies | current_age, retirement_age, life_expectancy, income, savings, withdrawal_strategy |
| `tax_loss_harvesting` | Tax-loss harvesting simulation with wash-sale rules | positions (cost_basis, market_value, holding_days), realized_gains, tax_rates |
| `estate_planning` | Estate tax planning with trust analysis and gifting strategy | estate_value, gifts, trusts, life_insurance, exemption, tax_rates |

### Scenarios

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `sensitivity_matrix` | 2-way sensitivity grid | model, variable_1, variable_2, base_inputs |
| `scenario_analysis` | Bear/Base/Bull with probability weights | scenarios (name, probability, overrides), output_values, base_case_value |

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

### Bond Analysis

1. `bond_pricer` — price bond with clean/dirty price, accrued interest
   - Day count conventions: Actual/Actual, 30/360, Actual/360, Actual/365
   - Returns clean price, dirty price, accrued interest, settlement details
2. `bond_duration` — compute duration, convexity, DV01, key rate durations
   - Macaulay, modified, effective duration
   - Key rate durations for non-parallel shift analysis
3. `credit_spreads` — decompose credit spread into Z-spread, OAS, I-spread, G-spread
   - Uses benchmark curve for spread computation
   - Returns spread breakdown and implied default probability

### Yield Curve Construction

1. `bootstrap_spot_curve` — bootstrap zero-coupon spot rates from par instruments
   - Iterative bootstrap from shortest to longest maturity
   - Returns spot rates and implied forward rates
2. `nelson_siegel_fit` — fit Nelson-Siegel model to observed yield data
   - Estimates beta_0 (level), beta_1 (slope), beta_2 (curvature), lambda (decay)
   - Extrapolate rates for arbitrary maturities

### Options Analysis

1. `option_pricer` — price options with Black-Scholes or binomial model
   - Returns option premium + full Greeks (delta, gamma, theta, vega, rho)
   - Supports European and American exercise styles
2. `implied_volatility` — back out implied volatility from market price
   - Newton-Raphson solver with convergence diagnostics
3. `option_strategy` — analyze multi-leg option strategies
   - 12 built-in strategies: straddle, strangle, butterfly, condor, spread, collar, etc.
   - Returns payoff diagram, max profit/loss, breakeven points

### Derivatives Portfolio

1. `forward_pricer` — price forwards/futures with cost-of-carry model
   - Supports equity, commodity, currency, and bond underlyings
   - Accounts for dividends, storage costs, convenience yield
2. `forward_position_value` — mark-to-market an existing forward position
   - Returns current MTM value, unrealised P+L, margin requirement
3. `futures_basis_analysis` — analyse futures term structure
   - Contango/backwardation detection, basis convergence, roll yield
4. `interest_rate_swap` — value IRS with fixed/floating leg decomposition
   - Par swap rate calculation, DV01, mark-to-market
5. `currency_swap` — value cross-currency swap
   - Dual-curve discounting, FX exposure, net settlement

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

### Quantitative Risk Management

1. `factor_model` — multi-factor regression analysis
   - CAPM (1-factor), Fama-French 3-factor, Carhart 4-factor, Custom
   - Returns: alpha, factor betas, t-stats, R-squared, Durbin-Watson, information ratio
2. `black_litterman` — portfolio optimisation with investor views
   - Equilibrium returns from market-cap weights, absolute/relative views
   - Returns: posterior returns, optimal weights, Sharpe ratio
3. `risk_parity` — risk-based portfolio construction
   - InverseVolatility, EqualRiskContribution (ERC), MinVariance
   - Returns: weights, risk contributions, diversification ratio
4. `stress_test` — multi-scenario portfolio stress testing
   - 5 built-in historical scenarios (GFC 2008, COVID 2020, Taper Tantrum, Dot-Com, Euro Crisis)
   - Custom hypothetical scenarios with factor shocks
   - Asset class mapping: equity (beta), fixed income (duration), credit, commodity, FX, real estate

### Restructuring & Distressed Debt

1. `recovery_analysis` — Absolute Priority Rule (APR) waterfall
   - Claim classes: DIP, admin, secured (1st/2nd lien), senior, sub, mezzanine, equity
   - Collateral deficiency → unsecured deficiency claim
   - Fulcrum security identification, going-concern vs liquidation analysis
2. `distressed_debt_analysis` — restructuring plan analysis
   - Treatment types: reinstate, amend, exchange, equity conversion, cash paydown
   - Fulcrum identification with mispricing detection
   - IRR at market price, credit bid value, DIP analysis

### Real Assets

1. `property_valuation` — real estate valuation
   - Direct capitalisation (NOI / cap rate), DCF with exit cap rate, GRM from comparables
   - Leveraged returns: mortgage amortisation, DSCR, cash-on-cash, equity multiple, levered IRR
2. `project_finance_model` — infrastructure project finance
   - Construction + operating phases with debt sculpting (level/sculpted/bullet)
   - DSCR, LLCR, PLCR coverage ratios
   - Distribution waterfall: CFADS → senior → sub → DSRA → equity

### FX & Commodities

1. `fx_forward` — FX forward pricing via covered interest parity
   - F = S * ((1+r_d)/(1+r_f))^T, forward points, premium/discount
2. `cross_rate` — cross rate derivation from two currency pairs
   - Finds common currency, chains rates algebraically
3. `commodity_forward` — commodity forward pricing (cost-of-carry)
   - F = S * (1+r+c-y)^T, contango/backwardation, roll yield
4. `commodity_curve` — futures term structure analysis
   - Implied convenience yields, calendar spreads, curve shape classification

### Securitization Analysis

1. `abs_mbs_cashflows` — project pool cash flows with prepayment/default assumptions
   - CPR (constant prepayment rate), PSA (Public Securities Association ramp), SMM (single monthly mortality)
   - CDR (constant default rate), SDA (Standard Default Assumption curve)
2. `cdo_tranching` — model sequential pay waterfall with OC/IC triggers
   - Senior/mezzanine/equity tranche allocation, credit enhancement, WAL
   - Loss allocation bottom-up, excess spread, reserve account mechanics

### Venture Capital Analysis

1. `venture_dilution` — model round-by-round dilution with option pool shuffle
   - Pre-money/post-money, option pool created pre-money (dilutes founders, not investor)
2. `convertible_instrument` — analyse SAFE or convertible note conversion
   - Cap vs discount (take the more favorable), accrued interest, MFN provisions
3. `venture_fund_returns` — analyse fund performance
   - J-curve, TVPI/DPI/RVPI, carry above hurdle, loss ratio, portfolio concentration

### ESG & Climate Analysis

1. `esg_score` — compute ESG score with sector-specific materiality
   - 9 sector-specific weighting schemes, 7-level rating bands (AAA→CCC)
2. `carbon_footprint` — analyse Scope 1/2/3 emissions and intensity
3. `green_bond` — assess green bond framework alignment
4. `sll_covenants` — test sustainability-linked loan KPI performance

### Regulatory Capital Analysis

1. `basel_capital` — compute Basel III capital ratios
   - Standardised approach risk weights, operational risk (BIA/SA), CRM via collateral
2. `lcr_nsfr` — compute liquidity ratios
   - LCR: HQLA with L2 cap (40%), L2B cap (15%), inflow cap (75%)
   - NSFR: ASF/RSF factors by category
3. `alm_analysis` — asset-liability management
   - Repricing/maturity gap analysis, NII sensitivity, EVE duration of equity

### Private Credit Analysis

1. `unitranche_pricing` — price unitranche with FOLO split
   - First-out/last-out economics, blended yield, borrower leverage metrics
2. `direct_lending` — model direct loan with PIK toggle and delayed draw
   - Amortisation schedules, rate floors, YTM via Newton-Raphson IRR, credit analytics
3. `syndication_analysis` — analyse loan syndication
   - Pro-rata scaling, arranger economics, participant allocations

### Insurance & Actuarial Analysis

1. `loss_reserving` — estimate IBNR reserves
   - Chain-ladder: volume-weighted age-to-age factors, cumulative development to ultimate
   - Bornhuetter-Ferguson: blends a priori ELR with development for immature years
2. `premium_pricing` — price insurance premium
   - Frequency × severity, trend projections, expense/profit loadings
3. `combined_ratio` — analyse underwriting profitability
   - Loss ratio, expense ratio, combined ratio, operating ratio (with investment income)
4. `solvency_scr` — compute Solvency II capital requirement
   - Premium/reserve risk, operational risk, diversification benefit, MCR floor

### FP&A Analysis

1. `variance_analysis` — analyse budget vs actual
   - Revenue: price/volume/mix decomposition (always sum to total)
   - Cost: favorable/unfavorable by line item
   - YoY comparison with margin expansion in bps
2. `breakeven_analysis` — compute break-even point
   - Contribution margin, break-even units/revenue, DOL, target volume
   - Scenario analysis with price/cost changes
3. `working_capital` — analyse working capital efficiency
   - DSO, DIO, DPO, cash conversion cycle, NWC as % of revenue
   - Trend analysis, optimisation recommendations, peer benchmarking
4. `rolling_forecast` — build driver-based rolling forecast
   - Revenue compounding, COGS/OpEx ratios, FCF projection, CAGR

### Wealth Management

1. `retirement_planning` — plan retirement with accumulation and decumulation phases
   - 4 withdrawal strategies: Constant Dollar, Constant Percentage, Guardrails, RMD
   - Savings gap analysis, real vs nominal values, legacy projection
2. `tax_loss_harvesting` — simulate TLH opportunities
   - Candidate identification, ST/LT classification, wash-sale 30-day rule
   - Tax savings from offsetting gains, carry-forward of excess losses
3. `estate_planning` — analyse estate tax and planning strategies
   - Annual exclusion gifts, lifetime exemption usage, 7 trust types
   - Federal/state estate tax, GST tax on skip-person gifts, ILIT exclusion

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

cfa bond-price --input bond.json --output table

cfa bond-yield --input bond_yield.json --output json

cfa bootstrap-spot-curve --input par_instruments.json --output table

cfa nelson-siegel --input observed_rates.json --output json

cfa bond-duration --input duration.json --output table

cfa credit-spreads --input spreads.json --output table

cfa option-price --input option.json --output table

cfa implied-vol --input implied_vol.json --output json

cfa forward-price --input forward.json --output json

cfa forward-position --input position.json --output table

cfa futures-basis --input futures.json --output table

cfa irs --input swap.json --output table

cfa currency-swap --input ccy_swap.json --output table

cfa option-strategy --input strategy.json --output table

cfa three-statement --input model.json --output table

cfa monte-carlo --input mc.json --output json

cfa mc-dcf --input mc_dcf.json --output json

cfa factor-model --input factors.json --output table

cfa black-litterman --input bl.json --output table

cfa risk-parity --input rp.json --output table

cfa stress-test --input stress.json --output table

cfa recovery --input recovery.json --output table

cfa distressed-debt --input distressed.json --output table

cfa property-valuation --input property.json --output table

cfa project-finance --input project.json --output table

cfa fx-forward --input fx.json --output json

cfa cross-rate --input cross.json --output json

cfa commodity-forward --input commodity.json --output json

cfa commodity-curve --input curve.json --output table

cfa abs-mbs --input pool.json --output table

cfa cdo-tranching --input cdo.json --output table

cfa venture-dilution --input rounds.json --output table

cfa convertible-instrument --input safe.json --output json

cfa venture-fund-returns --input fund.json --output table

cfa esg-score --input esg.json --output table

cfa carbon-footprint --input carbon.json --output json

cfa green-bond --input green.json --output json

cfa sll-covenants --input sll.json --output table

cfa basel-capital --input capital.json --output table

cfa lcr-nsfr --input liquidity.json --output table

cfa alm --input alm.json --output table

cfa unitranche --input unitranche.json --output table

cfa direct-lending --input loan.json --output table

cfa syndication --input syndication.json --output json

cfa loss-reserving --input triangle.json --output table

cfa premium-pricing --input premium.json --output json

cfa combined-ratio --input ratio.json --output table

cfa solvency-scr --input scr.json --output table

cfa variance --input variance.json --output table

cfa breakeven --input breakeven.json --output json

cfa working-capital --input wc.json --output table

cfa rolling-forecast --input forecast.json --output table

cfa retirement --input retirement.json --output table

cfa tax-loss-harvest --input tlh.json --output json

cfa estate-plan --input estate.json --output table
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
