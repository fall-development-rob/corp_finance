# Architecture Requirements Document (ARD)

## Corp Finance MCP Server & CLI

**Product:** corp-finance-mcp  
**Version:** 0.1.0  
**Author:** Robert Fall — Rob-otix AI Ltd  
**Date:** February 2026  
**Status:** Draft

---

## 1. Architecture Overview

### 1.1 System Context

```
┌──────────────────────────────────────────────────────────────┐
│                        Consumers                              │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌────────────┐ │
│  │  Claude   │  │  Custom  │  │   Shell   │  │  Rust/Node │ │
│  │  Desktop  │  │  Agent   │  │  Scripts  │  │   Apps     │ │
│  └────┬─────┘  └────┬─────┘  └─────┬─────┘  └─────┬──────┘ │
│       │              │              │               │        │
│       └──────┬───────┘              │               │        │
│              │                      │               │        │
│         MCP (stdio)            CLI (binary)    Library API   │
└──────────┬──────────────────────┬───────────────────┬────────┘
           │                      │                   │
┌──────────▼──────────────────────▼───────────────────▼────────┐
│                    corp-finance-mcp                            │
│                                                              │
│  ┌─────────────────────┐  ┌────────────────────────────────┐ │
│  │   MCP Server (TS)   │  │         CLI (Rust)             │ │
│  │                     │  │                                │ │
│  │  Tool registration  │  │  Subcommand parsing (clap)     │ │
│  │  Zod validation     │  │  Input validation              │ │
│  │  JSON formatting    │  │  Output formatting             │ │
│  └─────────┬───────────┘  └──────────────┬─────────────────┘ │
│            │                              │                   │
│  ┌─────────▼──────────────────────────────▼─────────────────┐ │
│  │              napi-rs Bindings (bridge)                    │ │
│  │                                                          │ │
│  │  Rust ←→ Node.js type marshalling                        │ │
│  │  JSON serialisation at boundary                          │ │
│  └──────────────────────┬───────────────────────────────────┘ │
│                         │                                     │
│  ┌──────────────────────▼───────────────────────────────────┐ │
│  │              corp-finance-core (Rust)                     │ │
│  │                                                          │ │
│  │  ┌──────────┐ ┌────────┐ ┌──────┐ ┌──────────┐         │ │
│  │  │Valuation │ │ Credit │ │  PE  │ │Portfolio │         │ │
│  │  │          │ │        │ │ / MA │ │          │         │ │
│  │  └──────────┘ └────────┘ └──────┘ └──────────┘         │ │
│  │  ┌──────────┐ ┌────────┐ ┌──────────────────┐          │ │
│  │  │  Fixed   │ │Scenario│ │  Three-Statement │          │ │
│  │  │  Income  │ │        │ │      Model       │          │ │
│  │  └──────────┘ └────────┘ └──────────────────┘          │ │
│  │                                                          │ │
│  │  rust_decimal │ serde │ thiserror                        │ │
│  └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### 1.2 Core Architectural Principles

| Principle | Description | Rationale |
|-----------|-------------|-----------|
| **Rust owns computation** | All financial math in Rust. Zero financial calculations in TypeScript. | Precision, performance, correctness guarantees |
| **TypeScript owns interface** | MCP tool registration, protocol handling, input validation | MCP SDK is TS-native, Zod schemas |
| **Stateless functions** | Every function is pure — same inputs always produce same outputs | Auditability, testability, no side effects |
| **JSON at the boundary** | Rust returns serialised JSON, TS parses it | Clean separation, debuggable, no complex FFI types |
| **Decimal precision** | `rust_decimal` for all monetary values, never `f64` | Financial accuracy — floating point errors are unacceptable |
| **Fail loud** | Invalid inputs and financial impossibilities return errors, not silent defaults | Analysts need to know when something's wrong |
| **Composable internals** | DCF can call WACC internally; LBO calls debt_schedule | Functions build on each other without duplication |

---

## 2. Repository Structure

```
corp-finance-mcp/
├── Cargo.toml                          # Workspace root
├── pnpm-workspace.yaml
├── package.json                        # Workspace scripts
├── .cargo/
│   └── config.toml                     # Build config, cross-compilation
│
├── crates/
│   └── corp-finance-core/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs                  # Public API surface
│       │   ├── error.rs                # Error types
│       │   ├── types.rs                # Shared types (Decimal wrappers, Currency, etc.)
│       │   ├── time_value.rs           # IRR, XIRR, NPV, PV, FV
│       │   ├── valuation/
│       │   │   ├── mod.rs
│       │   │   ├── wacc.rs
│       │   │   ├── dcf.rs
│       │   │   ├── comps.rs
│       │   │   ├── sotp.rs
│       │   │   └── ddm.rs
│       │   ├── credit/
│       │   │   ├── mod.rs
│       │   │   ├── metrics.rs
│       │   │   ├── capacity.rs
│       │   │   ├── covenants.rs
│       │   │   ├── rating.rs
│       │   │   └── altman.rs
│       │   ├── pe/
│       │   │   ├── mod.rs
│       │   │   ├── lbo.rs
│       │   │   ├── debt_schedule.rs
│       │   │   ├── returns.rs
│       │   │   ├── waterfall.rs
│       │   │   └── sources_uses.rs
│       │   ├── ma/
│       │   │   ├── mod.rs
│       │   │   ├── merger_model.rs
│       │   │   └── synergies.rs
│       │   ├── portfolio/
│       │   │   ├── mod.rs
│       │   │   ├── risk.rs             # VaR, CVaR, max drawdown
│       │   │   ├── attribution.rs      # Brinson model
│       │   │   ├── returns.rs          # Sharpe, Sortino, IR, Calmar
│       │   │   ├── sizing.rs           # Kelly criterion
│       │   │   └── correlation.rs
│       │   ├── fixed_income/
│       │   │   ├── mod.rs
│       │   │   ├── bond.rs             # Pricing, YTM, duration, convexity
│       │   │   ├── yield_curve.rs
│       │   │   └── spread.rs
│       │   ├── three_statement/
│       │   │   ├── mod.rs
│       │   │   ├── income.rs
│       │   │   ├── balance_sheet.rs
│       │   │   ├── cash_flow.rs
│       │   │   └── solver.rs           # Circular reference iteration
│       │   ├── jurisdiction/
│       │   │   ├── mod.rs
│       │   │   ├── reconciliation.rs   # GAAP/IFRS reconciliation
│       │   │   ├── withholding_tax.rs  # WHT calculator + treaty rates
│       │   │   ├── nav.rs              # NAV with equalisation
│       │   │   ├── fund_fees.rs        # Fund fee calculator
│       │   │   ├── gp_economics.rs     # GP economics model
│       │   │   ├── investor_returns.rs # Investor net returns
│       │   │   └── treaties.rs         # Treaty rate database
│       │   └── scenarios/
│       │       ├── mod.rs
│       │       ├── sensitivity.rs
│       │       ├── scenario.rs
│       │       └── monte_carlo.rs
│       └── tests/
│           ├── valuation_tests.rs
│           ├── credit_tests.rs
│           ├── pe_tests.rs
│           ├── portfolio_tests.rs
│           └── fixtures/               # Known-answer test data
│               ├── damodaran_wacc.json
│               ├── lbo_reference.json
│               └── bond_pricing.json
│
├── crates/
│   └── corp-finance-cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                 # Entry point
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── valuation.rs        # wacc, dcf, comps subcommands
│           │   ├── credit.rs           # credit-metrics, debt-capacity
│           │   ├── pe.rs               # lbo, returns, waterfall
│           │   ├── portfolio.rs        # risk, attribution, sizing
│           │   ├── fixed_income.rs     # bond, spread
│           │   ├── scenarios.rs        # sensitivity, monte-carlo
│           │   ├── jurisdiction.rs    # gaap-ifrs, wht, nav, fund-fees
│           │   └── batch.rs            # Batch mode from YAML config
│           ├── output/
│           │   ├── mod.rs
│           │   ├── json.rs
│           │   ├── table.rs            # ASCII table formatting
│           │   ├── csv.rs
│           │   └── minimal.rs
│           └── input/
│               ├── mod.rs
│               ├── stdin.rs            # Pipe support
│               └── file.rs             # JSON/YAML file input
│
├── packages/
│   ├── bindings/
│   │   ├── Cargo.toml                  # napi-rs crate
│   │   ├── src/
│   │   │   ├── lib.rs                  # All napi exports
│   │   │   ├── valuation.rs
│   │   │   ├── credit.rs
│   │   │   ├── pe.rs
│   │   │   ├── portfolio.rs
│   │   │   ├── fixed_income.rs
│   │   │   ├── scenarios.rs
│   │   │   └── jurisdiction.rs
│   │   ├── index.d.ts                  # Generated TypeScript types
│   │   ├── package.json
│   │   └── npm/                        # Platform-specific packages
│   │       ├── linux-x64-gnu/
│   │       ├── darwin-x64/
│   │       ├── darwin-arm64/
│   │       └── win32-x64-msvc/
│   │
│   └── mcp-server/
│       ├── src/
│       │   ├── index.ts                # Server entry, tool registration
│       │   ├── tools/
│       │   │   ├── valuation.ts
│       │   │   ├── credit.ts
│       │   │   ├── pe.ts
│       │   │   ├── ma.ts
│       │   │   ├── portfolio.ts
│       │   │   ├── fixed_income.ts
│       │   │   ├── scenarios.ts
│       │   │   └── jurisdiction.ts
│       │   ├── schemas/
│       │   │   ├── index.ts            # Re-exports
│       │   │   ├── valuation.ts        # Zod schemas for valuation tools
│       │   │   ├── credit.ts
│       │   │   ├── pe.ts
│       │   │   ├── jurisdiction.ts     # GAAP/IFRS, WHT, NAV, fund fees
│       │   │   └── common.ts           # Shared types (Currency, Jurisdiction, etc.)
│       │   └── formatters/
│       │       └── response.ts         # Standardised response envelope
│       ├── package.json
│       └── tsconfig.json
│
├── docs/
│   ├── PRD.md
│   ├── ARD.md
│   ├── DDD.md
│   └── api/                            # Generated Rust docs
│
├── .github/
│   └── workflows/
│       ├── ci.yml                      # Test on every PR
│       ├── release.yml                 # Build + publish
│       └── cross-compile.yml           # Multi-platform binaries
│
└── README.md
```

---

## 3. Component Architecture

### 3.1 Rust Core (`corp-finance-core`)

**Responsibility:** All financial computation. Pure functions, no I/O, no network, no state.

**Dependencies:**

| Crate | Version | Purpose |
|-------|---------|---------|
| `rust_decimal` | 1.x | 128-bit decimal arithmetic |
| `rust_decimal_macros` | 1.x | `dec!()` macro for literal decimals |
| `serde` | 1.x | Serialise/deserialise inputs and outputs |
| `serde_json` | 1.x | JSON serialisation |
| `thiserror` | 2.x | Ergonomic error types |
| `chrono` | 0.4.x | Date handling (XIRR, bond pricing) |
| `rand` | 0.8.x | Monte Carlo simulation (feature-gated) |
| `statrs` | 0.17.x | Statistical distributions for Monte Carlo |

**Feature Gates:**

```toml
[features]
default = ["valuation", "credit"]
valuation = []
credit = []
pe = []
ma = []
portfolio = []
fixed_income = []
three_statement = []
jurisdiction = []
scenarios = ["rand", "statrs"]
monte_carlo = ["scenarios"]
full = ["valuation", "credit", "pe", "ma", "portfolio", "fixed_income", "three_statement", "jurisdiction", "scenarios", "monte_carlo"]
```

**Type System:**

```rust
// Core decimal wrapper — prevents accidental f64 usage
pub type Money = rust_decimal::Decimal;
pub type Rate = rust_decimal::Decimal;    // 0.05 = 5%
pub type Multiple = rust_decimal::Decimal; // 8.5x
pub type Years = rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Currency {
    GBP, USD, EUR, CHF, JPY, CAD, AUD, Other(String),
}

