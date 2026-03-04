# Product Requirements Document: Institutional Real Estate Analytics

**Product**: Autonomous CFA Analyst Platform
**Package**: @robotixai/corp-finance-mcp
**Version**: 1.0
**Date**: 2026-03-04
**Author**: RobotixAI Engineering

---

## 1. Overview

The existing `real_assets/real_estate.rs` module provides basic property valuation (Direct Capitalization, DCF, GRM). Institutional investors, however, require granular lease-level cash flow modeling, comparable sales adjustment grids, highest-and-best-use analysis, replacement cost approaches, benchmark attribution, and structured acquisition decision frameworks. This PRD specifies six sub-modules that bridge the gap between basic property valuation and institutional-grade CRE analytics suitable for REIT analysis, open-end fund underwriting, and direct property investment committees.

---

## 2. Requirements

### RE-001: Rent Roll Analysis

**Priority**: P0 (Critical)
**Description**: Argus-style tenant-by-tenant cash flow modeling with lease-level granularity. Each tenant lease is projected forward through escalations, renewals, and vacancy to produce an annual property cash flow schedule.
**Acceptance Criteria**:
- Accepts a vector of tenant lease records (tenant name, suite, SF, base rent/SF, start date, end date, escalation type, escalation rate, renewal probability, downtime months)
- Generates annual cash flow schedule from individual tenant leases for a configurable hold period (default 10 years)
- Models three escalation types: fixed percentage, CPI-linked (spread over index), and market reset at lease expiry
- Calculates WALT (weighted average lease term) by SF and by revenue
- Calculates occupancy rate, vacancy loss, and annual rollover schedule (SF expiring per year)
- Applies renewal probability and downtime assumptions per tenant at lease expiry
- Produces mark-to-market analysis comparing in-place rent vs market rent per SF with aggregate over/under-rented metric
- Returns effective gross income (EGI) after vacancy, credit loss, and expense reimbursements

### RE-002: Comparable Sales Analysis

**Priority**: P0 (Critical)
**Description**: Quantitative adjustment grid methodology for estimating market value from recent comparable property transactions.
**Acceptance Criteria**:
- Accepts minimum 3 comparable sale records (address, sale price, sale date, SF, year built, cap rate, property type, condition rating)
- Applies adjustment grid with categories: location, condition, size, age, amenities, market conditions (time adjustment)
- Each adjustment expressed as percentage; individual adjustment capped at +/-50% with total net adjustment capped at +/-100%
- Extracts price per SF and cap rate from each comp after adjustments
- Reconciliation via three methods: equal-weighted average, user-supplied weights, and most-similar-comp selection (minimum total absolute adjustment)
- Returns adjusted value range (low/mid/high), indicated cap rate range, and confidence score based on adjustment dispersion

### RE-003: Highest and Best Use Analysis

**Priority**: P1 (High)
**Description**: Four-test sequential framework per the Appraisal Institute methodology to determine the highest and best use of a site (as vacant and as improved).
**Acceptance Criteria**:
- Implements four sequential tests: legally permissible, physically possible, financially feasible, maximally productive
- Each test returns pass/fail boolean with structured rationale string
- Legally permissible: evaluates zoning code, density (FAR), height limit, setback, use restrictions
- Physically possible: evaluates lot size, shape, topography, soil conditions, access/utilities, environmental constraints
- Financially feasible: calculates residual land value (stabilized NOI / cap rate - construction cost) and returns feasible if residual > current land value
- Maximally productive: compares alternative use scenarios by IRR and selects the use with highest IRR as maximally productive
- Short-circuits: if any prior test fails, subsequent tests are skipped with status "not evaluated"
- Returns overall HBU conclusion with recommended use type and supporting metrics

### RE-004: Replacement Cost Approach

