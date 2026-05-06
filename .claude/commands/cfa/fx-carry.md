# FX Carry Trade

Evaluate FX carry trade opportunities using the `vendor-lseg` and `corp-finance-analyst-markets` skills.

## What It Does
Executes a carry-trade screen: (1) interest-rate-differential ranking across major and EM crosses, (2) volatility-adjusted carry (carry-to-vol ratio), (3) trade construction long the high-yielder vs short the funding currency, (4) forward implied yield decomposition vs spot+rate-differential, (5) risk metrics (drawdown, tail risk, correlation to risk-on/risk-off regimes).

## Agent
Routes to `cfa-fixed-income-analyst` with `vendor-lseg` and `corp-finance-analyst-markets` skills.

## Key Tools
`lseg_fx_rates`, `lseg_economic_indicators`, `fx_forward`, `cross_rate`, `risk_metrics`, `international_economics`

## Quality Standards
- State day-count and forward convention used
- Carry-to-vol ranking, not raw carry alone
- Flag CIP deviations and cross-currency basis where material
- Identify funding stress scenarios (e.g., USD squeeze, JPY repatriation)

## Usage
Provide currency pair universe (or default to G10 + selected EM), funding currency, holding horizon, and notional sizing approach.

$ARGUMENTS