// Standard result type
pub type CorpFinanceResult<T> = Result<T, CorpFinanceError>;

// Every output includes metadata
#[derive(Debug, Serialize)]
pub struct ComputationOutput<T: Serialize> {
    pub result: T,
    pub methodology: String,
    pub assumptions: serde_json::Value,
    pub warnings: Vec<String>,
    pub metadata: ComputationMetadata,
}

#[derive(Debug, Serialize)]
pub struct ComputationMetadata {
    pub version: &'static str,
    pub computation_time_us: u64,
    pub precision: &'static str, // "rust_decimal_128bit"
}
```

**Error Hierarchy:**

```rust
#[derive(Debug, thiserror::Error)]
pub enum CorpFinanceError {
    #[error("Invalid input: {field} — {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Financial impossibility: {0}")]
    FinancialImpossibility(String),
    // e.g., terminal growth >= WACC, negative enterprise value

    #[error("Convergence failure: {function} did not converge after {iterations} iterations (delta: {last_delta})")]
    ConvergenceFailure {
        function: String,
        iterations: u32,
        last_delta: Decimal,
    },

    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    #[error("Division by zero in {context}")]
    DivisionByZero { context: String },

    #[error("Date error: {0}")]
    DateError(String),
}
```

### 3.2 CLI (`corp-finance-cli`)

**Responsibility:** Command-line interface for direct human use and shell scripting.

**Dependencies:**

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4.x | Argument parsing with derive macros |
| `tabled` | 0.16.x | ASCII table output |
| `serde_json` | 1.x | JSON input/output |
| `serde_yaml` | 0.9.x | YAML config for batch mode |
| `csv` | 1.x | CSV output |
| `colored` | 2.x | Terminal colours for warnings/errors |
| `corp-finance-core` | workspace | Calculation engine |

**Command Structure:**

```
cfa
├── wacc            # WACC calculation
├── dcf             # DCF valuation
├── comps           # Trading comparables
├── credit-metrics  # Credit ratios
├── debt-capacity   # Maximum debt sizing
├── covenant-test   # Covenant compliance
├── lbo             # LBO model
├── returns         # IRR, XIRR, MOIC
├── debt-schedule   # Build debt amortisation
├── waterfall       # GP/LP distribution
├── merger          # Accretion/dilution
├── bond            # Bond pricing/duration
├── sensitivity     # 2-way sensitivity table
├── monte-carlo     # Monte Carlo simulation
├── risk            # VaR, CVaR, drawdown
├── sharpe          # Risk-adjusted returns
├── kelly           # Position sizing
├── gaap-ifrs       # GAAP/IFRS reconciliation
├── wht             # Withholding tax calculator
├── nav             # NAV calculation with equalisation
├── fund-fees       # Fund fee modelling
├── gp-economics    # GP economics model
├── investor-net    # Investor net returns
├── batch           # Run multiple from YAML
└── version         # Version info
```

**I/O Flow:**

```
Input Sources              CLI              Output Targets
┌─────────┐           ┌──────────┐         ┌──────────┐
│  Flags  │──────────▶│  clap    │────────▶│  stdout  │
│  --arg  │           │  parse   │         │  (JSON)  │
└─────────┘           └────┬─────┘         └──────────┘
┌─────────┐                │               ┌──────────┐
│  stdin  │──────────▶│  validate│────────▶│  stdout  │
│  (pipe) │           │  & merge │         │  (table) │
└─────────┘           └────┬─────┘         └──────────┘
┌─────────┐                │               ┌──────────┐
│  file   │──────────▶│  core    │────────▶│  file    │
│  .json  │           │  compute │         │  (.csv)  │
└─────────┘           └──────────┘         └──────────┘
```

**Pipe Chaining Design:**

```bash
# WACC output feeds into DCF as --wacc parameter
cfa wacc --risk-free 0.04 --erp 0.055 --beta 1.2 \
         --cod 0.06 --tax 0.25 --debt-w 0.3 \
         --output minimal | \
