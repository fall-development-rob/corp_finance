# Product Requirements Document (PRD)

## Corp Finance MCP Server & CLI

**Product:** corp-finance-mcp  
**Version:** 0.1.0  
**Author:** Robert Fall — Rob-otix AI Ltd  
**Date:** February 2026  
**Status:** Draft

---

## 1. Executive Summary

Corp Finance MCP is a high-precision financial analysis toolkit exposing corporate finance calculations as both an MCP (Model Context Protocol) server and a standalone CLI. The Rust core delivers institutional-grade numerical accuracy; TypeScript bindings provide ergonomic integration with LLM toolchains and Node.js ecosystems.

The product replaces the need for a CFA-qualified analyst for routine quantitative work — valuations, credit assessments, deal modelling, portfolio analytics — while maintaining the auditability and precision standards expected in regulated financial services.

---

## 2. Problem Statement

### 2.1 Market Gap

LLMs can reason about finance but cannot reliably compute. Spreadsheets can compute but cannot reason. There is no open, composable toolkit that bridges this gap with:

- Deterministic, auditable financial calculations
- LLM-native interfaces (MCP tools)
- Institutional-grade precision (decimal arithmetic, not floating point)
- CLI access for pipeline integration, scripting, and automation

### 2.2 Target Users

| Persona | Use Case | Interface |
|---------|----------|-----------|
| **LLM Agent** | Autonomous financial analysis within Claude, GPT, or custom agents | MCP Server |
| **Fintech Developer** | Embed financial calculations in applications | Rust crate / npm package |
| **Analyst / Consultant** | Quick valuations, credit checks, deal screening | CLI |
| **Fractional CTO** | Due diligence, portfolio company analysis, investor reporting | MCP + CLI |
| **Grant Platform** | Funder financial health assessment (e.g., Granted Giving) | MCP Server |

### 2.3 Why Now

- MCP adoption is accelerating — Claude, Cursor, Windsurf, and custom agents all support it
- No existing MCP server provides serious financial computation
- Rust + napi-rs stack is mature enough for production financial workloads
- The analyst skill (SKILL.md) provides the reasoning layer; this product provides the computation layer

---

## 3. Product Vision

**One-liner:** The Bloomberg Terminal of MCP — institutional-grade financial computation accessible to any LLM or CLI.

**North Star Metric:** Number of financial decisions made using corp-finance-mcp calculations per month.

---

## 4. Functional Requirements

### 4.1 Valuation Module

| ID | Requirement | Priority | Notes |
|----|------------|----------|-------|
| VAL-001 | DCF model (FCFF and FCFE variants) | P0 | Multi-stage with explicit forecast + terminal value |
| VAL-002 | WACC calculator | P0 | CAPM-based, supports levered/unlevered beta |
| VAL-003 | Trading comps analysis | P0 | EV/EBITDA, EV/Revenue, P/E, P/B with statistical summary |
| VAL-004 | Precedent transactions | P1 | Store and query transaction database |
| VAL-005 | Sum-of-parts (SOTP) | P1 | Multiple methodologies per segment |
| VAL-006 | Dividend discount model | P2 | H-model and multi-stage |
| VAL-007 | Residual income model | P2 | For financial institutions |
| VAL-008 | Football field chart data | P1 | Output ranges for visual comparison |

### 4.2 Credit Analysis Module

| ID | Requirement | Priority | Notes |
|----|------------|----------|-------|
| CRD-001 | Credit metrics calculator | P0 | Net Debt/EBITDA, interest coverage, FFO/Debt, DSCR |
| CRD-002 | Debt capacity sizing | P0 | From coverage ratio constraints |
| CRD-003 | Covenant compliance testing | P0 | Test actuals vs covenant thresholds |
| CRD-004 | Synthetic credit rating | P1 | Map metrics to rating categories (AAA → CCC) |
| CRD-005 | Recovery analysis | P2 | Waterfall recovery by tranche in distress scenario |
| CRD-006 | Altman Z-score | P1 | Bankruptcy prediction |

