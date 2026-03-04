# Domain-Driven Design Supplement: Institutional Real Estate

## Overview

This supplement defines the **Commercial Real Estate Analytics** bounded context, a new context within the Real Assets domain that extends the existing `real_assets` bounded context with institutional-grade CRE analysis capabilities. It covers six sub-domains: rent roll analysis, comparable sales adjustment, highest-and-best-use testing, replacement cost estimation, benchmark attribution, and acquisition underwriting.

The existing `real_assets::real_estate` module provides basic property valuation (direct cap, DCF, GRM). This context deepens that foundation with the granular data structures and domain logic required by institutional investors, REITs, and fund managers.

---

## Bounded Context: Commercial Real Estate Analytics

**Type**: Supporting Domain (extends Core Real Assets)

**Responsibility**: Provide institutional-quality commercial real estate analysis including tenant-level rent roll analytics, market comparable adjustment grids, highest-and-best-use feasibility, replacement cost estimation, NCREIF/ODCE benchmark attribution, and acquisition decision modeling with levered/unlevered return targets.

### Relationship to Existing `real_assets`

This context does **not** replace the existing `real_assets::real_estate` module. It extends it:

- **Shared kernel**: `Money`, `Rate`, `ComputationOutput`, `CorpFinanceResult` types from `crate::types`
- **Reuses**: `PropertyValuationInput`, `ValuationMethod`, `ComparableSale` as base types
- **Adds**: Six new sub-modules (`rent_roll`, `comparable_sales`, `highest_best_use`, `replacement_cost`, `benchmark`, `acquisition`) under `crate::institutional_real_estate`

### Anti-Corruption Layer

CRE-specific concepts (rent steps, TI/LC allowances, absorption rates, cap rate adjustments) must not leak into generic financial primitives. The ACL operates at two boundaries:

| Boundary | Direction | Mechanism |
|----------|-----------|-----------|
| CRE -> Generic Finance | Outbound | CRE functions return `ComputationOutput<T>` wrapping CRE-specific output structs; consumers see standard result envelopes |
| Generic Finance -> CRE | Inbound | CRE modules import `Money`, `Rate`, `Decimal` but never expose raw Decimal math to callers; all intermediate CRE calculations (e.g., rent step compounding, adjustment grid netting) remain internal |

---

## Aggregates

### 1. Property (Root Aggregate)

The central aggregate representing a commercial real estate asset. All sub-domain analyses reference a Property.

| Field | Type | Description |
|-------|------|-------------|
| `property_id` | `String` | Unique identifier |
| `property_name` | `String` | Display name |
| `address` | `Address` | Street, city, state, zip, country |
| `property_type` | `PropertyType` | Office, Retail, Industrial, Multifamily, Mixed-Use, Hospitality |
| `total_sf` | `Decimal` | Total leasable square footage |
| `land_area_sf` | `Decimal` | Land parcel area |
| `year_built` | `u32` | Original construction year |
| `year_renovated` | `Option<u32>` | Most recent major renovation |
| `zoning` | `String` | Current zoning classification |
| `noi` | `Money` | Current net operating income |
| `purchase_price` | `Option<Money>` | Acquisition price (if owned) |

**Invariants enforced by the aggregate root**:
- `total_sf > 0`
- `noi = egi - operating_expenses` (derived, never set independently)
- `property_type` must be a valid enum variant

### 2. RentRoll (Aggregate)

A collection of `Tenant` value objects representing the in-place lease schedule for a Property.

| Field | Type | Description |
|-------|------|-------------|
| `property_id` | `String` | References parent Property |
| `as_of_date` | `String` | Snapshot date (YYYY-MM-DD) |
| `tenants` | `Vec<Tenant>` | All current leases |
| `vacancy_sf` | `Decimal` | Unoccupied square footage |

**Commands**:
- `AnalyzeRentRoll` -- computes WALT, occupancy, rollover schedule, mark-to-market gap
- `ProjectRentRoll` -- forward-projects cash flows with contractual escalations and market reversion

**Invariants**:
- Sum of all `tenant.sf` must not exceed the parent Property's `total_sf`
- No two tenants may occupy overlapping suites with overlapping date ranges
- `vacancy_sf = total_sf - sum(tenant.sf)` (derived)

