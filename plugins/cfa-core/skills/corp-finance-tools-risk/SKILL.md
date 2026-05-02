---
name: "Corp Finance Tools - Risk & Quant"
description: "Use the corp-finance-mcp server tools for quantitative risk and analytics calculations. Invoke when performing quantitative risk analysis (factor models, Black-Litterman, risk parity, stress testing), portfolio optimization (Markowitz mean-variance, Black-Litterman portfolio), risk budgeting (factor-based risk decomposition, tail risk VaR/CVaR), market microstructure (bid-ask spread analysis, optimal execution), quantitative strategies (pairs trading, momentum), behavioral finance (prospect theory, market sentiment), performance attribution (Brinson-Fachler, factor-based), credit portfolio analytics (credit VaR, rating migration), macro economics (Taylor rule, Phillips curve, Okun's law, recession risk, PPP, interest rate parity, balance of payments), credit scoring (logistic regression scorecard, Merton structural model, intensity model, PD calibration, scoring validation), capital allocation (economic capital, RAROC, Euler allocation, Shapley allocation, limit management), index construction (weighting, rebalancing, tracking error, smart beta, reconstitution). All computation uses 128-bit decimal precision."
---

# Corp Finance Tools - Risk & Quant

You have access to 35 quantitative risk and analytics MCP tools for factor analysis, portfolio optimization, risk budgeting, market microstructure, quantitative strategies, behavioral finance, performance attribution, credit portfolio analytics, macro economics, credit scoring, capital allocation, and index construction. All tools return structured JSON with `result`, `methodology`, `assumptions`, `warnings`, and `metadata` fields. All monetary math uses `rust_decimal` (128-bit fixed-point) — never floating-point.

## Tool Reference

### Quantitative Risk

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `factor_model` | Multi-factor regression (CAPM, FF3, Carhart4, Custom) | asset_returns, factor_returns (MKT, SMB, HML, MOM), model_type |
| `black_litterman` | Black-Litterman portfolio optimisation with views | market_cap_weights, covariance_matrix, views (absolute/relative), risk_aversion, tau |
| `risk_parity` | Risk parity portfolio construction | assets, covariance_matrix, method (InverseVol/ERC/MinVariance), target_volatility |
| `stress_test` | Multi-scenario stress testing with 5 built-in historical | portfolio positions, scenarios (or use built-in GFC/COVID/etc.), correlation_adjustments |

### Portfolio Optimization

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `mean_variance_optimization` | Markowitz mean-variance: efficient frontier, tangency (max Sharpe) portfolio, global minimum variance, constrained optimal weights, diversification ratio, HHI | asset_names, expected_returns, covariance_matrix, risk_free_rate, constraints (long_only, min/max_weights, sector_constraints), frontier_points, target_return, target_risk |
| `black_litterman_portfolio` | Black-Litterman portfolio: implied equilibrium returns, posterior estimation with views (absolute/relative), optimal tilted weights, view contribution, tracking error, information ratio | asset_names, market_cap_weights, covariance_matrix, risk_free_rate, risk_aversion, tau, views (Absolute/Relative), view_confidences |

### Risk Budgeting

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `factor_risk_budget` | Factor-based risk budgeting: per-factor risk contribution, factor exposure, marginal risk, systematic vs idiosyncratic breakdown, concentration analysis, rebalancing to target budgets | asset_names, weights, factor_names, factor_loadings (N x K), factor_covariance (K x K), specific_variances, risk_budgets, rebalance |
| `tail_risk_analysis` | Tail risk: parametric/Cornish-Fisher/historical VaR and CVaR, marginal and component risk, stress testing, maximum drawdown, tail dependence | asset_names, weights, expected_returns, covariance_matrix, confidence_level, time_horizon, distribution (Normal/CornishFisher/Historical), historical_returns, portfolio_value, stress_scenarios |