### 4.3 Private Equity / M&A Module

| ID | Requirement | Priority | Notes |
|----|------------|----------|-------|
| PE-001 | LBO model | P0 | Debt schedule, cash sweep, exit returns |
| PE-002 | Returns calculator (IRR, MOIC, CoC) | P0 | XIRR for irregular cash flows |
| PE-003 | Debt schedule builder | P0 | Multiple tranches, PIK, amortisation |
| PE-004 | Cash flow waterfall | P1 | GP/LP splits with hurdle rates and catch-up |
| PE-005 | Merger model (accretion/dilution) | P1 | Stock, cash, and mixed consideration |
| PE-006 | Synergy analysis | P2 | Revenue and cost synergies with phase-in |
| PE-007 | Sources & uses | P0 | Transaction financing summary |

### 4.4 Portfolio Analytics Module

| ID | Requirement | Priority | Notes |
|----|------------|----------|-------|
| PORT-001 | Portfolio risk metrics | P1 | VaR (parametric, historical), CVaR, max drawdown |
| PORT-002 | Performance attribution | P1 | Brinson model (allocation, selection, interaction) |
| PORT-003 | Risk-adjusted returns | P0 | Sharpe, Sortino, Information Ratio, Calmar |
| PORT-004 | Kelly criterion sizing | P1 | Full and fractional Kelly |
| PORT-005 | Correlation matrix | P1 | From return series |
| PORT-006 | Efficient frontier | P2 | Mean-variance optimisation |
| PORT-007 | Factor exposure analysis | P2 | Returns decomposition |

### 4.5 Fixed Income Module

| ID | Requirement | Priority | Notes |
|----|------------|----------|-------|
| FI-001 | Bond pricing (YTM, duration, convexity) | P1 | Modified and effective duration |
| FI-002 | Yield curve construction | P2 | Bootstrap from par yields |
| FI-003 | Spread analysis | P1 | OAS, Z-spread, G-spread |
| FI-004 | Duration hedging calculator | P1 | Futures-based hedge ratio |

### 4.6 Scenario & Sensitivity Module

| ID | Requirement | Priority | Notes |
|----|------------|----------|-------|
| SCN-001 | Sensitivity matrix (2-way) | P0 | Any two input variables vs any output |
| SCN-002 | Scenario analysis | P0 | Bear / base / bull with probability weighting |
| SCN-003 | Monte Carlo simulation | P1 | Configurable distributions, correlation, N runs |
| SCN-004 | Stress testing | P2 | Named scenarios (2008 GFC, COVID, rate shock) |

### 4.7 Three-Statement Model

| ID | Requirement | Priority | Notes |
|----|------------|----------|-------|
| TSM-001 | Income statement projection | P1 | Revenue build-up, margin assumptions, tax |
| TSM-002 | Balance sheet projection | P1 | Working capital, capex, debt schedule, equity |
| TSM-003 | Cash flow statement derivation | P1 | Indirect method from IS and BS changes |
| TSM-004 | Circular reference resolution | P2 | Interest ↔ debt ↔ cash flow iteration |

### 4.8 Jurisdiction & Fund Module