cfa dcf --wacc-stdin --revenue 100 --growth 0.08 --margin 0.15 --years 10

# Credit metrics from JSON file, output as table
cat company.json | cfa credit-metrics --output table

# Batch sensitivity across portfolio
cfa batch --config portfolio.yaml --output results/ --format csv
```

### 3.3 napi-rs Bindings (`packages/bindings`)

**Responsibility:** Bridge between Rust core and Node.js. Type marshalling, JSON serialisation boundary.

**Architecture Decision: JSON String Boundary**

Rather than complex napi type mappings, all functions accept and return JSON strings:

```rust
// packages/bindings/src/lib.rs
use napi_derive::napi;
use corp_finance_core::valuation::wacc;

#[napi]
pub fn calculate_wacc(input_json: String) -> napi::Result<String> {
    let input: wacc::WaccInput = serde_json::from_str(&input_json)
        .map_err(|e| napi::Error::from_reason(format!("Invalid input: {e}")))?;
    
    let output = wacc::calculate(&input)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    
    serde_json::to_string(&output)
        .map_err(|e| napi::Error::from_reason(format!("Serialisation error: {e}")))
}
```

**Rationale:**
- Avoids complex napi type definitions for nested financial structures
- JSON is debuggable — you can log inputs/outputs
- Zod validates on the TS side before calling Rust
- Rust validates again (belt and braces for financial software)
- Minimal binding surface area — each function is just `String → String`

**Build Configuration:**

```toml
# packages/bindings/Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
corp-finance-core = { path = "../../crates/corp-finance-core", features = ["full"] }
napi = { version = "2", features = ["napi4", "serde-json"] }
napi-derive = "2"
serde_json = "1.0"