### 3. ComparableSalesSet (Aggregate)

A curated set of market comparables with adjustment grids for the sales comparison approach.

| Field | Type | Description |
|-------|------|-------------|
| `subject_property_id` | `String` | The property being valued |
| `comparables` | `Vec<Comparable>` | Market transactions |
| `adjustment_categories` | `Vec<String>` | Active adjustment dimensions |

**Commands**:
- `AdjustComparables` -- applies adjustment grid to each comparable, derives adjusted price PSF
- `ReconcileValue` -- weights adjusted comparables to a single indicated value

**Invariants**:
- Each comparable's net adjustment must be within +/-50% (anti-manipulation guard)
- At least 3 comparables required for a defensible opinion of value
- All comparables must have a positive sale price and positive square footage

### 4. AcquisitionModel (Aggregate)

Encapsulates the buy/hold/sell decision for an institutional acquisition with levered and unlevered return targets.

| Field | Type | Description |
|-------|------|-------------|
| `property_id` | `String` | Target property |
| `purchase_price` | `Money` | Offered price |
| `equity_investment` | `Money` | LP/GP equity |
| `loan_amount` | `Money` | Senior debt |
| `loan_rate` | `Rate` | Annual interest rate |
| `loan_term_years` | `u32` | Loan maturity |
| `holding_period_years` | `u32` | Target hold |
| `exit_cap_rate` | `Rate` | Reversion cap rate |
| `rent_growth` | `Rate` | Annual NOI growth assumption |
| `target_irr` | `Rate` | Minimum acceptable IRR |
| `target_equity_multiple` | `Decimal` | Minimum acceptable MOIC |
| `target_cash_yield` | `Rate` | Minimum year-1 cash-on-cash |

**Commands**:
- `UnderwriteAcquisition` -- runs full acquisition model (unlevered IRR, levered IRR, equity multiple, cash-on-cash, max bid price)
- `SensitivityAnalysis` -- varies cap rate, rent growth, and exit cap to produce return surface

**Invariants**:
- `purchase_price = equity_investment + loan_amount` (sources = uses)
- Cap rate must be positive and less than 100%
- Holding period must be >= 1 year
- Exit cap rate must be positive

---

## Value Objects

All value objects are immutable once constructed. They carry no identity; equality is determined by field values.

### Tenant

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Tenant legal name |
| `suite` | `String` | Suite or unit identifier |
| `sf` | `Decimal` | Leased square footage |
| `rent_psf` | `Money` | Annual rent per SF |
| `lease_start` | `String` | Lease commencement (YYYY-MM-DD) |
| `lease_end` | `String` | Lease expiration (YYYY-MM-DD) |
| `options` | `Vec<LeaseOption>` | Renewal/expansion/termination options |
| `escalations` | `Vec<Escalation>` | Rent step schedule |
| `expense_stop` | `Option<Money>` | Base-year expense stop |
| `ti_allowance` | `Option<Money>` | Tenant improvement allowance PSF |
| `lc_pct` | `Option<Rate>` | Leasing commission as % of total rent |

### LeaseOption

| Field | Type | Description |
|-------|------|-------------|
| `option_type` | `OptionType` | Renewal, Expansion, Termination, ROFO, ROFR |
| `notice_months` | `u32` | Notice period required |
| `term_years` | `Option<u32>` | Option term if exercised |
| `rent_basis` | `Option<RentBasis>` | FMV, Fixed, CPI-indexed |

### Escalation

| Field | Type | Description |
|-------|------|-------------|
| `escalation_type` | `EscalationType` | Fixed, CPI, PercentPerAnnum, StepSchedule |
| `rate` | `Option<Rate>` | Annual increase rate |
| `schedule` | `Option<Vec<(u32, Money)>>` | Year-by-year explicit steps |

### Comparable

| Field | Type | Description |
|-------|------|-------------|
| `address` | `String` | Property address |
| `sale_date` | `String` | Transaction date (YYYY-MM-DD) |
| `sale_price` | `Money` | Gross sale price |
| `sf` | `Decimal` | Building area |
| `cap_rate` | `Rate` | Implied cap rate at sale |
| `noi` | `Money` | NOI at time of sale |
| `property_type` | `PropertyType` | Asset class |
| `year_built` | `u32` | Vintage |
| `adjustments` | `Vec<Adjustment>` | Applied adjustment grid |

