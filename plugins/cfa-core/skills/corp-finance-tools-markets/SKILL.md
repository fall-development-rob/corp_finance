---
name: "Corp Finance Tools - Markets"
description: "Use the corp-finance-mcp server tools for capital markets calculations. Invoke when performing fixed income analysis (bond pricing, yield analysis, duration/convexity, credit spreads, yield curve bootstrapping, Nelson-Siegel fitting), derivatives pricing (option pricing, implied volatility, forwards/futures, interest rate swaps, currency swaps, option strategies), volatility surface (implied vol surface construction, SABR calibration), interest rate models (Vasicek/CIR/Hull-White, Nelson-Siegel-Svensson term structure), mortgage analytics (prepayment modelling, MBS pass-through analytics), inflation-linked instruments (TIPS pricing, inflation swaps/caps/floors), repo financing (repo rates, implied repo, collateral management), FX (forwards, cross rates), commodities (forwards, term structure), securitization (ABS/MBS, CDO tranching), CLO analytics (waterfall, coverage tests, reinvestment, tranche analytics, scenario analysis), emerging markets (country risk premium, political risk, capital controls, EM bond analysis, EM equity premium). All computation uses 128-bit decimal precision."
---

# Corp Finance MCP Tools - Markets

You have access to 40 capital markets MCP tools for fixed income, derivatives, volatility, rate models, mortgage/MBS, inflation, repo, FX, commodities, securitization, CLO analytics, and emerging markets. All tools return structured JSON with `result`, `methodology`, `assumptions`, `warnings`, and `metadata` fields. All monetary math uses `rust_decimal` (128-bit fixed-point) — never floating-point.

## Tool Reference

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

### Volatility Surface

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `implied_vol_surface` | Build implied volatility surface: interpolation (linear/cubic spline/SVI), Greeks surface, skew/term structure, smile fitting, arbitrage detection, risk reversal, butterfly spreads | spot_price, risk_free_rate, dividend_yield, market_quotes (strike, expiry, implied_vol, option_type), interpolation_method, extrapolation, target_strikes, target_expiries |
| `sabr_calibration` | SABR stochastic volatility model: alpha/beta/rho/nu calibration via Levenberg-Marquardt, Hagan approximation, model vol surface, calibration error, ATM vol, skew, backbone | forward_price, expiry, market_vols (strike, implied_vol), beta (0=normal, 0.5=CIR, 1=lognormal), initial_alpha, initial_rho, initial_nu, target_strikes |

### Interest Rate Models

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `short_rate_model` | Short rate models: Vasicek (mean-reverting Gaussian), CIR (square-root, non-negative), Hull-White (market-calibrated). Bond prices, yields, forwards, Feller condition, theta calibration | model type (Vasicek/Cir/HullWhite), mean_reversion_speed, long_term_rate, volatility, current_rate, time_horizon, time_steps, market_zero_rates (HW) |
| `term_structure_fit` | Yield curve fitting: Nelson-Siegel (4-param), Svensson (6-param), Bootstrap (exact from par/zero/swap). Fitted rates, residuals, RMSE, R-squared, discount factors, forward rates | model type (NelsonSiegel/Svensson/Bootstrap), market_rates or instruments (maturity, rate/coupon/price), initial_params |

### Mortgage Analytics

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `prepayment_analysis` | Mortgage prepayment: PSA ramp, constant CPR, refinancing incentive with burnout. CPR/SMM schedules, projected balances, prepayment amounts, WAL, expected maturity | model type (Psa/Cpr/Refinancing), psa_speed/annual_cpr, loan_age_months, remaining_months, original/current_balance, mortgage_rate, market_rate (refi), burnout_factor |
| `mbs_analytics` | MBS pass-through: cash flow projection with PSA, servicing fees, OAS/Z-spread (bisection), effective duration/convexity, negative convexity detection, WAL, WAC | model type (PassThrough/Oas/Duration), original/current_balance, mortgage_rate, pass_through_rate, servicing_fee, remaining_months, psa_speed, market_price, benchmark_zero_rates |

### Inflation-Linked

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `tips_analytics` | TIPS/inflation-linked bonds: CPI-adjusted pricing (real/nominal), breakeven inflation (Fisher equation, term structure, forward breakeven), real yield curve, deflation floor | model type (Pricing/Breakeven/RealYield), face_value, real_coupon_rate, real_yield, cpi_base/current, cpi_projected_rate, nominal/real_yield_curves, tips_securities |
| `inflation_derivatives` | Inflation derivatives: zero-coupon inflation swap (ZCIS), year-on-year swap (YYIS), inflation cap/floor (Black model). Fair rates, leg PVs, NPV, caplet/floorlet, Greeks | model type (Zcis/Yyis/CapFloor), notional, maturity/num_periods, cpi_base, expected_inflation_curve, real/nominal_discount_curves, strike_rate, inflation_vol |

