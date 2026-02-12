# corp-finance-mcp

Institutional-grade corporate finance calculations exposed as an MCP (Model Context Protocol) server (v1.0.0). All financial math runs in 128-bit decimal precision via Rust, with Node.js bindings and a TypeScript MCP interface for AI-assisted financial analysis. 71 domain modules, 215 MCP tools, 71 CLI subcommands, 5,879 tests across ~201,000 lines of Rust.

## Architecture

```
crates/corp-finance-core    Rust library — all financial math (Decimal, no f64*)
crates/corp-finance-cli     Rust CLI — command-line interface
packages/bindings           napi-rs — Node.js native bindings (JSON string boundary)
packages/mcp-server         TypeScript — MCP server with Zod schema validation
```

\* Monte Carlo simulation uses f64 for performance with `rand`/`statrs`.

## Modules (71 features, 215 MCP tools)

| Phase | Module | Feature Flag | MCP Tools | Description |
|-------|--------|-------------|-----------|-------------|
| 1 | Valuation | `valuation` | 3 | WACC (CAPM), DCF (FCFF), trading comps |
| 1 | Credit | `credit` | 3 | Metrics, debt capacity, covenants, Altman Z |
| 1 | Private Equity | `pe` | 6 | IRR/MOIC, debt schedules, sources & uses, LBO, waterfall |
| 1 | Portfolio | `portfolio` | 3 | Risk-adjusted returns, VaR/CVaR, Kelly sizing |
| 1 | Scenarios | `scenarios` | 2 | Sensitivity matrix, scenario analysis |
| 2 | M&A | `ma` | 1 | Merger accretion/dilution |
| 2 | Jurisdiction | `jurisdiction` | 7 | GAAP/IFRS, WHT, NAV, GP economics, UBTI |
| 3 | Fixed Income | `fixed_income` | 6 | Bond pricing, yields, duration, spreads, curves, Nelson-Siegel |
| 3 | Derivatives | `derivatives` | 8 | Options, implied vol, forwards, swaps, basis, strategies |
| 4 | Three Statement | `three_statement` | 1 | Integrated IS/BS/CF model |
| 4 | Monte Carlo | `monte_carlo` | 2 | Generic simulation, stochastic DCF |
| 4 | Quant Risk | `quant_risk` | 4 | Factor models, Black-Litterman, risk parity, stress testing |
| 5 | Restructuring | `restructuring` | 2 | Recovery waterfall, distressed debt |
| 5 | Real Assets | `real_assets` | 2 | Property valuation, project finance |
| 5 | FX & Commodities | `fx_commodities` | 4 | FX forwards, cross rates, commodity forwards/curves |
| 6 | Securitization | `securitization` | 2 | ABS/MBS cash flows, CDO/CLO tranching |
| 6 | Venture Capital | `venture` | 5 | Funding rounds, dilution, convertibles, SAFEs, fund returns |
| 6 | ESG | `esg` | 4 | Scoring, carbon, green bonds, SLL |
| 6 | Regulatory | `regulatory` | 4 | Basel III capital, LCR, NSFR, ALM |
| 7 | Private Credit | `private_credit` | 3 | Unitranche, direct lending, syndication |
| 7 | Insurance | `insurance` | 4 | Loss reserving, premium pricing, combined ratio, SCR |
| 7 | FP&A | `fpa` | 4 | Variance analysis, break-even, working capital, forecast |
| 7 | Wealth | `wealth` | 3 | Retirement planning, tax-loss harvesting, estate planning |
| 8 | Crypto | `crypto` | 2 | Token valuation (NVT/Metcalfe/DCF), DeFi yield/staking |
| 8 | Trade Finance | `trade_finance` | 2 | Letters of credit, supply chain finance/forfaiting |
| 8 | Structured Products | `structured_products` | 2 | Structured notes, exotic derivatives (autocallables, barriers) |
| 8 | Municipal | `municipal` | 2 | Muni bond pricing/TEY, GO/revenue bond analysis |
| 9 | Credit Derivatives | `credit_derivatives` | 2 | CDS pricing, CVA/DVA calculation |
| 9 | Convertibles | `convertibles` | 2 | Convertible bond pricing (CRR), scenario analysis |
| 9 | Lease Accounting | `lease_accounting` | 2 | ASC 842/IFRS 16 classification, sale-leaseback |
| 9 | Pension | `pension` | 2 | Pension funding (PBO/ABO/NPPC), LDI strategy |
| 10 | Sovereign | `sovereign` | 2 | Sovereign bond analysis, country risk assessment |
| 10 | Real Options | `real_options` | 2 | Real option valuation, decision tree analysis |
| 10 | Equity Research | `equity_research` | 2 | Sum-of-the-parts (SOTP), target price |
| 10 | Commodity Trading | `commodity_trading` | 2 | Commodity spreads, storage economics |
| 11 | Quant Strategies | `quant_strategies` | 2 | Pairs trading (cointegration), momentum factor |
| 11 | Treasury | `treasury` | 2 | Cash management, hedge effectiveness |
| 11 | Infrastructure | `infrastructure` | 2 | PPP/PFI project models, concession valuation |
| 11 | Behavioral | `behavioral` | 2 | Prospect theory, market sentiment (Fear & Greed) |
| 12 | Performance Attribution | `performance_attribution` | 2 | Brinson-Fachler, factor-based attribution |
| 12 | Credit Portfolio | `credit_portfolio` | 2 | Portfolio credit risk (Gaussian copula), migration |
| 12 | Macro Economics | `macro_economics` | 2 | Monetary policy (Taylor Rule), international economics |
| 12 | Compliance | `compliance` | 2 | MiFID II best execution, GIPS reporting |
| 13 | Onshore Structures | `onshore_structures` | 2 | US fund structures (LP, REIT, MLP, BDC), UK/EU funds |
| 13 | Offshore Structures | `offshore_structures` | 2 | Cayman/BVI funds (SPC, Exempted LP), Lux/Ireland (SICAV) |
| 13 | Transfer Pricing | `transfer_pricing` | 2 | OECD BEPS compliance, intercompany pricing (CUP, TNMM) |
| 13 | Tax Treaty | `tax_treaty` | 2 | Treaty network analysis, WHT optimization |
| 14 | FATCA/CRS | `fatca_crs` | 2 | FATCA/CRS reporting, entity classification |
| 14 | Substance Requirements | `substance_requirements` | 2 | Economic substance (BEPS Action 5), jurisdiction tests |
| 14 | Regulatory Reporting | `regulatory_reporting` | 2 | AIFMD Annex IV, SEC/CFTC (Form PF, ADV, CPO-PQR) |
| 14 | AML Compliance | `aml_compliance` | 2 | KYC risk assessment, sanctions screening (OFAC/EU/UN) |
| 15 | Volatility Surface | `volatility_surface` | 2 | Implied vol surface construction, SABR calibration |
| 15 | Portfolio Optimization | `portfolio_optimization` | 2 | Mean-variance (Markowitz), Black-Litterman portfolios |
| 15 | Risk Budgeting | `risk_budgeting` | 2 | Factor-based risk budgets, tail risk (VaR/CVaR) |
| 16 | Market Microstructure | `market_microstructure` | 2 | Spread decomposition, optimal execution (Almgren-Chriss) |
| 16 | Interest Rate Models | `interest_rate_models` | 2 | Short rate models (Vasicek/CIR/HW), term structure fitting |
| 16 | Mortgage Analytics | `mortgage_analytics` | 2 | Prepayment analysis (PSA/CPR), MBS pass-through analytics |
| 16 | Inflation Linked | `inflation_linked` | 2 | TIPS analytics, inflation derivatives (ZCIS, caps/floors) |
| 16 | Repo Financing | `repo_financing` | 2 | Repo rate analytics, collateral management |
| 18 | Credit Scoring | `credit_scoring` | 5 | Scorecards (WoE/IV), Merton PD, intensity models, validation |
| 18 | Capital Allocation | `capital_allocation` | 5 | Economic capital, RAROC, Euler/Shapley allocation, limits |
| 18 | CLO Analytics | `clo_analytics` | 5 | CLO waterfall, OC/IC tests, reinvestment, tranche, scenarios |
| 18 | Fund of Funds | `fund_of_funds` | 5 | J-Curve, commitment pacing, manager selection, secondaries |
| 19 | Earnings Quality | `earnings_quality` | 5 | Beneish M-Score, Piotroski F-Score, accrual/revenue quality |
| 19 | Bank Analytics | `bank_analytics` | 5 | NIM analysis, CAMELS rating, CECL provisioning, deposit beta |
| 19 | Dividend Policy | `dividend_policy` | 5 | H-Model DDM, multi-stage DDM, buyback, payout, TSR |
| 19 | Carbon Markets | `carbon_markets` | 5 | Carbon pricing, ETS compliance, CBAM, offsets, shadow price |
| 20 | Private Wealth | `private_wealth` | 5 | Concentrated stock, philanthropy, wealth transfer, direct indexing |
| 20 | Emerging Markets | `emerging_markets` | 5 | Country risk premium, political risk, EM bonds/equity |
| 20 | Index Construction | `index_construction` | 5 | Weighting, rebalancing, tracking error, smart beta, reconstitution |
| 20 | Financial Forensics | `financial_forensics` | 5 | Benford's Law, DuPont, Z-Scores, peer benchmarking, red flags |