[build-dependencies]
napi-build = "2"
```

**Cross-Platform Build Matrix:**

| Target | OS | Arch | Package |
|--------|-----|------|---------|
| `x86_64-unknown-linux-gnu` | Linux | x64 | `@corp-finance/linux-x64-gnu` |
| `x86_64-apple-darwin` | macOS | x64 | `@corp-finance/darwin-x64` |
| `aarch64-apple-darwin` | macOS | ARM64 | `@corp-finance/darwin-arm64` |
| `x86_64-pc-windows-msvc` | Windows | x64 | `@corp-finance/win32-x64-msvc` |

### 3.4 MCP Server (`packages/mcp-server`)

**Responsibility:** MCP protocol handling, tool registration, input validation, response formatting.

**Zero financial computation in this layer.**

```typescript
// packages/mcp-server/src/index.ts
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { registerValuationTools } from "./tools/valuation.js";
import { registerCreditTools } from "./tools/credit.js";
import { registerPETools } from "./tools/pe.js";
import { registerPortfolioTools } from "./tools/portfolio.js";
import { registerFixedIncomeTools } from "./tools/fixed_income.js";
import { registerScenarioTools } from "./tools/scenarios.js";
import { registerJurisdictionTools } from "./tools/jurisdiction.js";

const server = new McpServer({
  name: "corp-finance-mcp",
  version: "0.1.0",
  description: "Institutional-grade corporate finance calculations",
});

