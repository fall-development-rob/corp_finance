# Derivatives Valuation

Perform derivatives pricing and analysis using the `corp-finance-tools-markets` skill. Covers options valuation, Greeks, volatility surfaces, and structured product decomposition.

## What It Does
Executes a multi-step derivatives assessment: (1) Options pricing via Black-Scholes (European) and CRR binomial (American), (2) Full Greeks calculation (delta, gamma, vega, theta, rho), (3) Implied volatility extraction and volatility surface construction (SABR calibration), (4) Strategy payoff analysis for multi-leg positions, (5) Structured product pricing and risk decomposition.

## Agent
Routes to `cfa-derivatives-analyst` with `corp-finance-tools-markets` skill.

## Key Tools
`options_pricing`, `greeks_calculator`, `implied_volatility`, `volatility_surface`, `sabr_model`, `forward_pricing`, `swap_pricing`, `strategy_payoff`, `structured_notes`, `exotic_products`, `sensitivity_matrix`, `monte_carlo_simulation`, `yf_options_chain`, `yf_quote`

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