### Adjustment

| Field | Type | Description |
|-------|------|-------------|
| `category` | `AdjustmentCategory` | Location, Condition, Size, Age, Amenities, MarketConditions, Financing |
| `pct_adj` | `Rate` | Percentage adjustment (positive = comp inferior, negative = comp superior) |
| `narrative` | `String` | Appraiser's rationale |

### HbuTest

| Field | Type | Description |
|-------|------|-------------|
| `test_name` | `HbuTestName` | LegallyPermissible, PhysicallyPossible, FinanciallyFeasible, MaximallyProductive |
| `passes` | `bool` | Whether the test is satisfied |
| `rationale` | `String` | Supporting analysis |
| `value_if_developed` | `Option<Money>` | Estimated value under the tested use |

### DepreciationSchedule

| Field | Type | Description |
|-------|------|-------------|
| `useful_life_years` | `u32` | Economic useful life |
| `method` | `DepreciationMethod` | StraightLine, DecliningBalance |
| `salvage_value` | `Money` | Residual value at end of life |
| `effective_age` | `u32` | Current effective age |
| `replacement_cost_new` | `Money` | Cost to reproduce as new |
| `accrued_depreciation` | `Money` | Total depreciation to date |

### BenchmarkPeriod

| Field | Type | Description |
|-------|------|-------------|
| `period` | `String` | Quarter identifier (e.g., "2025-Q4") |
| `property_return_pct` | `Rate` | Subject property total return |
| `index_return_pct` | `Rate` | Benchmark index return (NCREIF NPI, ODCE) |
| `alpha` | `Rate` | Excess return (property - index) |
| `income_return` | `Rate` | Income component of return |
| `appreciation_return` | `Rate` | Capital appreciation component |
| `attribution` | `ReturnAttribution` | Sector, geography, leverage attribution |

---

## Domain Events

Events are raised by aggregate commands and consumed by downstream contexts or stored in the event log.

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `PropertyValuationCompleted` | Property | `property_id`, `valuation_method`, `indicated_value`, `cap_rate`, `timestamp` | Financial Memory, Analysis Orchestration |
| `RentRollAnalyzed` | RentRoll | `property_id`, `occupancy_pct`, `walt_years`, `mark_to_market_gap`, `timestamp` | AcquisitionModel (informs NOI projection) |
| `ComparableSalesReconciled` | ComparableSalesSet | `property_id`, `indicated_value_psf`, `comparable_count`, `avg_net_adjustment`, `timestamp` | Property (feeds sales comparison value) |
| `HbuAnalysisCompleted` | Property | `property_id`, `highest_use`, `value_if_developed`, `all_tests_passed`, `timestamp` | AcquisitionModel (land residual input) |
| `ReplacementCostEstimated` | Property | `property_id`, `replacement_cost_new`, `accrued_depreciation`, `depreciated_value`, `timestamp` | Property (feeds cost approach value) |
| `AcquisitionDecisionRendered` | AcquisitionModel | `property_id`, `decision` (Go/NoGo/Conditional), `unlevered_irr`, `levered_irr`, `equity_multiple`, `max_bid`, `timestamp` | Analysis Orchestration, Financial Memory |
| `BenchmarkComparisonCompleted` | Property | `property_id`, `benchmark_index`, `period_count`, `cumulative_alpha`, `information_ratio`, `timestamp` | Financial Memory, Learning & Adaptation |

---

## Invariants

These invariants are enforced at the aggregate boundary. Violations result in `CorpFinanceError::InvalidInput`.

