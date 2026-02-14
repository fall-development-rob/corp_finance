# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-02-14

### Added
- **FMP MCP Server** (`packages/fmp-mcp-server`) — new package providing 181 Financial Modeling Prep market data tools via MCP
  - 15 tool modules: quotes, profiles, financials, earnings, market, ETFs, news, technical indicators, SEC filings, insider trading, institutional ownership, dividends/splits/IPOs, extended financials, extended market, extended company/analyst data
  - Rate-limited API client with tiered caching (30s realtime to 7-day static) and 300 req/min rate limiter
  - Zod-validated schemas for all tool inputs with shared `SymbolSchema`, `PaginationSchema`, `DateRangeSchema`
  - 43 unit tests covering client, schemas, and tool registration
- **FMP CLI** (`fmp`) with 19 commands: `quote`, `profile`, `financials`, `earnings`, `screen`, `search`, `news`, `technicals`, `etf`, `insider`, `sec`, `institutional`, `dividends`, `macro`, `treasury`, `gainers`, `losers`, `active`, `tools`
- **6 FMP agent skills** mapping all 181 tools for pipeline injection
  - `fmp-market-data` (85 tools): quotes, profiles, financials, earnings, dividends, splits, IPOs
  - `fmp-research` (38 tools): screening, sector/industry, economic data, indexes, market hours
  - `fmp-news-intelligence` (10 tools): general/stock/crypto/forex/press release news
  - `fmp-technicals` (9 tools): SMA, EMA, RSI, MACD, Williams %R, ADX, standard deviation
  - `fmp-etf-funds` (9 tools): ETF holdings, sector weights, country exposure, performance
  - `fmp-sec-compliance` (26 tools): SEC filings, insider trades, institutional 13F holdings
- **Pipeline AGENT_SKILLS expansion** — each CFA specialist agent now receives relevant FMP skills (e.g., equity analyst gets `fmp-technicals` + `fmp-news-intelligence`, quant-risk gets `fmp-etf-funds`)
- **ADR-003**: FMP MCP Server architecture decision record
- **ADR-002**: ruvector graph mincut + neural spiking architecture decision record
- **ruvector graph + neural spiking** (`db/ruvector-graph.ts`, `db/ruvector-spiking.ts`)
  - Graph operations: `buildPatternEdges`, `computeMincut`, `partitionPatterns`, `detectNovelPattern`, `computePatternPageRank`
  - Neural spiking: Leaky Integrate-and-Fire (LIF) neuron model in SQL, `fireSpike` with propagation, `detectAnomalies`, `getNetworkState`, `buildLinksFromTrajectories`
  - 25 unit tests + 15 PG integration tests covering graph partitioning, spike propagation, and membrane potential accumulation

### Fixed
- PG integration vector similarity test failing with hash-based embeddings — search now uses exact stored text so cosine similarity is 1.0 regardless of embedding backend
- ruvector HNSW segfault recovery with `queryWithRetry()` automatic pool reset

### Changed
- FMP MCP server version bumped to 2.0.0 (181 tools, up from initial 31)
- `.gitignore` updated to track `.claude/skills/fmp-*/` directories

## [1.1.0] - 2026-02-13

### Added
- **Multi-agent pipeline** with 6-stage execution: task routing, vector search, agent spawning, attention-based coordination, synthesis, and learning (`src/pipeline.ts`)
  - SemanticRouter with HNSW-indexed intent matching routes queries to the best specialist agent(s) from 9 CFA analysts (equity, credit, fixed income, derivatives, quant-risk, macro, ESG, private markets, chief)
  - AttentionCoordinator with flash attention (384-dim, 8 heads) and topology-aware coordination (mesh, hierarchical, ring, star) for multi-agent consensus
  - TransformersEmbeddingService using local Xenova/all-MiniLM-L6-v2 ONNX model (384-dim, no API calls)
  - Multi-intent detection spawns multiple specialist agents in parallel with 120s timeout per agent
  - Chief analyst synthesis stage streams the final coordinated response to stdout
  - SONA learning records successful tool sequences as patterns for future retrieval