**Priority**: P1 (High)
**Description**: Cost approach to value using reproduction/replacement cost less depreciation plus land value, following Marshall & Swift methodology.
**Acceptance Criteria**:
- Calculates replacement cost new (RCN): base cost per SF x gross building area + site improvements + soft costs (as percentage of hard costs)
- Applies regional cost multiplier and time adjustment factor (Marshall & Swift framework)
- Calculates three-tier depreciation: physical (age-life method with effective age), functional obsolescence (curable and incurable), and external obsolescence (paired sales or capitalized income loss)
- Total depreciation capped at 95% of RCN (floor at 5% residual)
- Land value estimated via comparable land sales or residual land value extraction
- Returns replacement cost indication: RCN - total depreciation + land value
- Returns depreciation breakdown table with each tier's dollar amount and percentage of RCN

### RE-005: NCREIF/ODCE Benchmarking

**Priority**: P1 (High)
**Description**: Performance attribution and benchmarking against institutional real estate indices (NCREIF NPI, ODCE).
**Acceptance Criteria**:
- Decomposes quarterly total return into income return, appreciation return, and leverage effect
- Income return = NOI / beginning market value; appreciation return = (ending MV - beginning MV - capex) / beginning MV
- Leverage effect calculated as (unlevered return - cost of debt) x LTV / (1 - LTV)
- Compares property-level returns vs benchmark index returns over matching periods
- Calculates alpha (excess return), beta (regression vs benchmark), and tracking error (std dev of return differential)
- Relative value scoring: ranks property by sector and geography quintiles vs benchmark constituents
- Returns quarterly attribution table and cumulative performance summary

### RE-006: Acquisition Decision Model

**Priority**: P0 (Critical)
**Description**: Structured investment decision framework comparing hold, sell, develop, and value-add strategies with a configurable go/no-go decision matrix.
**Acceptance Criteria**:
- Computes NPV for three scenarios: hold (status quo cash flows), sell (net sale proceeds at assumed exit cap), and develop (land + hard/soft costs vs stabilized value)
- Value-add IRR: models capex budget, renovation downtime, rent-up schedule to stabilized occupancy, and exit at target cap rate
- Development feasibility: total development cost (land + hard costs + soft costs + financing carry) vs stabilized value; returns residual profit and development spread
- Refinancing scenario: models new loan terms (rate, term, amortization, LTV) with cash-out proceeds and impact on levered returns
- Go/No-Go decision matrix evaluates configurable thresholds: minimum IRR, maximum basis per SF, minimum equity multiple, minimum DSCR, maximum LTV
- Each threshold returns pass/fail; overall recommendation is Go (all pass), Conditional (1-2 fail), or No-Go (3+ fail)
- Returns side-by-side comparison table of all scenarios with key metrics (IRR, equity multiple, NPV, DSCR, cash-on-cash)

---

## 3. Technical Constraints

- All financial math in `rust_decimal::Decimal` (no f64 except Monte Carlo paths if added later)
- Target ~240 new unit tests (~40 per sub-module) in `crates/corp-finance-core/src/real_assets/`
- 6 new MCP tools registered in `packages/mcp-server/` (one per sub-module)
- ~12 new NAPI bindings in `crates/corp-finance-bindings/`
- 2 new slash commands in `.claude/commands/cfa/`: `property-analysis.md`, `acquisition-model.md`
- Must integrate with existing `real_assets` module; reuse shared types from `real_assets/mod.rs`
- New Rust types placed in `crates/corp-finance-core/src/real_assets/institutional_re.rs` (or sub-modules)
- Feature flag: `real_assets` (existing feature, no new feature flag needed)

---

## 4. Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| Institutional CRE MCP tools | 0 | 6 |
| Real estate unit tests | ~20 | ~260 |
| NAPI bindings (real estate) | ~4 | ~16 |
| CFA slash commands (real estate) | 0 | 2 |
| Rent roll / lease-level modeling | Not supported | Full Argus-style |
| Benchmark attribution | Not supported | NCREIF/ODCE quarterly |
| Acquisition decision framework | Not supported | 4-scenario go/no-go |

---

## 5. Out of Scope

- Real-time market data feeds (use FMP MCP server for market context where needed)
- GIS/mapping integration or spatial analysis
- Property management operations (tenant billing, maintenance, work orders)
- Brokerage and transaction management (listing, offers, closings)
- Construction project management or draw scheduling
- Mortgage-backed securities analytics (covered by existing `mortgage_analytics` module)
- REIT financial statement modeling (covered by existing `three_statement` module)
