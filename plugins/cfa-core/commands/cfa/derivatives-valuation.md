---
requires_tools:
  - analyze_strategy
  - build_implied_vol_surface
  - build_sensitivity_grid
  - calibrate_sabr
  - implied_volatility
  - price_exotic
  - price_forward
  - price_option
  - price_structured_note
  - run_monte_carlo
  - value_interest_rate_swap
requires_external_tools:
  - yf_options_chain
  - yf_quote
---
# Derivatives Valuation

Perform derivatives pricing and analysis using the `corp-finance-tools-markets` skill. Covers options valuation, Greeks, volatility surfaces, and structured product decomposition.

## What It Does
Executes a multi-step derivatives assessment: (1) Options pricing via Black-Scholes (European) and CRR binomial (American), (2) Full Greeks calculation (delta, gamma, vega, theta, rho), (3) Implied volatility extraction and volatility surface construction (SABR calibration), (4) Strategy payoff analysis for multi-leg positions, (5) Structured product pricing and risk decomposition.

## Agent
Routes to `cfa-derivatives-analyst` with `corp-finance-tools-markets` skill.

## Key Tools
`price_option`, `implied_volatility`, `build_implied_vol_surface`, `calibrate_sabr`, `price_forward`, `value_interest_rate_swap`, `analyze_strategy`, `price_structured_note`, `price_exotic`, `build_sensitivity_grid`, `run_monte_carlo`, `yf_options_chain`, `yf_quote`

## Quality Standards
- Cite every price, Greek, and implied vol to the specific tool call that produced it
- State all assumptions (risk-free rate, dividend yield, exercise style, model choice, time steps for binomial)
- Cross-validate Black-Scholes vs binomial for American options and explain any divergence
- Show Greeks in both per-unit and portfolio-level (position-weighted) terms
- Present P&L profiles under at least three spot scenarios and two vol scenarios
- Flag any arbitrage bounds violations or model limitations (smile dynamics, early exercise premium)

## Usage
Provide instrument details (underlying, strike, expiry, option type, spot price, volatility) or a multi-leg strategy. The agent will price, compute risk sensitivities, and produce a valuation summary.

$ARGUMENTS