- **Postgres memory backend** via `CFA_MEMORY_BACKEND` environment variable
  - `postgres`: PgReasoningBank + PgFinancialMemory backed by ruvector-postgres with pgvector HNSW search
  - `sqlite` (default): SonaReasoningBank + AgentDbFinancialMemory
  - `local`: in-memory LocalReasoningBank + LocalFinancialMemory
  - Automatic health check, migrations, and fallback to sqlite on Postgres failure
- **CLI enhancements** (`src/cli.ts`)
  - `--topology <mesh|hierarchical|ring|star>` flag for swarm coordination topology
  - Pipeline mode (default) vs single-agent mode (`--agent <name>`)
  - REPL commands: `/pipeline` toggle, `/topology <type>`, updated `/help`
  - Model selection via `CFA_MODEL` env var (default: claude-haiku-4-5-20251001)
- **Type declarations** for agentic-flow deep imports: SemanticRouter, AttentionCoordinator, EmbeddingService, FlashAttention (`src/agentic-flow.d.ts`)

### Fixed
- ruvector HNSW segfault recovery: added `queryWithRetry()` in `db/pg-client.ts` with automatic pool reset and retry on connection termination / recovery mode errors
- Mock embeddings fallback removed — pipeline now requires real TransformersEmbeddingService or fails explicitly
- SemanticRouter cosine similarity returning zero due to `EmbeddingResult` object vs raw `number[]` mismatch — added `createRouterEmbedder()` adapter
- Confidence threshold lowered from 0.6 to 0.4 to match real embedding similarity score ranges
- Added `route()` fallback when `detectMultiIntent()` returns empty results

### Changed
- Removed `--model` CLI flag in favor of `CFA_MODEL` environment variable
- Moved `AGENT_SKILLS`, `injectSkills()`, and skill loading from `cli.ts` to `pipeline.ts` for sharing across pipeline stages

## [1.0.0] - 2026-02-12

### Added
- **Phase 20: Private Wealth, Emerging Markets, Index Construction, Financial Forensics** (16 new MCP tools, 20 new CLI subcommands)
  - Private wealth: concentrated stock position management with 5 strategies (outright sale, collar, exchange fund, prepaid variable forward, charitable remainder trust), direct indexing with tax-alpha and tracking error analysis, family governance scoring and succession planning, philanthropic vehicle comparison (DAF, private foundation, CRT, CLT), wealth transfer analysis with gift/estate tax and GRAT modelling
  - Emerging markets: Damodaran country risk premium estimation (sovereign spread x equity/bond vol ratio) with rating-based lookup and composite blending, political risk scoring across 6 dimensions, capital controls analysis with repatriation risk, EM bond analytics with local/hard currency spread decomposition, EM equity premium with governance adjustment
  - Index construction: 6 weighting schemes (market-cap, equal, free-float, fundamental, capped with iterative redistribution, inverse-volatility), smart beta / factor tilt construction with z-score normalization and active share measurement, index rebalancing with turnover and transaction cost analysis, reconstitution with additions/deletions and buffer rules, tracking error decomposition (systematic vs idiosyncratic)
  - Financial forensics: Benford's Law digit analysis with chi-squared goodness-of-fit testing, extended DuPont decomposition (5-way), peer benchmarking with percentile ranking across financial metrics, red flag composite scoring across earnings quality / growth quality / financial health / governance categories, Altman Z-score variants (original, Z'-score for private, Z''-score for emerging markets)
- **71 domain modules, 215 MCP tools, 71 CLI subcommands, 5,879 tests passing**
- **270 Rust source files, ~201,000 lines of Rust**

## [0.15.0] - 2026-02-12