| ID | Invariant | Aggregate | Rationale |
|----|-----------|-----------|-----------|
| CRE-INV-001 | NOI = EGI - OpEx (no double counting) | Property | Fundamental accounting identity; EGI = GPR - Vacancy + Other Income |
| CRE-INV-002 | Cap rate > 0% and < 100% | Property, ComparableSalesSet, AcquisitionModel | A zero or negative cap rate implies infinite or negative value; >= 100% implies NOI exceeds value |
| CRE-INV-003 | Sum of tenant SF <= building total SF | RentRoll | Physical constraint; cannot lease more space than exists |
| CRE-INV-004 | Net comparable adjustments within +/-50% | ComparableSalesSet | Anti-manipulation guard; adjustments beyond 50% indicate the comparable is not truly comparable |
| CRE-INV-005 | HBU tests must pass sequentially: Legal -> Physical -> Financial -> Maximal | Property (HBU) | Appraisal Institute standard; a use that fails an earlier test cannot be evaluated at a later stage |
| CRE-INV-006 | Purchase price = equity + debt | AcquisitionModel | Sources must equal uses; capital structure must balance |
| CRE-INV-007 | Holding period >= 1 year | AcquisitionModel | Sub-annual holds are merchant/flip transactions outside this model's scope |
| CRE-INV-008 | At least 3 comparables for reconciliation | ComparableSalesSet | Professional appraisal standard for defensible value conclusions |

---

## Context Map

```
+-----------------------------------------------------------------------+
|                    COMMERCIAL REAL ESTATE ANALYTICS                    |
|                                                                       |
|  +-------------------+     +-----------------------+                  |
|  |   RentRoll        |---->|   AcquisitionModel    |                  |
|  |   (NOI input)     |     |   (underwriting)      |                  |
|  +-------------------+     +-----------+-----------+                  |
|          |                             |                              |
|          |  +-------------------+      |                              |
|          +->|   Property        |<-----+                              |
|             |   (root aggregate)|                                     |
|             +--------+----------+                                     |
|                      ^                                                |
|  +-------------------+---+   +-------------------+                    |
|  | ComparableSalesSet    |   | HBU / Replacement |                    |
|  | (sales comparison)    |   | (cost approach)   |                    |
|  +-----------------------+   +-------------------+                    |
|                                                                       |
+----+-----------+----------+-----------+-------------+-----------------+
     |           |          |           |             |
     v           v          v           v             v
 Real Assets  Lease Acctg  Fixed Inc  Onshore Str   PE/Waterfall
 (extends)    (IFRS 16)   (NPV/DF)   (REIT)        (LP/GP dist)
```

### Context Relationships

| Upstream | Downstream | Relationship | Integration Detail |
|----------|------------|-------------|-------------------|
| **Real Assets** | CRE Analytics | Shared Kernel (extends) | CRE imports `Money`, `Rate`, `ComputationOutput`, `ValuationMethod`, `ComparableSale` from `crate::real_assets::real_estate` and `crate::types`. CRE modules live alongside existing real estate code. |
| **CRE Analytics** | Lease Accounting | Published Language | `RentRollAnalyzed` events carry lease term data consumed by `lease_accounting` for IFRS 16 right-of-use asset classification and lease liability calculation. The ACL translates CRE `Tenant` into lease accounting's `LeaseContract` type. |
| **Fixed Income** | CRE Analytics | Customer/Supplier | CRE acquisition models consume discount factor and NPV functions from `fixed_income` for DCF-based property valuation. CRE is the customer; fixed income supplies the math. |
| **Onshore Structures** | CRE Analytics | Partnership | REIT structuring logic from `onshore_structures::us_funds` informs CRE acquisition models on tax-optimized holding structures (95% distribution requirement, asset/income tests). Bidirectional: CRE provides NOI projections, onshore structures provides entity-level tax implications. |
| **PE/Waterfall** | CRE Analytics | Customer/Supplier | Real estate fund vehicles use `pe::waterfall` for LP/GP distribution modeling. CRE acquisition models produce property-level cash flows; waterfall logic distributes them per the fund's partnership agreement (preferred return, catch-up, carried interest). |
| **CRE Analytics** | Financial Memory | Publisher/Subscriber | All domain events (`PropertyValuationCompleted`, `AcquisitionDecisionRendered`, etc.) are published to Financial Memory for retrieval-augmented analysis in future queries. |
| **CRE Analytics** | Hosted MCP Gateway | Conformist | CRE tools conform to the MCP tool registration schema. Each sub-domain exposes tools via NAPI bindings following the existing `JSON string -> JSON string` boundary pattern. |

---

## Sub-Domain Breakdown

### 1. Rent Roll (`rent_roll`)

**Purpose**: Tenant-level lease analysis including weighted-average lease term (WALT), occupancy metrics, mark-to-market analysis, and rollover scheduling.