registerValuationTools(server);
registerCreditTools(server);
registerPETools(server);
registerPortfolioTools(server);
registerFixedIncomeTools(server);
registerScenarioTools(server);
registerJurisdictionTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
```

**Tool Registration Pattern:**

```typescript
// packages/mcp-server/src/tools/valuation.ts
import { z } from "zod";
import { calculateWacc, buildDcf } from "corp-finance-bindings";
import { wrapResponse } from "../formatters/response.js";

const WaccSchema = z.object({
  risk_free_rate: z.number().min(0).max(0.20).describe("10Y government bond yield"),
  equity_risk_premium: z.number().min(0).max(0.15).describe("Market risk premium"),
  beta: z.number().min(0).max(5).describe("Levered equity beta"),
  cost_of_debt: z.number().min(0).max(0.30).describe("Pre-tax cost of debt"),
  tax_rate: z.number().min(0).max(0.50).describe("Corporate tax rate"),
  debt_weight: z.number().min(0).max(1).describe("Debt / (Debt + Equity)"),
  equity_weight: z.number().min(0).max(1).describe("Equity / (Debt + Equity)"),
  size_premium: z.number().optional().describe("Small-cap premium if applicable"),
  country_risk_premium: z.number().optional().describe("Emerging market premium"),
});

export function registerValuationTools(server: McpServer) {
  server.tool(
    "wacc_calculator",
    "Calculate weighted average cost of capital using CAPM",
    WaccSchema.shape,
    async (params) => {
      const validated = WaccSchema.parse(params);
      const resultJson = calculateWacc(JSON.stringify(validated));
      return wrapResponse(resultJson);
    }
  );
}
```

**Response Envelope:**

```typescript
// packages/mcp-server/src/formatters/response.ts
export function wrapResponse(resultJson: string) {
  const parsed = JSON.parse(resultJson);
  
  // Build MCP text content with structured output
  const content = [
    { type: "text" as const, text: resultJson }
  ];
  
  return { content };
}
```

---

## 4. Data Flow

### 4.1 MCP Request Flow

```
1. LLM sends MCP tool call
   │