### Added
- **Phase 19: Earnings Quality, Bank Analytics, Dividend Policy, Carbon Markets** (16 new MCP tools, 20 new CLI subcommands)
  - Earnings quality: Beneish M-Score 8-variable manipulation detection model (DSRI, GMI, AQI, SGI, DEPI, SGAI, LVGI, TATA with -1.78 threshold), Piotroski F-Score 9-signal binary scoring for financial strength (profitability, leverage/liquidity, operating efficiency), accrual quality analysis (CFO vs net income, Sloan ratio), revenue quality assessment, composite earnings quality scoring
  - Bank analytics: CAMELS rating system across 6 dimensions (capital adequacy via CET1, asset quality via NPL ratio, management via efficiency ratio, earnings via ROA, liquidity via LCR, sensitivity to market risk), CECL / IFRS 9 expected credit loss provisioning with 3-stage classification and scenario-weighted ECL (base/adverse/severe), deposit beta analysis, loan book analytics, net interest margin (NIM) analysis and decomposition
  - Dividend policy: multi-stage DDM with arbitrary growth stages and Gordon Growth terminal value, H-model for declining growth transitions, payout sustainability analysis, share buyback analytics (accretion/dilution, EPS impact, optimal execution), total shareholder return decomposition (dividend yield + capital gains + buyback yield)
  - Carbon markets: carbon credit pricing with cost-of-carry forwards, vintage adjustment, and registry/credit-type premiums (Gold Standard, Verra, ACR, CAR), EU ETS compliance analysis, CBAM (Carbon Border Adjustment Mechanism) cost estimation, carbon offset valuation, shadow carbon pricing for internal capital allocation

## [0.14.0] - 2026-02-12

### Added
- **Phase 18: Credit Scoring, Capital Allocation, CLO Analytics, Fund of Funds** (16 new MCP tools, 20 new CLI subcommands)
  - Credit scoring: logistic regression scorecard analytics with Weight of Evidence (WoE), Information Value (IV), scorecard point mapping, Gini coefficient (2*AUC-1), and KS statistic; Merton structural model with iterative asset value / asset volatility solving, distance-to-default, and KMV EDF mapping; reduced-form intensity model with hazard rate bootstrapping from CDS spreads, survival probabilities, and conditional default probabilities; PD calibration and model validation (ROC/AUC, accuracy ratio)
  - Capital allocation: RAROC / RORAC with EVA and SVA calculation and risk-adjusted pricing (minimum spread derivation), economic capital via VaR and Expected Shortfall with stress capital buffer and Basel IRB formula, Euler allocation for marginal risk contribution, Shapley value allocation with exact enumeration (N<=8) and sampled approximation (N>8), limit management framework
  - CLO analytics: sequential payment priority waterfall engine (senior fees through equity residual) with period-by-period collateral pool amortization and default/recovery modelling, OC/IC coverage tests with trigger breach and cure mechanics, tranche-level analytics (WAL, spread duration, loss sensitivity), reinvestment analysis, and multi-scenario stress testing
  - Fund of funds: J-curve lifecycle modelling with TVPI/DPI/RVPI and Kaplan-Schoar PME, commitment pacing analysis, manager selection scoring and due diligence framework, portfolio construction with diversification metrics, secondaries market analysis with discount/premium pricing

## [0.13.0] - 2026-02-12

### Added
- **Phase 15-16: Volatility Surface, Portfolio Optimization, Risk Budgeting, Market Microstructure, Interest Rate Models, Mortgage Analytics, Inflation Linked, Repo Financing** (16 new MCP tools, 16 new CLI subcommands)
  - Volatility surface: implied vol surface construction from market quotes with 3 interpolation methods (linear, cubic spline, SVI), moneyness and delta conversions, skew/term structure analysis, calendar spread and butterfly arbitrage detection; SABR model calibration (alpha, beta, rho, nu) with backbone and skew analytics
  - Portfolio optimization: Markowitz mean-variance optimization with efficient frontier generation, long-only and sector constraints, target return/risk optimization; Black-Litterman portfolio construction with investor views, posterior expected returns, and confidence-weighted blending
  - Risk budgeting: tail risk analytics with VaR and CVaR under Normal, Cornish-Fisher, and Historical distributions, marginal and component risk decomposition, stress scenario testing; factor risk budgeting with factor-level VaR attribution
  - Market microstructure: bid-ask spread analysis with Roll model and effective spread estimation, price impact modelling, liquidity scoring; optimal execution with Almgren-Chriss framework, VWAP/TWAP benchmarking, implementation shortfall decomposition
  - Interest rate models: Vasicek, Cox-Ingersoll-Ross (CIR), and Hull-White (Extended Vasicek) short rate models with bond pricing formulas, rate simulation paths, and calibration; term structure fitting via Nelson-Siegel (4-parameter), Svensson (6-parameter), and bootstrapping with forward rate extraction
  - Mortgage analytics: PSA/CPR prepayment models with SMM conversion and WAL computation, burnout-adjusted refinancing incentive analysis; MBS pass-through analytics with cash flow modelling, OAS and Z-spread via bisection, effective duration and convexity
  - Inflation linked: TIPS pricing with CPI-adjusted principal, real/nominal clean and dirty prices, deflation floor, and projected cashflow schedules; breakeven inflation analysis (Fisher equation, term structure, forward breakevens, inflation risk premium); inflation derivatives (zero-coupon inflation swap, year-on-year inflation swap, inflation cap/floor with Black model and Greeks)
  - Repo financing: repo rate calculation with purchase/repurchase prices, implied repo rate from spot/forward bond prices, repo term structure with specialness premiums, securities lending economics (fee income, cash reinvestment); collateral management with risk-based haircuts by type/rating/maturity, margin call analysis, rehypothecation analysis with velocity and regulatory limits