### Market Microstructure

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `spread_analysis` | Bid-ask spread decomposition: quoted/effective/realized spreads, adverse selection (Kyle lambda), inventory risk, information share, market quality score, price impact | security_name, trade_data (timestamp, price, volume, side), quote_data (timestamp, bid/ask price/size), analysis_method (Quoted/Effective/Realized/RollModel/KyleModel), daily_volume, market_cap |
| `optimal_execution` | Optimal execution: Almgren-Chriss framework, TWAP/VWAP/IS/POV strategies, market impact/timing risk/opportunity cost estimation, optimal trajectory, adaptive scheduling | security_name, order_size, side, execution_strategy, market_params (price, volume, volatility, spread, impact coefficients), time_horizon, num_slices, urgency, constraints |

### Quantitative Strategies

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `pairs_trading` | Statistical pairs trading: cointegration, z-scores, half-life, backtested trades, Sharpe ratio | asset_a_name, asset_b_name, asset_a_prices, asset_b_prices, lookback_period, entry_z_score, exit_z_score, stop_loss_z_score, capital |
| `momentum_analysis` | Momentum factor scoring: risk-adjusted rankings, portfolio construction, backtest, crash risk | assets (name, monthly_returns), lookback_months, skip_months, rebalance_frequency, top_n, risk_free_rate |

### Behavioral Finance

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `prospect_theory` | Prospect theory: loss aversion, probability weighting, certainty equivalent, disposition effect, framing | outcomes (value, probability), reference_point, loss_aversion_lambda, alpha, beta_param, gamma, delta_param |
| `market_sentiment` | Market sentiment: fear/greed index, put-call ratio, VIX analysis, fund flows, contrarian signals | market_name, vix_current, vix_sma_50, put_call_ratio, advance_decline_ratio, margin_debt_change_pct, fund_flows, short_interest_ratio |

### Performance Attribution

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `brinson_attribution` | Brinson-Fachler performance attribution (allocation, selection, interaction) | portfolio_sectors (name, portfolio_weight, benchmark_weight, portfolio_return, benchmark_return), total_portfolio_return, total_benchmark_return, periods, linking_method |
| `factor_attribution` | Factor-based return attribution (active exposure decomposition) | portfolio_returns, factor_returns (name, returns), risk_free_rate, benchmark_returns |

### Credit Portfolio

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `portfolio_credit_risk` | Portfolio credit risk (Gaussian copula VaR, HHI, granularity adjustment) | exposures (name, exposure, pd, lgd, rating), correlation, confidence_level, num_simulations |
| `credit_migration` | Rating migration analysis (transition matrix, multi-year default) | transition_matrix, initial_ratings, horizon_years, spreads_by_rating, exposures |