2. MCP SDK routes to registered tool handler
   │
3. Zod schema validates input (TS layer)
   │  ├── Invalid → Return MCP error with field-level detail
   │  └── Valid ↓
   │
4. Serialise validated input to JSON string
   │
5. Call napi binding function (crosses FFI boundary)
   │
6. Rust deserialises JSON to typed input struct
   │
7. Rust validates financial constraints
   │  ├── Invalid → Return CorpFinanceError
   │  └── Valid ↓
   │
8. Rust performs computation (all in rust_decimal)
   │
9. Rust serialises output to JSON string
   │
10. JSON crosses FFI boundary back to TypeScript
    │
11. MCP server formats response envelope
    │
12. Response sent to LLM via stdio
```

### 4.2 CLI Request Flow

```
1. User invokes CLI command
   │
2. clap parses arguments + reads stdin/file if present
   │
3. Merge all input sources into typed struct
   │
4. Validate input constraints
   │  ├── Invalid → Print error to stderr, exit 1
   │  └── Valid ↓
   │
5. Call corp-finance-core function directly (no FFI)
   │
6. Format output based on --output flag
   │  ├── json → serde_json pretty print
   │  ├── table → tabled ASCII table
   │  ├── csv → csv writer
   │  └── minimal → just the answer
   │
