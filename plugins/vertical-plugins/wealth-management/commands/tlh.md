# Tax-Loss Harvesting

Run a tax-loss harvesting workflow using the TLH workflow from `workflow-wealth-management`.

## What It Does
Produces a TLH opportunity list: realised and unrealised gain/loss inventory split by holding period, gain/loss budget vs YTD realised gains and prior-year carryforwards, candidate ranking by tax benefit, replacement-security pairing that preserves market exposure without violating wash-sale, and after-tax alpha estimate at the client's marginal rate.

## Agent
Routes to `cfa-quant-risk-analyst` with `workflow-wealth-management` skill.

## Key Tools
`tax_loss_harvesting`, `risk_metrics`, `estate_planning`

## Quality Standards
- Short-term losses prioritised (offset ordinary income up to $3,000)
- 30-day wash-sale check across taxable, IRA, Roth, 401(k), and DRIPs
- Replacement security: similar but not substantially identical (correlation check via `risk_metrics`)
- Estimated tax savings = loss amount x marginal tax rate

## Usage
Provide holdings with cost basis and lot dates, YTD realised gains, prior-year carryforwards, marginal tax rate, and any restricted securities.

$ARGUMENTS