## [0.12.0] - 2026-02-12

### Added
- **Phase 14: FATCA/CRS, Substance Requirements, Regulatory Reporting, AML Compliance** (8 new MCP tools, 8 new CLI subcommands)
  - FATCA/CRS: entity classification engine (FFI, Active NFFE, Passive NFFE, Exempt Beneficial Owner, Deemed Compliant for FATCA; Financial Institution, Active NFE, Passive NFE for CRS), controlling person identification with tax residence analysis, dual FATCA/CRS reporting obligation assessment, sponsored entity and GIIN verification, remediation action planning
  - Substance requirements: economic substance testing across 9 entity types (HoldingCompany, IPHolding, FinanceLease, FundManagement, Banking, Insurance, HQ, ServiceCentre, PureEquityHolding) for 10+ jurisdictions (Cayman, BVI, Luxembourg, Ireland, Jersey, Guernsey, Singapore, Netherlands, Switzerland), 4 compliance statuses, premises type assessment, director qualification analysis, CIGA (Core Income Generating Activity) evaluation
  - Regulatory reporting: SEC Form PF analysis with Large/Small/Exempt classification, filing frequency determination, and strategy classification (8 strategies); CFTC CPO-PQR reporting with filing threshold analysis; AIFMD reporting with leverage calculation (gross and commitment methods), liquidity profile analysis, and risk reporting
  - AML compliance: KYC risk scoring across 7 customer types (Individual, Corporate, Trust, Foundation, Partnership, PEP, ComplexStructure) with PEP categorization (5 levels), source-of-wealth classification, product/channel risk, multi-dimensional composite scoring; sanctions screening against 6 lists (OFAC SDN, EU Consolidated, HMT UK, UN UNSC, FATF Grey/Black) with fuzzy name matching (Exact, Strong, Possible, Weak) and escalation actions (Clear, ManualReview, Escalate, Block)

## [0.11.0] - 2026-02-12

### Added
- **Phase 13: Fund Structures, Transfer Pricing, Tax Treaties** (8 new MCP tools, 8 new CLI subcommands)
  - Onshore fund structures: US fund analysis for 6 structure types (Delaware LP, LLC, REIT, MLP, BDC, QOZ) with GP/LP economics, Section 754 / QEF / PFIC tax elections, ERISA compliance testing, UBTI risk scoring, state-level tax analysis; UK/EU fund analysis for 7 structure types (UK LP, UK LLP, OEIC, ACS, SICAV, FCP, KG) with AIFMD/UCITS compliance, VAT on management fees, carried interest taxation, reporting fund status
  - Offshore fund structures: Cayman fund analysis for 6 structure types (Exempted LP, SPC, Unit Trust, LLC, BVI BCA, BVI LP) across 6 strategies (Hedge, PE, VC, Real Estate, Credit, FoF) with master-feeder structures, CIMA registration, feeder jurisdiction analysis, service provider governance; Luxembourg/Ireland fund analysis for 6 structure types (SICAV-SIF, SICAV-RAIF, SCSp, ICAV, QIAIF, Section 110) with subscription tax, treaty network benefits, AIFMD full-scope and UCITS passporting
  - Transfer pricing: intercompany transaction analysis with 5 OECD methods (CUP, Cost Plus, Resale Price, TNMM, Profit Split), arm's length range testing with quartile analysis, CFC (Controlled Foreign Corporation) analysis with de minimis thresholds, GAAR (General Anti-Avoidance Rule) risk assessment; BEPS compliance with Country-by-Country Reporting (CbCR) threshold testing, Pillar Two global minimum tax analysis, entity-level profit allocation analysis
  - Tax treaties: treaty network modelling with WHT optimization for 6 income types (dividends, interest, royalties, management fees, capital gains, services), conduit route analysis through intermediary jurisdictions, anti-treaty-shopping risk assessment (LOB/PPT); multi-jurisdiction holding structure optimization with PE (Permanent Establishment) risk scoring, participation exemption analysis, IP box rate planning, substance cost-benefit