| ID | Requirement | Priority | Notes |
|----|------------|----------|-------|
| JUR-001 | GAAP/IFRS reconciliation tool | P1 | Input IFRS metrics, output US GAAP equivalents (and vice versa). Adjusts for leases, LIFO, dev costs, revaluation |
| JUR-002 | Withholding tax calculator | P1 | Given investor jurisdiction, fund jurisdiction, and asset jurisdiction — compute WHT drag on gross returns |
| JUR-003 | NAV calculator with equalisation | P1 | Series accounting, equalisation shares, depreciation deposit methods |
| JUR-004 | Multi-class share NAV tracker | P1 | Per-class NAV, high-water mark, performance fee accrual, currency hedging cost allocation |
| JUR-005 | Fund fee calculator | P0 | Management fee (committed vs invested vs NAV-based), performance fee with hurdle, catch-up, clawback. American and European waterfall variants |
| JUR-006 | GP economics model | P1 | Management fee income, carry projection, GP commitment return, break-even AUM |
| JUR-007 | Investor net returns calculator | P1 | Gross return → net of management fee, performance fee, fund expenses, WHT, blocker cost |
| JUR-008 | UBTI/ECI screening | P2 | Flag investment types that generate UBTI for US tax-exempt investors |
| JUR-009 | Treaty rate lookup | P2 | Given source country and investor country, return applicable WHT rates for dividends, interest, royalties |
| JUR-010 | Fund structure recommender | P2 | Given investor mix and strategy, suggest optimal structure (standalone, master-feeder, parallel, SPC) |
| JUR-011 | Economic substance checker | P2 | Given entity activities and jurisdiction, assess whether economic substance requirements are met |
| JUR-012 | Side pocket NAV segregation | P2 | Separate liquid and illiquid NAV tracking with gate modelling |

---

## 5. Non-Functional Requirements

### 5.1 Precision

