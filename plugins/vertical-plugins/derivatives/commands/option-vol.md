# Option Volatility Analysis

Analyse equity or index option volatility using the `corp-finance-analyst-markets` skill.

## What It Does
Executes an option-vol workflow: (1) implied volatility surface construction across strikes and expiries, (2) skew and term-structure characterisation, (3) SABR calibration and parameter stability, (4) dispersion analysis (index vol vs weighted single-name vol), (5) calendar-spread and skew-trade construction with Greeks-neutral hedging, (6) sensitivity grid for vega/gamma/theta exposure.

## Agent
Routes to `cfa-fixed-income-analyst` with `corp-finance-analyst-markets` skill.

## Key Tools
`implied_vol_surface`, `sabr_model`, `option_pricing`, `sensitivity_matrix`, `lseg_options_chain`

## Quality Standards
- State option model (Black-Scholes, binomial, SABR) and dividend assumption
- Show ATM vol, 25-delta risk reversal, 25-delta butterfly per expiry
- Greeks aggregated at the portfolio level, not just per leg
- Flag liquidity and bid-ask wideness for any trade leg

## Usage
Provide underlying (index or ticker), expiry range, strike range or moneyness band, and trade idea (calendar/skew/dispersion).

$ARGUMENTS