7. Write to stdout (or file if --output-file specified)
```

### 4.3 Composition Flow (Internal)

Functions compose internally without re-serialisation:

```rust
// LBO model internally calls debt_schedule, returns, and waterfall
pub fn build_lbo(input: &LboInput) -> CorpFinanceResult<LboOutput> {
    // 1. Build debt schedule for each tranche
    let schedules: Vec<DebtScheduleOutput> = input.tranches.iter()
        .map(|t| debt_schedule::build(t))
        .collect::<Result<_, _>>()?;
    
    // 2. Project cash flows with debt service
    let projections = project_cash_flows(input, &schedules)?;
    
    // 3. Calculate exit value
    let exit = calculate_exit(input, &projections)?;
    
    // 4. Calculate returns
    let returns = returns::calculate(&ReturnsInput {
        cash_flows: build_cash_flow_series(input, &projections, &exit),
        dates: build_date_series(input),
    })?;
    
    // 5. Build waterfall if GP/LP structure specified
    let waterfall = if let Some(wf_input) = &input.waterfall {
        Some(waterfall::calculate(wf_input, &exit)?)
    } else {
        None
    };
    
    Ok(LboOutput { schedules, projections, exit, returns, waterfall })
}
```

---

## 5. Cross-Cutting Concerns

### 5.1 Validation Strategy (Belt and Braces)

```
Layer 1: Zod (TypeScript)         Layer 2: Rust
─────────────────────────         ─────────────────────
Type checking                     Type checking (serde)
Range validation                  Financial constraint validation
Required fields                   Cross-field consistency
Schema documentation              Domain-specific warnings
```

**Why both?**
- Zod catches malformed requests before FFI (cheaper)
- Rust catches financial impossibilities that require domain knowledge
- CLI bypasses Zod entirely — Rust validation is the single source of truth

### 5.2 Numerical Precision Architecture

```rust
// RULE: All internal computation uses rust_decimal::Decimal
// RULE: f64 ONLY permitted in Monte Carlo simulation (feature-gated, clearly marked)
// RULE: Conversion to f64 for display only, never for further computation

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// Constants as Decimal
const DEFAULT_TERMINAL_GROWTH: Decimal = dec!(0.025);
const MAX_WACC: Decimal = dec!(0.30);
const CONVERGENCE_THRESHOLD: Decimal = dec!(0.0000001);

// XIRR uses Newton-Raphson in Decimal space
fn xirr_newton_raphson(
    cash_flows: &[(NaiveDate, Decimal)],
    guess: Decimal,
    max_iterations: u32,
) -> CorpFinanceResult<Decimal> {
    let mut rate = guess;
    for i in 0..max_iterations {
        let (npv, dnpv) = npv_and_derivative(cash_flows, rate)?;
        if npv.abs() < CONVERGENCE_THRESHOLD {
            return Ok(rate);
        }
        rate = rate - npv / dnpv;
    }
    Err(CorpFinanceError::ConvergenceFailure { ... })
}
```

### 5.3 Testing Architecture

```
Test Type           Location                    Runner
─────────────────   ────────────────────────    ──────────
Unit (Rust)         crates/*/tests/             cargo test
Known-answer        crates/*/tests/fixtures/    cargo test (from JSON fixtures)
Integration (napi)  packages/bindings/tests/    vitest
MCP protocol        packages/mcp-server/tests/  vitest
CLI end-to-end      tests/e2e/                  bash + cargo test
Cross-validation    tests/cross-val/            Compare Rust output to Excel/Bloomberg reference
```

**Known-Answer Test Pattern:**

```rust
#[test]
fn test_wacc_damodaran_reference() {
    // Reference: Damodaran's WACC calculation for Apple (Jan 2024)
    let input = WaccInput {
        risk_free_rate: dec!(0.0425),
        equity_risk_premium: dec!(0.0472),
        beta: dec!(1.24),
        cost_of_debt: dec!(0.034),
        tax_rate: dec!(0.1623),
        debt_weight: dec!(0.0637),
        equity_weight: dec!(0.9363),
        ..Default::default()
    };
    let output = wacc::calculate(&input).unwrap();
    // Reference answer: ~10.03%
    assert_decimal_approx(output.result.wacc, dec!(0.1003), dec!(0.001));
}
```

### 5.4 CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
jobs:
  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - cargo fmt --check
      - cargo clippy -- -D warnings
      - cargo test --workspace --all-features
  
  bindings-build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, macos-14, windows-latest]
    steps:
      - napi build --release
      - vitest run

  mcp-tests:
    needs: bindings-build
    steps:
      - vitest run

  cli-tests:
    steps:
      - cargo build --release -p corp-finance-cli
      - bash tests/e2e/run_all.sh
```