| Requirement | Specification |
|-------------|--------------|
| Decimal arithmetic | `rust_decimal` crate — 128-bit fixed-point, 28 significant digits |
| No floating-point for money | All monetary calculations in `Decimal`, conversion only at serialisation boundary |
| Rounding | Configurable per output (banker's rounding default) |
| Currency | All outputs labelled with currency code |

### 5.2 Performance

| Requirement | Target |
|-------------|--------|
| Single DCF calculation | < 5ms |
| Full LBO model (10yr, 3 tranches) | < 20ms |
| Monte Carlo (10,000 runs) | < 500ms |
| XIRR convergence | < 2ms, 100 iterations max |
| CLI cold start | < 200ms |

### 5.3 Auditability

- Every output includes the input parameters used
- Intermediate calculations exposed (not just final answer)
- Sensitivity tables show assumption ranges
- JSON output includes `methodology`, `assumptions`, `warnings` fields
- Version string embedded in every output for reproducibility

### 5.4 Error Handling

- Invalid inputs return structured errors with field-level detail
- Financial impossibilities flagged (e.g., negative WACC, terminal growth > WACC)
- Convergence failures reported with iteration count and last delta
- Warnings for suspicious inputs (e.g., beta > 3, growth > 20%)

### 5.5 Security

- No network access from Rust core (pure computation)
- No persistent state — stateless function calls
- Input validation at TypeScript boundary (Zod) AND Rust boundary
- No secrets or API keys required

---

## 6. Interface Specifications

### 6.1 MCP Server

**Transport:** stdio (standard for MCP)  
**Protocol:** MCP v1.0  
**SDK:** @modelcontextprotocol/sdk  

Each tool maps 1:1 to a Rust function. Tools are grouped by module:

```
wacc_calculator          → valuation module
dcf_model               → valuation module
comps_analysis           → valuation module
precedent_transactions   → valuation module
lbo_model               → PE module
returns_calculator       → PE module
debt_schedule            → PE module
waterfall_calculator     → PE module
merger_model             → M&A module
credit_metrics           → credit module
debt_capacity            → credit module
covenant_compliance      → credit module
sensitivity_matrix       → scenarios module
monte_carlo              → scenarios module
risk_metrics             → portfolio module
performance_attribution  → portfolio module
bond_pricing             → fixed income module
gaap_ifrs_reconcile      → jurisdiction module
withholding_tax          → jurisdiction module
nav_calculator           → jurisdiction module
fund_fee_calculator      → jurisdiction module
gp_economics             → jurisdiction module
investor_net_returns     → jurisdiction module
```

**Tool Response Format:**
```json
{
  "result": { /* computed output */ },
  "methodology": "DCF (FCFF, 2-stage)",
  "assumptions": { /* all inputs echoed back */ },
  "warnings": ["Terminal growth (3.5%) is above long-term GDP estimates"],
  "metadata": {
    "version": "0.1.0",
    "computation_time_ms": 4.2,
    "precision": "rust_decimal_128bit"
  }
}
```

### 6.2 CLI

**Binary:** `cfa` (or `corp-finance`)  
**Runtime:** Compiled Rust (no Node.js dependency for CLI)  
**Shell:** Supports pipe-in, pipe-out, JSON and table output formats

```bash
# Direct computation
cfa wacc --risk-free-rate 0.04 --erp 0.055 --beta 1.2 --cost-of-debt 0.06 --tax-rate 0.25 --debt-weight 0.3

# JSON input from file
cfa dcf --input deal.json

# Pipe from stdin
cat financials.json | cfa credit-metrics --output table

# Sensitivity analysis
cfa sensitivity --model dcf --var1 wacc:0.08:0.12:0.005 --var2 terminal-growth:0.01:0.03:0.005

# Chain commands
cfa wacc --output json | cfa dcf --wacc-from-stdin --input projections.json

# Batch mode
cfa batch --config portfolio.yaml --output results/
```

**Output Formats:**
- `--output json` (default) — full structured output
- `--output table` — human-readable ASCII table
- `--output csv` — for spreadsheet import
- `--output minimal` — just the answer (for scripting)

### 6.3 Rust Crate (Library)

Published as `corp-finance-core` on crates.io. All functions are pure, stateless, and return `Result<T, CorpFinanceError>`.

```rust
use corp_finance_core::valuation::{dcf, wacc};
use corp_finance_core::credit::metrics;
use rust_decimal::Decimal;

let cost_of_capital = wacc::calculate(&wacc::WaccInput { ... })?;
let valuation = dcf::build(&dcf::DcfInput { wacc: cost_of_capital.wacc, ... })?;
```

### 6.4 npm Package (Bindings)

Published as `corp-finance-bindings` on npm. Prebuilt binaries for linux-x64, darwin-x64, darwin-arm64, win32-x64.

```typescript
import { calculateWacc, buildDcf, creditMetrics } from 'corp-finance-bindings';
```

---

## 7. Phased Delivery

### Phase 1 — Foundation (4 weeks)

**Rust Core:**
- WACC calculator
- Credit metrics (all coverage and leverage ratios)
- DCF model (FCFF, 2-stage)
- Debt capacity sizing
- Covenant compliance
- Sensitivity matrix (2-way)
- Returns calculator (IRR, XIRR, MOIC)
- Risk-adjusted returns (Sharpe, Sortino, Calmar)

**Bindings:**
- napi-rs bindings for all Phase 1 functions
- npm package build pipeline (cross-platform)

**MCP Server:**
- stdio transport
- All Phase 1 tools registered with Zod schemas
- Structured JSON responses

**CLI:**
- All Phase 1 tools as subcommands
- JSON, table, and minimal output formats
- Stdin pipe support

**Tests:**
- Unit tests for every Rust function (known-answer tests from textbook examples)
- Integration tests for napi bindings
- MCP protocol compliance tests

### Phase 2 — Deal Modelling (3 weeks)

- LBO model with multi-tranche debt
- Debt schedule builder
- Sources & uses
- Merger model (accretion/dilution)
- Trading comps with statistical summary
- Precedent transactions
- Cash flow waterfall
- Scenario analysis (bear/base/bull)
- Altman Z-score
- Fund fee calculator (American + European waterfall, management fee variants)

### Phase 3 — Jurisdiction & Fund Analytics (3 weeks)

- GAAP/IFRS reconciliation tool (lease capitalisation, LIFO adjustment, dev cost normalisation)
- Withholding tax calculator (treaty rate lookup, net-of-WHT return computation)
- NAV calculator with equalisation (series accounting, equalisation shares, depreciation deposit)
- Multi-class share NAV tracker (per-class HWM, performance fee accrual)
- GP economics model (fee income, carry projection, break-even AUM)
- Investor net returns calculator (gross → net of all fees, expenses, WHT, blocker cost)
- UBTI/ECI screening tool

### Phase 4 — Portfolio & Fixed Income (3 weeks)

- Three-statement model engine
- Bond pricing, duration, convexity
- VaR and CVaR
- Performance attribution (Brinson)
- Kelly criterion position sizing
- Correlation matrix
- Monte Carlo simulation
- Spread analysis

### Phase 5 — Advanced (Ongoing)

- Efficient frontier optimisation
- Factor exposure analysis
- Yield curve construction
- Stress testing (named scenarios)
- Circular reference resolution (iterative solver)
- SOTP valuation
- Dividend discount model
- Recovery analysis
- Fund structure recommender
- Economic substance checker
- Side pocket NAV segregation with gate modelling
- Treaty rate database

---

## 8. Integration Points

### 8.1 Granted Giving

The MCP server enables automated funder financial health assessment:

- Pull Companies House data → feed into credit metrics tool
- Assess funder solvency before recommending to charities
- Automate trust/foundation financial screening at scale

### 8.2 Financial Analyst Skill

The SKILL.md analyst skill provides reasoning and methodology selection. This MCP provides the computation. Together they form a complete analyst:

```
User question → Analyst Skill (reasoning, methodology selection)
                    ↓
              MCP Server (computation)
                    ↓
              Analyst Skill (interpretation, memo writing)
```

### 8.3 Trading Infrastructure

Feeds into existing QSConnect/QSResearch/Omega stack:

- Portfolio risk metrics for position monitoring
- Kelly sizing for trade allocation
- Scenario analysis for strategy stress testing

---

## 9. Success Criteria

| Metric | Target | Timeframe |
|--------|--------|-----------|
| Phase 1 shipped | All P0 requirements passing tests | 4 weeks |
| Calculation accuracy | 100% match to textbook reference answers | Ongoing |
| MCP tool response time | < 50ms p99 for single calculations | Phase 1 |
| CLI cold start | < 200ms | Phase 1 |
| npm package size | < 15MB (prebuilt binary) | Phase 1 |
| Crate documentation | 100% public API documented | Each phase |
| Cross-platform builds | Linux, macOS (x64 + ARM), Windows | Phase 1 |

---

## 10. Out of Scope

- Real-time market data feeds (use external data providers)
- GUI / web interface (CLI and MCP only)
- Data storage / database (stateless computation)
- Backtesting framework (separate product)
- Regulatory reporting generation
- Natural language interpretation (that's the LLM's job)

---

## 11. Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| napi-rs cross-platform build issues | Delays npm publishing | Medium | CI matrix for all targets, prebuilt binaries |
| Numerical edge cases (e.g., XIRR non-convergence) | Incorrect results | Medium | Extensive test suite, known-answer validation |
| MCP protocol changes | Server incompatibility | Low | Pin SDK version, monitor changelog |
| Scope creep into data/storage layer | Architecture bloat | Medium | Strict stateless constraint, no database |
| rust_decimal performance for Monte Carlo | Slow simulation | Low | Benchmark early, fallback to f64 for MC only |

---

## 12. Competitive Landscape

| Product | Strengths | Weaknesses vs This |
|---------|-----------|-------------------|
| Bloomberg Terminal | Complete, real-time data | $24k/yr, no LLM integration, no API composability |
| Aswath Damodaran Spreadsheets | Free, well-documented | Excel-only, not programmable, not composable |
| QuantLib | Comprehensive quant library | C++, steep learning curve, no MCP |
| OpenBB | Good data aggregation | Python, no MCP, limited computation |
| Financial Modeling Prep API | Good data | Data only, no computation |

**Unique positioning:** Only product offering institutional-grade financial computation as both MCP tools and a standalone CLI with Rust precision.