### Repo Financing

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `repo_analytics` | Repo rate and securities lending: repo rate (haircut, margin, forward price), implied repo (carry, basis), term structure (interpolated curve, specialness premium), sec lending (fee income, reinvestment) | model type (Rate/ImpliedRepo/TermStructure/SecLending), collateral_value, repo_rate, term_days, haircut_pct, initial_margin, spot/forward_clean_price, overnight_rate, term_rates, lending_fee_bps |
| `collateral_analytics` | Collateral management: risk-based haircuts (credit/maturity/volatility/liquidity/FX), margin calls (trigger, LTV, coverage), rehypothecation (funding benefit, velocity, counterparty exposure, regulatory) | model type (Haircut/MarginCall/Rehypothecation), collateral_type, credit_rating, remaining_maturity, price_volatility, initial/current_collateral_value, loan_amount, rehypothecation_limit_pct, num_reuse_chains |

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

### CLO Analytics

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `clo_waterfall` | CLO waterfall engine: payment priority cascades, interest/principal distribution, sequential paydown, equity cash flows | deal_structure, collateral_cashflows, tranches (name, balance, coupon, seniority), fee_schedule, payment_dates, turbo_paydown |
| `clo_coverage_tests` | CLO coverage tests: OC/IC ratios, trigger breach detection, cure mechanics, diversion amounts | tranche_par_values, collateral_par_value, interest_received, interest_due, oc_triggers, ic_triggers, cure_waterfall |
| `clo_reinvestment` | CLO reinvestment period: WARF, WAL, WALS, diversity score, par build test, criteria compliance | portfolio_assets, reinvestment_criteria, warf_limit, wal_limit, diversity_min, par_coverage_target, reinvestment_end_date |
| `clo_tranche_analytics` | CLO tranche analytics: yield-to-worst, WAL, spread duration, breakeven CDR, equity IRR, cash-on-cash | tranche_cashflows, tranche_price, tranche_coupon, discount_curve, prepayment_assumptions, default_scenarios |
| `clo_scenario` | CLO scenario analysis: multi-scenario stress testing, tranche loss allocation, attachment/detachment points | deal_structure, scenarios (default_rate, recovery_rate, prepayment_speed), tranches, attachment_points, detachment_points |

### Emerging Markets

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `country_risk_premium` | Country risk premium: Damodaran sovereign spread, relative volatility, composite risk premium with governance and macro adjustments | country_code, sovereign_spread, equity_volatility, bond_volatility, base_erp, governance_score, macro_indicators |
| `political_risk` | Political risk assessment: WGI composite scoring, MIGA insurance valuation, expropriation/sanctions/conflict risk quantification | country_code, wgi_scores (voice, stability, government, regulatory, rule_of_law, corruption), miga_premium, risk_events |
| `capital_controls` | Capital controls analysis: repatriation delay cost, withholding tax drag, FX conversion cost, effective yield impact, total cost of controls | country_code, gross_yield, repatriation_delay_days, opportunity_cost_rate, withholding_tax_rate, fx_conversion_spread, investment_horizon |
| `em_bond_analysis` | EM bond analysis: local vs hard currency comparison, FX-adjusted yield, carry trade decomposition, hedged/unhedged return scenarios | local_currency_yield, hard_currency_yield, spot_fx_rate, forward_fx_rate, hedge_cost, inflation_differential, duration |
| `em_equity_premium` | EM equity risk premium: sovereign spread method, relative volatility method, composite ERP with valuation and growth adjustments | country_code, sovereign_spread, em_equity_volatility, dm_equity_volatility, base_erp, pe_ratio, gdp_growth, dm_pe_ratio, dm_gdp_growth |

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

### Volatility Surface Analysis

1. `implied_vol_surface` — build complete implied vol surface from market option quotes
   - Interpolation methods: Linear, CubicSpline, SVI (Stochastic Volatility Inspired)
   - Greeks surface: delta, gamma, vega, theta at every strike/expiry point
   - Skew analysis: risk reversal (25-delta call vol - 25-delta put vol), butterfly (wing avg - ATM)
   - Term structure: ATM vol by expiry, forward vol between expiries
   - Arbitrage detection: calendar spread violations (variance must increase with maturity), butterfly violations (convexity in strike)
   - Extrapolation beyond observed data with flat/linear extension
2. `sabr_calibration` — calibrate SABR stochastic volatility model
   - Parameters: alpha (vol level), beta (backbone: 0=normal, 1=lognormal), rho (spot-vol correlation), nu (vol-of-vol)
   - Hagan closed-form approximation for European options
   - Levenberg-Marquardt optimisation minimising squared vol errors
   - Use cases: swaption vol, equity skew, FX smile calibration