## Quick Start

### As an MCP Server (for Claude, Cursor, etc.)

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "corp-finance": {
      "command": "node",
      "args": ["/path/to/packages/mcp-server/dist/index.js"]
    }
  }
}
```

### Build from Source

```bash
# Build Rust workspace
cargo build --workspace --all-features

# Build Node.js bindings
cd packages/bindings && npm install && npm run build

# Build MCP server
cd packages/mcp-server && npm install && npm run build

# Start MCP server
cd packages/mcp-server && npm start
```

### Run Tests

```bash
# All Rust tests (5,879 tests)
cargo test --workspace --all-features

# Clippy lint
cargo clippy --workspace --all-features -- -D warnings

# Format check
cargo fmt --all --check
```

### CLI Usage

```bash
cargo run -p corp-finance-cli -- wacc \
  --risk-free-rate 0.04 \
  --equity-risk-premium 0.055 \
  --beta 1.2 \
  --cost-of-debt 0.06 \
  --tax-rate 0.25 \
  --debt-weight 0.4 \
  --equity-weight 0.6
```

## Design Principles

- **Decimal precision**: All financial math uses `rust_decimal::Decimal` (128-bit) to avoid floating-point drift. Discount factors use iterative multiplication, never `powd()`.
- **Feature-gated modules**: Each domain is behind a Cargo feature flag. Use `--all-features` or pick what you need.
- **JSON boundary**: napi bindings use `String -> String` (JSON in, JSON out) for clean interop. Zod schemas validate on the TypeScript side.
- **Computation metadata**: Every tool returns a `ComputationOutput<T>` envelope with methodology description, assumptions, warnings, version, computation time, and precision indicator.

## Key Implementation Notes

| Topic | Approach |
|-------|----------|
| NPV / discount factors | Iterative multiplication (`df *= 1/(1+r)`), never `powd()` |
| Square root | Newton's method, 20 iterations |
| exp / ln | Taylor series, 40 iterations |
| Normal CDF | Abramowitz & Stegun polynomial approximation |
| Matrix inverse | Gauss-Jordan with partial pivoting (all in Decimal) |
| American options | CRR binomial tree |
| Implied volatility | Newton-Raphson (50 iterations) |
| Monte Carlo | f64 with `rand`/`statrs` for performance |
| Interest expense circularity | 5-iteration convergence (three-statement model) |
| Debt sculpting | Level, sculpted (target DSCR), or bullet |

## CI/CD

- **CI** (`ci.yml`): Format check, Clippy, tests on Ubuntu + macOS matrix. Cargo audit. napi bindings build with artifact passing. MCP server smoke test.
- **Publish** (`publish.yml`): Multi-platform napi builds (x86_64/aarch64, Linux/macOS), npm publish with provenance, crates.io publish, GitHub releases.
- **Release** (`release.yml`): Automated version bumping, changelog generation, tag creation. Dry-run by default.
- **Dependabot**: Weekly updates for Cargo, npm, and GitHub Actions dependencies.

## Project Structure

```
.
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── corp-finance-core/        # Rust library (all financial math)
│   │   └── src/
│   │       ├── valuation/        # WACC, DCF, comps
│   │       ├── credit/           # Metrics, capacity, covenants, Altman
│   │       ├── pe/               # Returns, LBO, debt schedules
│   │       ├── derivatives/      # Options, forwards, swaps
│   │       ├── fixed_income/     # Bonds, yields, duration, spreads
│   │       ├── quant_risk/       # Factor models, BL, risk parity
│   │       ├── securitization/   # ABS/MBS, CDO/CLO tranching
│   │       ├── venture/          # Funding rounds, SAFEs, dilution
│   │       ├── credit_scoring/   # Scorecards, Merton PD, validation
│   │       ├── clo_analytics/    # CLO waterfall, tranche analytics
│   │       ├── financial_forensics/ # Benford's Law, DuPont, red flags
│   │       ├── emerging_markets/ # Country risk, EM bonds/equity
│   │       ├── index_construction/ # Weighting, smart beta, rebalancing
│   │       ├── private_wealth/   # Concentrated stock, philanthropy
│   │       └── ...               # 57 more domain modules (71 total)
│   └── corp-finance-cli/         # CLI binary
├── packages/
│   ├── bindings/                 # napi-rs Node.js bindings
│   └── mcp-server/              # TypeScript MCP server
│       └── src/
│           ├── tools/            # One file per domain
│           └── schemas/          # Zod validation schemas
└── .github/workflows/            # CI, publish, release
```

## License

MIT