```yaml
# .github/workflows/release.yml
on:
  push:
    tags: ['v*']
jobs:
  publish-crate:
    - cargo publish -p corp-finance-core
  
  publish-npm:
    - napi prepublish
    - npm publish (platform packages)
    - npm publish (main package)
  
  publish-cli:
    # Build static binaries for each platform
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-musl
          - x86_64-apple-darwin
          - aarch64-apple-darwin
          - x86_64-pc-windows-msvc
    steps:
      - cargo build --release --target ${{ matrix.target }} -p corp-finance-cli
      - Upload to GitHub Releases
```

### 5.5 Configuration

**MCP Server Configuration (claude_desktop_config.json):**

```json
{
  "mcpServers": {
    "corp-finance": {
      "command": "npx",
      "args": ["-y", "corp-finance-mcp"],
      "env": {}
    }
  }
}
```

**CLI Configuration (~/.config/cfa/config.toml):**

```toml
[defaults]
currency = "GBP"
output_format = "json"

[defaults.valuation]
equity_risk_premium = 0.055
terminal_growth = 0.025

[defaults.credit]
warning_threshold_coverage = 2.0
```

---

## 6. Performance Budget

| Operation | Target | Measurement |
|-----------|--------|-------------|
| WACC calculation | < 1ms | `Instant::now()` in Rust |
| DCF (10yr, 2-stage) | < 5ms | Including terminal value |
| LBO (10yr, 3 tranches) | < 20ms | Full model with waterfall |
| Credit metrics (all ratios) | < 2ms | From financial data input |
| Sensitivity (20×20 matrix) | < 50ms | 400 DCF calculations |
| Monte Carlo (10k runs) | < 500ms | Feature-gated, f64 permitted |
| XIRR convergence | < 2ms | Newton-Raphson, max 100 iterations |
| napi FFI overhead | < 0.5ms | JSON serialise + deserialise |
| MCP tool response | < 10ms | End-to-end excluding LLM |
| CLI cold start | < 200ms | Static binary, no runtime |

---

## 7. Security Model

| Concern | Mitigation |
|---------|------------|
| Rust core network access | None — pure computation, no `tokio` or `reqwest` in core |
| Input injection | Typed structs via serde, no string interpolation |
| Integer overflow | `rust_decimal` handles precision; checked arithmetic throughout |
| Denial of service (MC) | Max iteration limits, configurable timeout |
| Supply chain | Minimal dependency tree, `cargo audit` in CI |
| Secrets | None required — no API keys, no authentication |
| Data at rest | None — stateless, no persistence |

---

## 8. Extensibility

### 8.1 Adding a New Tool

1. Add Rust function in appropriate module (`crates/corp-finance-core/src/`)
2. Add unit tests with known-answer fixtures
3. Add napi binding in `packages/bindings/src/`
4. Add Zod schema in `packages/mcp-server/src/schemas/`
5. Register MCP tool in `packages/mcp-server/src/tools/`
6. Add CLI subcommand in `crates/corp-finance-cli/src/commands/`
7. Add E2E test

Each step touches exactly one file. No shotgun surgery.

### 8.2 Adding a New Module

1. Create module directory in Rust core
2. Add to feature gate in `Cargo.toml`
3. Create corresponding files in bindings, MCP, CLI, and tests
4. Follow existing module pattern exactly

### 8.3 Third-Party Integration

The Rust crate can be embedded in any Rust application. The npm package can be imported in any Node.js application. The CLI can be called from any language via subprocess. The MCP server can be consumed by any MCP client.