3. **Key benchmarks**: skew slope -0.5 to -2.0 per 10 delta points for equity; ATM vol typically 15-25% for major indices; SABR rho typically -0.3 to -0.7 for equity (negative skew)

### Interest Rate Models

1. `short_rate_model` — equilibrium and no-arbitrage rate models
   - Vasicek: dr = a(b-r)dt + sigma*dW (mean-reverting, allows negative rates)
   - CIR: dr = a(b-r)dt + sigma*sqrt(r)*dW (non-negative if 2ab > sigma^2 Feller condition)
   - Hull-White: dr = (theta(t)-a*r)dt + sigma*dW (market-calibrated via theta from market zero curve)
   - Outputs: expected rate path, variance, zero-coupon bond prices P(0,T), yields, forward rates
2. `term_structure_fit` — yield curve fitting models
   - Nelson-Siegel: 4 params (level beta0, slope beta1, curvature beta2, decay lambda)
   - Svensson: 6 params (NS + second hump beta3, lambda2) for complex curve shapes
   - Bootstrap: exact fit from market instruments (zero-coupon, par bond, swap rates)
   - Outputs: fitted rates, discount factors, forward rates, residuals, RMSE, R-squared
3. **Key benchmarks**: NS R-squared > 0.99; Feller condition 2ab > sigma^2 for CIR; HW calibration RMSE < 5bps; Svensson preferred when curve has two humps

### Mortgage Analytics

1. `prepayment_analysis` — prepayment speed modelling
   - PSA: ramp from 0.2% CPR/month to plateau at 6% CPR at month 30, scaled by PSA speed (100% PSA = standard)
   - Constant CPR: flat annual prepayment rate, converted to monthly SMM = 1 - (1-CPR)^(1/12)
   - Refinancing incentive: base CPR + incentive_multiplier * max(0, mortgage_rate - market_rate), with burnout decay for seasoned loans
   - Outputs: monthly CPR/SMM schedule, projected balances, prepayment and principal amounts, WAL
2. `mbs_analytics` — MBS pass-through analysis
   - Pass-through cash flows: scheduled principal + interest + prepayment - servicing fee
   - OAS (option-adjusted spread): spread over benchmark that equates PV of cash flows to market price (bisection solver)
   - Duration/convexity: effective (parallel shift +-shock), Macaulay, modified, DV01
   - Negative convexity: prepayment acceleration at lower rates caps price upside
   - WAC (weighted average coupon), WAL (weighted average life)
3. **Key benchmarks**: 100% PSA = standard; 150-200% PSA for rate rallies; OAS 30-80bps for agency MBS; negative convexity typical for premium MBS; WAL 3-7 years at 150% PSA

### Inflation-Linked Instruments

1. `tips_analytics` — TIPS and inflation-linked bond analysis
   - CPI-adjusted pricing: real clean/dirty price, nominal clean/dirty price with index ratio = CPI_current/CPI_base
   - Breakeven inflation: nominal yield - real yield (Fisher equation), term structure of breakevens, forward breakeven rates
   - Real yield curve: fit from TIPS securities at multiple maturities
   - Deflation floor: TIPS principal repaid at max(par, CPI-adjusted par) — option value in deflation
   - Projected cash flows: inflation-adjusted coupons and principal
2. `inflation_derivatives` — inflation derivative pricing
   - ZCIS (zero-coupon inflation swap): fixed leg pays (1+strike)^T, inflation leg pays CPI_T/CPI_0
   - YYIS (year-on-year inflation swap): periodic payments based on annual CPI change
   - Inflation cap/floor: Black model for caplet/floorlet pricing with inflation volatility
   - Greeks: delta (sensitivity to inflation expectations), vega (sensitivity to inflation vol)
3. **Key benchmarks**: 10Y breakeven 2.0-2.5% = well-anchored inflation expectations; TIPS real yield negative = strong inflation hedging demand; ZCIS rate vs breakeven divergence = liquidity premium

### Repo Financing

1. `repo_analytics` — repo and securities lending analysis
   - Repo rate: repurchase price = collateral_value * (1-haircut) * (1 + rate * term/basis)
   - Implied repo: back out financing cost from spot/forward price differential and coupon income
   - Term structure: interpolated repo curve, forward repo rates, specialness premium (GC vs special)
   - Securities lending: fee income, cash collateral reinvestment spread, intrinsic value (lending fee - rebate)
2. `collateral_analytics` — collateral management
   - Risk-based haircuts: credit (AAA=1%, B=15%), maturity (scaling), volatility (3x daily vol), liquidity (add-on for illiquid), FX (5% for cross-currency)
   - Margin calls: trigger detection (current LTV vs maintenance), call amount, coverage ratio
   - Rehypothecation: funding benefit from reuse, collateral velocity (chains of reuse), counterparty exposure amplification, regulatory limits (e.g., 140% under SEC Rule 15c3-3)