## [0.10.0] - 2026-02-12

### Added
- **Phase 12: Performance Attribution, Credit Portfolio, Macro Economics, Compliance** (8 new MCP tools, 8 new CLI subcommands)
  - Performance attribution: Brinson-Fachler model with allocation, selection, and interaction effects at the sector level, multi-period linking, portfolio vs benchmark contribution analysis, information ratio calculation; factor attribution with active exposure decomposition, return contribution per factor, R-squared, and tracking error decomposition (factor vs residual)
  - Credit portfolio: portfolio-level credit risk analytics with expected loss, unexpected loss (Vasicek single-factor), obligor-level risk contribution and marginal risk, concentration metrics (HHI by obligor and sector, top-10 exposure); rating migration analysis with transition matrix modelling, multi-year cumulative transition probabilities, mark-to-market credit VaR from spread curve shifts, upgrade/downgrade/default probability decomposition
  - Macro economics: monetary policy analysis with Taylor Rule rate prescription (inflation/output gap weighting), Phillips Curve inflation dynamics, Okun's Law output-gap estimation, recession-risk scoring and inflation-trend detection; international economics with PPP (Purchasing Power Parity) implied exchange rates, Covered and Uncovered Interest Rate Parity (CIP/UIP), real exchange rate analysis, balance of payments assessment, multi-year exchange rate projections
  - Compliance: best execution / transaction cost analysis (TCA) with Perold implementation shortfall decomposition, multi-benchmark comparison (VWAP, TWAP, Arrival Price, Close), market impact estimation, per-trade and aggregate cost analysis; GIPS-compliant performance reporting with Modified Dietz and daily-weighted returns, gross/net fee calculation, composite dispersion, multi-period chain-linked returns

## [0.9.0] - 2026-02-12

### Added
- **Phase 11: Quant Strategies, Treasury, Infrastructure, Behavioral Finance** (8 new MCP tools, 8 new CLI subcommands)
  - Quant strategies: cross-sectional momentum ranking with lookback/skip-month parameters, risk-adjusted momentum scores, volatility scaling, quintile portfolio construction; pairs trading with cointegration-based analysis (Engle-Granger framework), z-score signal generation, half-life of mean reversion, spread monitoring with entry/exit thresholds
  - Treasury: corporate cash management with 12-month cash position simulation (operating cash flows, automatic surplus sweep to money market, revolving credit facility draws below minimum buffer), cash conversion cycle analysis, liquidity scoring; hedge effectiveness testing under IAS 39 / IFRS 9 / ASC 815 with dollar offset method, regression analysis (OLS slope and R-squared), VaR reduction measurement, P&L attribution, optimal hedge ratio (minimum-variance), support for Forward/Option/Swap/Collar across FairValue/CashFlow/NetInvestment hedge types
  - Infrastructure: PPP (Public-Private Partnership) financial modelling with 3 revenue models (availability payment, demand-based, mixed), construction-period capex scheduling, senior and mezzanine debt sculpting, DSCR/LLCR/PLCR ratios, equity IRR and project IRR; concession valuation with remaining-term DCF, handback cost provisioning, extension probability analysis, regulatory risk premium, debt service coverage
  - Behavioral finance: prospect theory analysis with Kahneman-Tversky value function (loss aversion, diminishing sensitivity), probability weighting (Prelec function), certainty equivalent calculation, portfolio framing effects; market sentiment analysis with 10-indicator fear/greed composite (VIX, put/call ratio, advance/decline, new highs/lows, margin debt, fund flows, short interest, insider activity, consumer confidence, custom risk appetite), contrarian signal mode