**Key Computations**:
- WALT (weighted by SF or by rent)
- Occupancy rate (physical and economic)
- In-place rent vs. market rent gap (mark-to-market)
- Rollover schedule by year (SF and rent expiring)
- Effective gross rent after TI/LC amortization

### 2. Comparable Sales (`comparable_sales`)

**Purpose**: Sales comparison approach with institutional-grade adjustment grids.

**Key Computations**:
- Per-comparable adjustment grid (location, condition, size, age, amenities, market conditions, financing)
- Adjusted price PSF per comparable
- Reconciled indicated value (weighted by quality/recency)
- Statistical measures (mean, median, range, standard deviation of adjusted prices)

### 3. Highest and Best Use (`highest_best_use`)

**Purpose**: Four-test HBU analysis per Appraisal Institute standards.

**Key Computations**:
- Legally permissible use screening (zoning, deed restrictions, environmental)
- Physically possible use screening (size, shape, topography, utilities)
- Financially feasible analysis (residual land value > as-is value)
- Maximally productive use selection (highest residual value among feasible uses)

### 4. Replacement Cost (`replacement_cost`)

**Purpose**: Cost approach valuation using Marshall & Swift or comparable construction cost data.

**Key Computations**:
- Replacement cost new (RCN) estimation
- Accrued depreciation (physical, functional, external obsolescence)
- Depreciated replacement cost = RCN - accrued depreciation + land value
- Effective age / remaining useful life estimation

### 5. Benchmark (`benchmark`)

**Purpose**: Performance attribution against institutional real estate benchmarks (NCREIF NPI, ODCE, custom peer indices).

**Key Computations**:
- Time-weighted return (income + appreciation components)
- Alpha calculation vs. benchmark index
- Information ratio (alpha / tracking error)
- Return attribution by sector, geography, and leverage contribution
- Rolling period analysis (1yr, 3yr, 5yr, since-inception)

### 6. Acquisition (`acquisition`)

**Purpose**: Full acquisition underwriting model with go/no-go decision framework.

**Key Computations**:
- Unlevered IRR (property-level, before debt service)
- Levered IRR (equity-level, after debt service)
- Equity multiple (MOIC)
- Cash-on-cash yield (year 1 and stabilized)
- Max bid price (solving for target IRR via Newton-Raphson)
- Sensitivity tables (cap rate x rent growth, exit cap x hold period)
- Debt service coverage ratio (DSCR) by year

---

## MCP Tool Mapping

Each sub-domain maps to one or more MCP tools following the existing naming convention.

| Sub-Domain | Tool Name | Description |
|------------|-----------|-------------|
| rent_roll | `analyze_rent_roll` | Compute WALT, occupancy, rollover schedule, mark-to-market |
| rent_roll | `project_rent_roll` | Forward cash flow projection with escalations and market reversion |
| comparable_sales | `adjust_comparables` | Apply adjustment grid, compute adjusted PSF |
| comparable_sales | `reconcile_sales_value` | Weight and reconcile comparables to indicated value |
| highest_best_use | `analyze_hbu` | Run four-test HBU analysis |
| replacement_cost | `estimate_replacement_cost` | Cost approach with depreciation schedule |
| benchmark | `compare_benchmark` | Property vs. index return attribution |
| acquisition | `underwrite_acquisition` | Full acquisition model with levered/unlevered returns |
| acquisition | `acquisition_sensitivity` | Return sensitivity tables across key assumptions |

---

## Domain Model Impact Summary

| Bounded Context | Change Type | Impact |
|----------------|-------------|--------|
| Real Assets | Extension (6 new sub-modules) | High -- significant new aggregate and value object surface area |
| Lease Accounting | New event consumer | Low -- receives `RentRollAnalyzed` events via ACL |
| Fixed Income | Existing supplier | None -- CRE calls existing NPV/DF functions |
| Onshore Structures | Partnership integration | Low -- bidirectional data flow for REIT analysis |
| PE/Waterfall | Existing supplier | None -- CRE calls existing waterfall distribution logic |
| Hosted MCP Gateway | 9 new tools registered | Medium -- new NAPI bindings and MCP tool registrations |
| Analysis Orchestration | New routing targets | Low -- SemanticRouter gains CRE intent classifications |
| Financial Memory | New event types stored | Low -- standard event storage, no schema changes |
