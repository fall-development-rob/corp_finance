# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-02-11

### Added
- **Phase 8: Crypto, Municipal, Structured Products, Trade Finance** (8 new MCP tools, 152 new tests)
  - Crypto: token/protocol valuation (NVT ratio, P/S, FDV vs circulating, DCF of protocol revenue, relative valuation via comparable protocols), DeFi analysis (yield farming with gas-adjusted APR/APY, impermanent loss, staking with validator commission and slashing risk, liquidity pool fee income and pool share)
  - Municipal bonds: pricing with tax-equivalent yield, de minimis rule, callable analysis, muni/Treasury and muni/corporate spreads (GO, Revenue, Assessment, TIF, COP types), credit analysis with GO debt ratios, revenue bond DSCR and rate covenant, 10-factor composite credit scoring, advance refunding savings
  - Structured products: note pricing for capital-protected (zero-coupon bond + call), yield enhancement / reverse convertible (short put + bond), participation, and credit-linked notes; exotic products including autocallable (observation schedule with knock-in probability), barrier options (up/down in/out with Greeks), digital options (cash-or-nothing, asset-or-nothing)
  - Trade finance: letter of credit pricing for 5 LC types (commercial, standby, revolving, back-to-back, transferable) with multi-dimensional risk scoring and banker's acceptance discounting; supply chain finance (reverse factoring, dynamic discounting, forfaiting with avalised/non-avalised analysis, export credit with ECA/CIRR/OECD terms)
- 91 MCP tools total, 27 Cargo features, 1,548 tests passing

## [0.5.0] - 2026-02-10

### Added
- **Phase 7: Private Credit, Insurance, FP&A, Wealth Management** (14 new MCP tools, 228 new tests)
  - Private credit: unitranche FOLO pricing, direct lending (PIK toggle, delayed draw, rate floors), syndication analysis
  - Insurance: chain-ladder and Bornhuetter-Ferguson loss reserving, premium pricing (freq x severity), combined ratio analysis, Solvency II SCR standard formula
  - FP&A: budget variance with price/volume/mix decomposition, break-even and DOL analysis, working capital efficiency (DSO/DIO/DPO/CCC), driver-based rolling forecast
  - Wealth management: retirement planning with 4 withdrawal strategies (Constant Dollar, Constant Percentage, Guardrails, RMD), tax-loss harvesting with wash-sale rules, estate planning with 7 trust types and GST tax
- Skill documentation for all 83 tools in `docs/skills/`

## [0.4.0] - 2026-02-10

### Added
- **Phase 6: Securitization, Venture Capital, ESG, Regulatory** (15 new MCP tools, 289 new tests)
  - Securitization: ABS/MBS pool cash flow projection (CPR/PSA/SMM prepayment, CDR/SDA default models), CDO/CLO tranching waterfall with OC/IC triggers and reserve accounts
  - Venture capital: round-by-round dilution with option pool shuffle, SAFE and convertible note conversion, VC fund return analytics (J-curve, TVPI/DPI/RVPI, carry)
  - ESG: sector-specific ESG scoring (9 sectors, 7 rating bands), carbon footprint analysis (Scope 1/2/3), green bond framework, SLL covenant testing
  - Regulatory: Basel III capital adequacy (SA risk weights, BIA/SA operational risk, CRM), LCR/NSFR liquidity ratios, ALM gap/NII sensitivity/EVE analysis

## [0.3.0] - 2026-02-10

### Added
- **Phase 5: Advanced Analytics** (16 new MCP tools, 350 new tests)
  - Three-statement financial model with circular reference resolution (5-iteration convergence)
  - Monte Carlo simulation (generic + stochastic DCF) using f64 for performance
  - Quantitative risk: multi-factor models (CAPM, FF3, Carhart4), Black-Litterman optimisation, risk parity (ERC), stress testing with 5 built-in historical scenarios
  - Restructuring: APR recovery waterfall, distressed debt analysis with fulcrum identification
  - Real assets: property valuation (direct cap, DCF, GRM), project finance with 3 debt sculpting methods
  - FX/commodities: FX forwards (CIP), cross rates, commodity forwards (cost-of-carry), term structure analysis

## [0.2.0] - 2026-02-10

### Added
- **Phase 4: Fixed Income & Derivatives** (14 new MCP tools, 220 new tests)
  - Fixed income: bond pricing (clean/dirty, day count), yield analysis (YTM, BEY), duration/convexity/DV01/key rate, credit spreads (Z-spread, OAS, I-spread, G-spread), yield curve bootstrapping, Nelson-Siegel fitting
  - Derivatives: option pricing (Black-Scholes, CRR binomial, Greeks), implied volatility (Newton-Raphson), forward/futures pricing (cost-of-carry), futures basis analysis, interest rate swaps (par rate, DV01), currency swaps, option strategy payoff analysis (12 built-in strategies)

### Removed
- Trading diary module (removed as out of scope)

## [0.1.0] - 2026-02-10

### Added
- **Phase 1-3: Core Finance Modules** (24 MCP tools, 81 integration tests + 320 unit tests)
  - Valuation: WACC (CAPM), DCF (FCFF), trading comps
  - Credit: metrics suite + synthetic rating, debt capacity, covenant compliance, Altman Z-score
  - Private equity: IRR/XIRR/MOIC returns, debt schedules, sources & uses, LBO model, waterfall distributions
  - M&A: merger accretion/dilution model
  - Portfolio: risk-adjusted returns (Sharpe, Sortino, Calmar), risk metrics (VaR, CVaR), Kelly sizing
  - Fund economics: fee calculator, GP economics, investor net returns
  - Jurisdiction: GAAP/IFRS reconciliation, withholding tax, NAV with equalisation, UBTI/ECI screening
  - Scenarios: sensitivity matrix, scenario analysis
- TypeScript MCP server with Zod validation
- napi-rs bridge with JSON-boundary functions
- CLI binary (`cfa`) with 4 output formats (json, table, csv, minimal)
- 81 integration tests with known-answer fixtures
- Pre-commit hooks for formatting, linting, and TypeScript checks

### Fixed
- Resolved all clippy warnings and fixed pre-commit hook

### Changed
- Applied cargo fmt across entire workspace
- Added git hooks for conventional commits and linting