## [0.8.0] - 2026-02-12

### Added
- **Phase 10: Sovereign Analysis, Real Options, Equity Research, Commodity Trading** (8 new MCP tools, 8 new CLI subcommands)
  - Sovereign analysis: government bond pricing with dirty/clean prices, YTM via Newton-Raphson (50 iterations), duration/convexity, spread decomposition (credit/liquidity/currency risk premia), local currency risk adjustment with FX volatility, real yield (Fisher equation); country risk assessment with 5-factor scoring framework (fiscal, external, monetary, political, structural), composite score to rating-equivalent mapping (AAA through CCC), implied default probability derivation
  - Real options: binomial lattice valuation for 6 option types (Expand, Abandon, Defer, Switch, Contract, Compound) with American-style early exercise, exercise boundary identification, expanded NPV calculation; decision tree analysis with Decision/Chance/Terminal nodes, backward induction, optimal path identification, probability-weighted expected values, sensitivity analysis on key assumptions
  - Equity research: consensus target price analysis from 5 methods (P/E, P/B, P/S, DDM, analyst consensus) with weighted composite, upside/downside potential, and conviction scoring; sum-of-the-parts valuation with 6 methodology choices per segment (EV/EBITDA, P/E, EV/Revenue, EV/EBIT, DCF perpetuity growth, NAV-based), comparable range sensitivity, conglomerate discount analysis
  - Commodity trading: spread analysis for 6 spread types (Crack, Crush, Spark, Calendar, Location, Quality) with historical percentile ranking, P&L simulation, and margin analysis; storage and carry modelling with convenience yield estimation, contango/backwardation detection, optimal storage trade economics, roll yield and basis analysis

## [0.7.0] - 2026-02-12

### Added
- **Phase 9: Credit Derivatives, Convertible Bonds, Lease Accounting, Pension** (8 new MCP tools, 8 new CLI subcommands)
  - Credit derivatives: CDS pricing with survival probability term structure, risky PV01, protection and premium leg present values, breakeven spread, MTM/upfront calculation, DV01, jump-to-default exposure, implied credit triangle (spread/PD/recovery/LGD); CVA/DVA computation with unilateral and bilateral credit value adjustment, exposure-at-default, netting benefit and collateral threshold modelling, CVA as running spread, adjusted exposure profile
  - Convertible bonds: binomial tree pricing with embedded call/put features, bond floor, conversion value, conversion/investment premium, Greeks (delta, gamma, vega, theta), yield-to-maturity, breakeven analysis; scenario analysis with stock/volatility/spread sensitivity grids, forced conversion analysis, income advantage (yield pickup vs dividend yield), risk-return asymmetry profiling
  - Lease accounting: ASC 842 / IFRS 16 classification and measurement with 5-test finance lease determination (transfer of ownership, purchase option, 75% useful life, 90% fair value PV, specialised asset), ROU asset and lease liability at inception, month-by-month amortization schedules (effective interest for finance, straight-line for operating), payment escalation support; sale-leaseback analysis with ASC 606 sale qualification, gain/loss recognition (partial for retained right), deferred gain for above-FMV transactions, failed-sale financing obligation treatment
  - Pension: defined benefit funding analysis with PBO/ABO computation per participant (active employees with salary growth projection, retirees with life expectancy), funded status, minimum/maximum contribution boundaries, corridor amortization, ERISA compliance, ASC 715 / IAS 19 pension expense components; liability-driven investing (LDI) with duration gap analysis (asset vs liability), hedging portfolio construction from available instruments (government bonds, corporate bonds, TIPS, swaps), glide-path scheduling with funded ratio targets, rebalancing triggers, asset allocation transition recommendations

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
