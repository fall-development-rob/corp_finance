# corp-finance-mcp

Institutional-grade corporate finance calculations exposed as an MCP (Model Context Protocol) server. All financial math runs in 128-bit decimal precision via Rust, with Node.js bindings and a TypeScript MCP interface for AI-assisted financial analysis.

## Architecture

```
crates/corp-finance-core    Rust library — all financial math (Decimal, no f64*)
crates/corp-finance-cli     Rust CLI — command-line interface
packages/bindings           napi-rs — Node.js native bindings (JSON string boundary)
packages/mcp-server         TypeScript — MCP server with Zod schema validation
```

\* Monte Carlo simulation uses f64 for performance with `rand`/`statrs`.

## Modules (27 features, 95+ MCP tools)

| Phase | Module | Feature Flag | MCP Tools | Description |
|-------|--------|-------------|-----------|-------------|
| 1 | Valuation | `valuation` | 3 | WACC (CAPM), DCF (FCFF), trading comps |
| 1 | Credit | `credit` | 4 | Metrics, debt capacity, covenants, Altman Z |
| 1 | Private Equity | `pe` | 4 | IRR/MOIC, debt schedules, sources & uses, LBO |
| 1 | Portfolio | `portfolio` | 3 | Risk-adjusted returns, VaR/CVaR, Kelly sizing |
| 1 | Scenarios | `scenarios` | 2 | Sensitivity matrix, scenario analysis |
| 2 | M&A | `ma` | 1 | Merger accretion/dilution |
| 2 | Jurisdiction | `jurisdiction` | 7 | GAAP/IFRS, WHT, NAV, GP economics, UBTI |
| 3 | Fixed Income | `fixed_income` | 5 | Bond pricing, yields, duration, spreads, curves |
| 3 | Derivatives | `derivatives` | 7 | Options, implied vol, forwards, swaps, strategies |
| 4 | Three Statement | `three_statement` | 1 | Integrated IS/BS/CF model |
| 4 | Monte Carlo | `monte_carlo` | 2 | Generic simulation, stochastic DCF |
| 4 | Quant Risk | `quant_risk` | 4 | Factor models, Black-Litterman, risk parity, stress testing |
| 5 | Restructuring | `restructuring` | 2 | Recovery waterfall, distressed debt |
| 5 | Real Assets | `real_assets` | 2 | Property valuation, project finance |
| 5 | FX & Commodities | `fx_commodities` | 4 | FX forwards, cross rates, commodity forwards/curves |
| 6 | Securitization | `securitization` | 2 | ABS/MBS cash flows, CDO/CLO tranching |
| 6 | Venture Capital | `venture` | 3 | Dilution, convertibles, fund returns |
| 6 | ESG | `esg` | 4 | Scoring, carbon, green bonds, SLL |
| 6 | Regulatory | `regulatory` | 3 | Basel III capital, LCR/NSFR, ALM |
| 7 | Private Credit | `private_credit` | 3 | Unitranche, direct lending, syndication |
| 7 | Insurance | `insurance` | 4 | Loss reserving, premium pricing, combined ratio, SCR |
| 7 | FP&A | `fpa` | 4 | Variance analysis, break-even, working capital, forecast |
| 7 | Wealth | `wealth` | 3 | Retirement planning, tax-loss harvesting, estate planning |
| 8 | Crypto | `crypto` | 2 | Token valuation (NVT/Metcalfe/DCF), DeFi yield/staking |
| 8 | Trade Finance | `trade_finance` | 2 | Letters of credit, supply chain finance/forfaiting |
| 8 | Structured Products | `structured_products` | 2 | Structured notes, exotic derivatives (autocallables, barriers) |
| 8 | Municipal | `municipal` | 2 | Muni bond pricing/TEY, GO/revenue bond analysis |

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
# All Rust tests (1500+ tests)
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
│   │       ├── crypto/           # Token valuation, DeFi yield
│   │       ├── trade_finance/    # LC, supply chain finance
│   │       ├── structured_products/ # Notes, exotic derivatives
│   │       ├── municipal/        # Muni bonds, GO/revenue analysis
│   │       └── ...               # 13 more domain modules
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