### Macro Economics

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `monetary_policy` | Monetary policy analysis (Taylor Rule, Phillips Curve, Okun's Law) | current_fed_funds_rate, inflation_rate, target_inflation, gdp_growth, potential_gdp_growth, unemployment_rate, natural_unemployment_rate, output_gap |
| `international_economics` | International economics (PPP, interest rate parity, BoP, REER) | domestic_country, foreign_country, spot_fx_rate, domestic_inflation, foreign_inflation, domestic_interest_rate, foreign_interest_rate, forward_fx_rate, trade_balance, capital_flows |

### Credit Scoring

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `credit_scorecard` | Logistic regression scorecard: WoE binning, IV calculation, scorecard points, Gini coefficient, KS statistic | applicant_data, target_variable, features, base_score, pdo (points to double odds), binning_method |
| `merton_pd` | Merton structural model: asset value/volatility estimation, distance to default, PD, KMV EDF | equity_value, equity_volatility, debt_face_value, risk_free_rate, time_horizon, dividend_yield |
| `intensity_model` | Reduced-form intensity model: hazard rates from CDS spreads, survival probability, term structure | cds_spreads, tenors, recovery_rate, discount_curve, interpolation_method |
| `pd_calibration` | PIT/TTC PD calibration: Vasicek single-factor model, Basel IRB correlation, central tendency | raw_pds, rating_grades, macro_factor, calibration_method (PIT/TTC), central_tendency, asset_correlation |
| `scoring_validation` | Credit model validation: AUC-ROC, accuracy ratio, Gini, Brier score, Hosmer-Lemeshow test | predicted_probabilities, actual_outcomes, score_bins, validation_method |

### Capital Allocation

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `economic_capital` | Economic capital: VaR/ES-based capital, IRB capital requirement (Basel), stress capital buffer, adequacy ratio | exposures, pds, lgds, correlations, confidence_level, capital_method (VaR/ES/IRB), available_capital |
| `raroc_calculation` | RAROC: risk-adjusted return on capital, RORAC, EVA, SVA, spread to hurdle, risk-adjusted pricing | revenue, costs, expected_loss, economic_capital, cost_of_capital, hurdle_rate, tax_rate |
| `euler_allocation` | Euler risk contribution: marginal capital allocation, diversification benefit, HHI concentration | business_units, standalone_risks, portfolio_risk, correlation_matrix, risk_measure (VaR/ES) |
| `shapley_allocation` | Shapley value capital allocation: game-theoretic fair allocation (exact N<=8, sampled N>8) | business_units, risk_function, num_samples (for N>8), confidence_interval |
| `limit_management` | Risk limit management: notional/VaR/concentration limits, utilization tracking, breach detection | business_units, limits (notional/VaR/concentration), current_exposures, escalation_thresholds |

### Index Construction

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `index_weighting` | Index weighting: market-cap, equal, fundamental, free-float with cap constraints, HHI concentration, sector breakdown | constituents, weighting_method (MarketCap/Equal/Fundamental/FreeFloat), cap_constraint, sector_caps |
| `index_rebalancing` | Index rebalancing: drift analysis, optimal trade list, transaction cost estimation, turnover metrics, liquidity-adjusted scheduling | current_weights, target_weights, market_data, cost_model, rebalance_frequency, liquidity_data |
| `tracking_error` | Tracking error analysis: ex-post TE from returns, ex-ante TE from weights/covariance, active share, information ratio decomposition | portfolio_returns, benchmark_returns, portfolio_weights, benchmark_weights, covariance_matrix |
| `smart_beta` | Smart beta construction: multi-factor tilted weights (value, momentum, quality, low-vol, dividend) with factor exposure and risk analysis | constituents, factor_data, factor_weights, tilt_strength, constraints, risk_model |
| `index_reconstitution` | Index reconstitution: member eligibility screening, additions/deletions, buffer zone management, turnover estimation, impact analysis | universe, eligibility_criteria, current_members, buffer_pct, effective_date, market_data |

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

### Portfolio Optimization Workflow

1. `mean_variance_optimization` — Markowitz efficient frontier construction
   - Efficient frontier: set of portfolios with maximum return for each risk level
   - Tangency portfolio: maximum Sharpe ratio portfolio (optimal risk-return trade-off)
   - Global minimum variance: lowest volatility portfolio on the frontier
   - Constraints: long-only, min/max per-asset weights, sector limits, max total short
   - Metrics: diversification ratio (weighted avg vol / portfolio vol), HHI concentration
2. `black_litterman_portfolio` — portfolio optimization with investor views
   - Implied equilibrium returns: reverse-optimise from market-cap weights (Pi = delta * Sigma * w_mkt)
   - Views: absolute ("Asset A returns 8%") or relative ("A outperforms B by 2%")
   - Posterior returns: blend equilibrium with views weighted by confidence
   - View contribution analysis: how each view tilts the portfolio vs market weights
   - Tracking error vs market portfolio, information ratio
3. **Key benchmarks**: diversification ratio > 1.3 = well-diversified; HHI < 0.10 = unconcentrated; tau 0.02-0.10 (lower = less weight on views)

### Risk Budgeting Workflow

1. `factor_risk_budget` — factor-based risk decomposition and budgeting
   - Per-factor risk contribution: how much of total portfolio risk comes from each systematic factor
   - Marginal risk: incremental risk from small increase in factor exposure
   - Systematic vs idiosyncratic: breakdown of total risk into factor-driven and residual
   - Target budgets: solve for weights that achieve desired risk allocation across factors
   - Concentration analysis: identify dominant risk sources
2. `tail_risk_analysis` — tail risk measurement and stress testing
   - VaR methods: parametric (Normal), Cornish-Fisher (skewness/kurtosis adjusted), historical
   - CVaR (Expected Shortfall): average loss beyond VaR threshold
   - Component risk: per-asset contribution to portfolio VaR
   - Marginal risk: sensitivity of VaR to small weight changes
   - Stress scenarios: portfolio P&L under user-defined shock vectors
   - Maximum drawdown estimation from return distribution
3. **Key benchmarks**: CVaR/VaR ratio > 1.3 for fat-tailed distributions; Cornish-Fisher VaR > Normal VaR indicates negative skew/excess kurtosis; factor risk > 60% systematic = factor-driven portfolio

### Market Microstructure Workflow

1. `spread_analysis` — bid-ask spread decomposition and market quality
   - Quoted spread: ask - bid (raw spread from order book)
   - Effective spread: 2 * |trade_price - midpoint| (actual transaction cost)
   - Realized spread: effective spread minus price impact (market maker revenue)
   - Roll model: implied spread from serial covariance of returns
   - Kyle lambda: adverse selection component (price impact per unit volume)
   - Market quality score: composite of spread, depth, resilience
2. `optimal_execution` — trade execution optimization
   - Almgren-Chriss framework: minimize expected cost + risk_aversion * variance of cost
   - Strategies: TWAP (uniform), VWAP (volume-weighted), IS (implementation shortfall), POV (participation)
   - Cost decomposition: market impact (temporary + permanent), timing risk, opportunity cost
   - Optimal trajectory: shares per time slice minimising total cost
   - Constraints: max participation rate, no-trade periods, min/max slice size
3. **Key benchmarks**: effective spread < 5bps for large-cap liquid; Kyle lambda < 0.01 = low adverse selection; IS cost < 25bps = good execution

### Quantitative Strategies Analysis

1. `pairs_trading` — statistical arbitrage pairs analysis
   - Pearson correlation and OLS hedge ratio between two assets
   - Spread z-score: (current_spread - mean) / std for entry/exit signals
   - ADF cointegration test: Dickey-Fuller statistic for mean-reversion
   - Half-life: time for spread to revert halfway to mean (via AR(1) regression)
   - Backtested trade history: entry/exit points, P&L per trade, win rate
   - Sharpe ratio, max drawdown, total return of backtested strategy
2. `momentum_analysis` — cross-sectional momentum factor
   - Cumulative return scoring with skip period (avoid short-term reversal)
   - Risk-adjusted momentum: Sharpe-weighted ranking across assets
   - Inverse-volatility portfolio weighting for risk control
   - Rolling backtest with configurable rebalance frequency
   - HHI concentration index: portfolio diversification measure
   - Crash risk score: tail risk from momentum crowding
3. **Key benchmarks**:
   - Pairs: correlation > 0.8, ADF p-value < 0.05 for tradeable pair
   - Half-life 5-30 days optimal for mean-reversion strategies
   - Momentum: 12-1 lookback (12 months, skip 1) is standard academic factor
   - Momentum Sharpe > 0.5 annualized is attractive after costs

### Behavioral Finance Analysis

1. `prospect_theory` — Kahneman-Tversky prospect theory analysis
   - Value function: concave for gains (risk-averse), convex for losses (risk-seeking)
   - Probability weighting (Prelec): overweight small probabilities, underweight large
   - Certainty equivalent via bisection: the guaranteed amount equivalent to a gamble
   - Disposition effect: tendency to sell winners too early, hold losers too long
   - Framing bias: gain frame vs loss frame comparison
   - Mental accounting: segregation of outcomes into psychological zones
2. `market_sentiment` — Fear & Greed composite index
   - 9-indicator composite score (0-100): VIX, put-call ratio, breadth, new highs/lows, margin debt, fund flows, short interest, insider activity, consumer confidence
   - Each indicator normalised to 0-100 scale with bullish/bearish thresholds
   - Volatility regime classification: Low, Normal, Elevated, High, Extreme
   - Contrarian signals: extreme fear = potential buy, extreme greed = potential sell
   - Smart money indicator: insider buy/sell ratio as informed-participant signal
   - Flow momentum: trend in fund flows (acceleration/deceleration)
3. **Key benchmarks**:
   - Fear & Greed < 20: extreme fear (contrarian buy signal)
   - Fear & Greed > 80: extreme greed (contrarian sell signal)
   - VIX > 30: elevated fear; VIX < 15: complacency
   - Put/call > 1.2: bearish sentiment; < 0.7: bullish sentiment

### Performance Attribution Analysis

1. `brinson_attribution` — Brinson-Fachler sector-level attribution
   - Allocation effect: over/underweight in outperforming sectors
   - Selection effect: stock picking within each sector
   - Interaction effect: combined allocation + selection cross-term
   - Multi-period linking via Carino method for compounding consistency
   - Total active return = sum of allocation + selection + interaction across all sectors
2. `factor_attribution` — factor-based return decomposition
   - Active exposure: portfolio factor loadings minus benchmark loadings
   - Factor contribution: active exposure x factor return for each factor
   - R-squared: proportion of active return explained by systematic factors
   - Tracking error breakdown: systematic vs idiosyncratic components
   - Information ratio: active return / tracking error
3. **Key benchmarks**:
   - Allocation effect dominance: top-down managers
   - Selection effect dominance: bottom-up stock pickers
   - R-squared > 0.90: returns largely factor-driven (closet indexer risk)
   - Tracking error < 2%: enhanced index; 2-6%: active; > 6%: concentrated

### Credit Portfolio Analysis

1. `portfolio_credit_risk` — portfolio-level credit risk analytics
   - Gaussian copula: correlated default simulation for portfolio VaR/CVaR
   - HHI concentration index: name and sector concentration measurement
   - Granularity adjustment: correction for finite number of obligors
   - Marginal risk contribution: incremental VaR from each exposure
   - Expected loss: sum of EAD x PD x LGD across all exposures
2. `credit_migration` — rating transition analysis
   - Transition matrix exponentiation: multi-year migration probabilities
   - Cumulative default probability: probability of reaching default state by horizon
   - Mark-to-market migration VaR: portfolio value change from rating movements
   - Spread changes by rating: P&L impact of upgrades/downgrades
   - Fallen angel probability: investment-grade to high-yield transition risk
3. **Key benchmarks**:
   - HHI < 0.10: well-diversified portfolio
   - Single-name limit: typically 2-5% of portfolio
   - IG cumulative 5Y default: ~1-2%; HY cumulative 5Y default: ~15-25%
   - Migration VaR typically 2-5x expected loss for IG portfolios

### Macro Economics Analysis

1. `monetary_policy` — monetary policy framework analysis
   - Taylor Rule: prescribed rate = neutral + 1.5*(inflation - target) + 0.5*(output gap)
   - Phillips Curve: inflation-unemployment trade-off dynamics
   - Okun's Law: GDP gap = -2 * (unemployment - NAIRU)
   - Recession risk scoring: composite of leading indicators
   - Policy stance: hawkish/neutral/dovish relative to Taylor Rule prescription
2. `international_economics` — open economy analytics
   - Purchasing Power Parity: relative/absolute PPP, implied equilibrium FX rate
   - Covered Interest Rate Parity: forward = spot * (1+r_d)/(1+r_f), arbitrage check
   - Uncovered Interest Rate Parity: expected spot = forward (no risk premium)
   - Balance of payments: current account + capital account decomposition
   - Real Effective Exchange Rate (REER): trade-weighted, inflation-adjusted FX index
3. **Key benchmarks**:
   - Taylor Rule gap > 100bp: policy significantly loose/tight
   - PPP deviation > 20%: currency potentially over/undervalued
   - Current account deficit > 4% GDP: external vulnerability
   - REER deviation > 15% from 10Y average: mean-reversion opportunity

### Credit Scoring Workflow

1. `credit_scorecard` — build logistic regression scorecard
   - Weight of Evidence (WoE) binning for each variable
   - Information Value (IV) for variable selection (IV > 0.3 = strong predictor)
   - Scorecard points: scale to industry standard (e.g., 600 base, 20 points per doubling of odds)
   - Gini coefficient and KS statistic for discrimination power
2. `merton_pd` — structural model probability of default
   - Estimate asset value and asset volatility from equity price and volatility
   - Distance to default: (ln(V/D) + (mu - 0.5*sigma^2)*T) / (sigma*sqrt(T))
   - PD from distance to default via normal CDF
   - KMV EDF (Expected Default Frequency) mapping
3. `intensity_model` — reduced-form hazard rate model
   - Extract hazard rates from CDS spreads: lambda = spread / (1 - recovery)
   - Survival probability term structure
   - Forward hazard rates for conditional default probability
4. `pd_calibration` — calibrate PIT/TTC probabilities
   - Vasicek single-factor model: conditional PD at given systematic factor
   - Basel IRB correlation function: rho = 0.12 * (1 - e^(-50*PD)) / (1 - e^(-50)) + 0.24 * (1 - ...)
   - Central tendency adjustment for through-the-cycle PD
5. `scoring_validation` — validate credit model performance
   - AUC-ROC (area under receiver operating characteristic curve)
   - Accuracy ratio = 2*AUC - 1 (Gini coefficient)
   - Brier score: mean squared probability error
   - Hosmer-Lemeshow goodness-of-fit test
Key benchmarks: Gini > 0.60 = good scorecard; AUC > 0.80 = strong discriminator; KS > 40% = good separation; Basel IRB minimum requirements: AR > 0.40

### Capital Allocation Workflow

1. `economic_capital` — compute risk-based capital requirements
   - VaR/ES-based economic capital at 99.9% confidence
   - IRB capital requirement: K = LGD * (Phi^-1(PD) + sqrt(rho)*Phi^-1(0.999)) / sqrt(1-rho) - PD*LGD
   - Stress capital buffer: incremental capital for stress scenarios
   - Adequacy ratio: available capital / required capital
2. `raroc_calculation` — risk-adjusted pricing and performance
   - RAROC = (revenue - costs - expected_loss) / economic_capital
   - RORAC = net income / economic_capital
   - EVA = net income - (cost_of_capital * economic_capital)
   - Spread to hurdle: minimum spread above funding cost to earn hurdle rate
3. `euler_allocation` — proportional risk contribution
   - Marginal contribution: partial derivative of portfolio risk w.r.t. position size
   - Euler property: marginal contributions sum to total portfolio risk
   - Diversification benefit: total of standalone risks minus portfolio risk
4. `shapley_allocation` — game-theoretic fair allocation
   - Exact computation for N<=8 (all permutations)
   - Sampled approximation for N>8 (Monte Carlo permutation sampling)
   - Each business unit assigned capital reflecting marginal contribution across all possible coalitions
5. `limit_management` — risk limit monitoring
   - Notional, VaR, and concentration limits per business unit
   - Utilization tracking: current exposure / limit
   - Breach detection and escalation triggers
Key benchmarks: RAROC hurdle rate typically 12-15%; Euler allocation ensures sub-additivity; Shapley is the unique allocation satisfying efficiency, symmetry, dummy, and additivity axioms

### Index Construction Workflow

1. `index_weighting` — choose and apply weighting scheme
   - Market-cap: standard float-adjusted capitalization
   - Equal-weight: 1/N allocation, rebalanced periodically
   - Fundamental: weighted by revenue, earnings, book value, or dividends
   - Free-float: exclude locked-up/strategic/government holdings
   - Cap constraints: max weight per constituent, sector caps
   - HHI concentration: measure index concentration
2. `index_rebalancing` — manage periodic rebalancing
   - Drift analysis: current weights vs target weights
   - Optimal trade list: buy/sell quantities to restore target
   - Transaction cost estimation: spread + impact + opportunity cost
   - Turnover: sum of absolute weight changes / 2
   - Liquidity-adjusted scheduling: phase trades for illiquid names
3. `tracking_error` — measure index replication quality
   - Ex-post TE: standard deviation of return differences (portfolio vs benchmark)
   - Ex-ante TE: from weights and covariance matrix
   - Active share: sum of |w_portfolio - w_benchmark| / 2
   - Information ratio = active return / tracking error
4. `smart_beta` — construct factor-tilted indices
   - Multi-factor: value, momentum, quality, low-volatility, dividend yield
   - Factor exposure analysis: loading on each factor
   - Risk decomposition: factor risk vs specific risk
5. `index_reconstitution` — manage index membership changes
   - Eligibility screening: market cap, liquidity, domicile, free float
   - Buffer zone: prevents excessive turnover from borderline constituents
   - Additions/deletions list with effective date
   - Turnover estimation and market impact analysis
Key benchmarks: TE < 50bps = close replication; active share > 60% = truly active; turnover < 20% annual for low-cost index; HHI < 0.10 = well-diversified

---

## CLI Equivalent

The same calculations are available via the `cfa` binary:

```bash
cfa factor-model --input factors.json --output table

cfa black-litterman --input bl.json --output table

cfa risk-parity --input rp.json --output table

cfa stress-test --input stress.json --output table

cfa mean-variance-opt --input mv.json --output table

cfa black-litterman-portfolio --input bl_portfolio.json --output table

cfa factor-risk-budget --input risk_budget.json --output table

cfa tail-risk --input tail_risk.json --output json

cfa spread-analysis --input spread.json --output table

cfa optimal-execution --input execution.json --output json

cfa pairs-trading --input pairs.json --output table

cfa momentum --input momentum.json --output json

cfa prospect-theory --input prospect.json --output table

cfa market-sentiment --input sentiment.json --output json

cfa brinson-attribution --input attribution.json --output table

cfa factor-attribution --input factor_attr.json --output json

cfa portfolio-credit-risk --input credit_portfolio.json --output table

cfa credit-migration --input migration.json --output json

cfa monetary-policy --input macro.json --output table

cfa international-economics --input intl_econ.json --output json

cfa credit-scorecard --input scorecard.json --output table

cfa merton-pd --input merton.json --output json

cfa intensity-model --input intensity.json --output table

cfa pd-calibration --input calibration.json --output json

cfa scoring-validation --input validation.json --output table

cfa economic-capital --input ecap.json --output table

cfa raroc --input raroc.json --output json

cfa euler-allocation --input euler.json --output table

cfa shapley-allocation --input shapley.json --output json

cfa limit-management --input limits.json --output table

cfa index-weighting --input weights.json --output table

cfa index-rebalancing --input rebalance.json --output json

cfa tracking-error --input te.json --output table

cfa smart-beta --input smartbeta.json --output json

cfa index-reconstitution --input reconst.json --output table
```

Output formats: `--output json` (default), `--output table`, `--output csv`, `--output minimal`.

Pipe support: `cat data.json | cfa factor-model --output table`

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