3. **Key benchmarks**: Treasury haircut 1-2%; corporate bond haircut 5-15%; GC repo rate near Fed Funds; special repo rate < GC = collateral scarcity; rehypothecation velocity 2-3x typical

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

### CLO Analytics Workflow

1. `clo_waterfall` — model full CLO payment cascade
   - Interest waterfall: senior fees -> AAA interest -> AA -> A -> BBB -> BB -> equity residual
   - Principal waterfall: AAA principal -> sequential paydown through capital structure
   - Turbo: divert excess interest to principal paydown when OC/IC triggers breached
2. `clo_coverage_tests` — monitor compliance triggers
   - OC (overcollateralisation): par value / tranche par > trigger level
   - IC (interest coverage): interest received / interest due > trigger level
   - Cure mechanics: redirect equity cash flows to cure breached tests
3. `clo_reinvestment` — manage reinvestment period constraints
   - WARF (weighted average rating factor): portfolio credit quality measure
   - WAL (weighted average life): average maturity of collateral pool
   - Diversity score: effective number of uncorrelated issuers
   - Par build test: reinvestment must maintain/increase par coverage
4. `clo_tranche_analytics` — analyse individual tranche metrics
   - Yield-to-worst, spread duration, breakeven CDR
   - Equity IRR and cash-on-cash return analysis
5. `clo_scenario` — stress test across multiple scenarios
   - Default rate stress, recovery stress, prepayment stress
   - Tranche loss allocation at attachment/detachment points
Key benchmarks: CLO AAA OC trigger ~120%; BB CDR breakeven 3-5%; equity IRR target 12-18%; reinvestment period typically 4-5 years; diversity score > 50 for well-diversified pool

### Emerging Markets Workflow

1. `country_risk_premium` — estimate CRP for WACC adjustments
   - Damodaran sovereign spread: CRP = default spread * (equity_vol / bond_vol)
   - Relative volatility method: CRP = base_ERP * (EM_vol / DM_vol)
   - Composite with governance and macro adjustments
2. `political_risk` — quantify political/regulatory risks
   - World Governance Indicators (WGI) composite across 6 dimensions
   - MIGA insurance: cost of political risk insurance
   - Expropriation, sanctions, and conflict risk scoring
3. `capital_controls` — cost of investing with capital restrictions
   - Repatriation delay opportunity cost, WHT drag, FX conversion friction
   - Net effective yield after controls vs gross yield
4. `em_bond_analysis` — local vs hard currency EM fixed income
   - Carry trade decomposition: interest differential, FX appreciation, rolldown
   - Hedged vs unhedged return scenarios
5. `em_equity_premium` — estimate EM equity risk premium
   - Sovereign spread method, relative volatility, composite ERP
Key benchmarks: EM CRP range 100-800bps; political risk insurance 0.5-3% annually; capital control cost 50-300bps effective drag; EM local-hard currency spread 200-600bps

---

## CLI Equivalent

The same calculations are available via the `cfa` binary:

```bash
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

cfa implied-vol-surface --input vol_surface.json --output table

cfa sabr-calibration --input sabr.json --output json

cfa short-rate --input short_rate.json --output table

cfa term-structure-fit --input term_structure.json --output json

cfa prepayment --input prepayment.json --output table

cfa mbs-analytics --input mbs.json --output json

cfa tips-analytics --input tips.json --output table

cfa inflation-derivative --input inflation_deriv.json --output json

cfa repo-analytics --input repo.json --output table

cfa collateral-analytics --input collateral.json --output json

cfa fx-forward --input fx.json --output json

cfa cross-rate --input cross.json --output json

cfa commodity-forward --input commodity.json --output json

cfa commodity-curve --input curve.json --output table

cfa abs-mbs --input pool.json --output table

cfa cdo-tranching --input cdo.json --output table

cfa clo-waterfall --input clo.json --output table

cfa clo-coverage --input clo_tests.json --output table

cfa clo-reinvestment --input clo_reinvest.json --output json

cfa clo-tranche --input tranche.json --output table

cfa clo-scenario --input clo_stress.json --output json

cfa country-risk-premium --input crp.json --output table

cfa political-risk --input pol_risk.json --output json

cfa capital-controls --input controls.json --output table

cfa em-bond-analysis --input em_bond.json --output table

cfa em-equity-premium --input em_erp.json --output json
```

Output formats: `--output json` (default), `--output table`, `--output csv`, `--output minimal`.

Pipe support: `cat data.json | cfa bond-price --output table`

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
