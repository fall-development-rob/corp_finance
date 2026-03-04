# ADR-013: Institutional Commercial Real Estate Analytics

## Status: Proposed

## Date: 2026-03-04

## Context

The CFA agent platform includes a `real_assets/real_estate.rs` module (Phase 5) with foundational property valuation: Direct Capitalisation, DCF, Gross Rent Multiplier, NOI calculation, DSCR, and cash-on-cash return. Adjacent modules cover project finance PPP modeling (`real_assets/project_finance.rs`, `infrastructure/ppp_model.rs`), lease accounting under IFRS 16 / ASC 842 (`lease_accounting/`), sale-leaseback analysis, and REIT structuring (`onshore_structures/`).

However, the platform lacks **institutional-grade commercial real estate analytics** expected by pension funds, open-end real estate funds, REITs, and private equity real estate (PERE) sponsors. Specific gaps include:

| Gap | Why It Matters |
|-----|---------------|
| No tenant-by-tenant rent roll modeling | Argus Enterprise / Yardi-style cash flow projections are the industry standard for underwriting; lump-sum NOI is insufficient for lease rollover risk analysis |
| No comparable sales adjustment grid | Appraisers and acquisition teams require quantitative adjustment grids (location, size, condition, age, lease terms) to reconcile comparable transactions |
| No highest-and-best-use (HBU) analysis | Required by USPAP Standards Rule 1-3 and IVS 104 for all property appraisals; four-tests framework (legal, physical, financial, maximal) is absent |
| No replacement cost / cost approach | The cost approach (Marshall & Swift / RS Means methodology) is required for insurance valuation, property tax appeals, and special-use property appraisal |
| No NCREIF/ODCE benchmark attribution | Institutional investors benchmark against NCREIF NPI and ODCE; attribution of returns into income, appreciation, and leverage components is missing |
| No acquisition / hold-sell / development modeling | PERE sponsors need full lifecycle decision frameworks: buy vs. build, value-add IRR waterfall, hold-sell break-even, refinancing analysis |

The existing `real_estate.rs` module provides the correct mathematical foundation (NOI, cap rates, DCF) but operates at the property summary level. Institutional analytics require tenant-level granularity, multi-method reconciliation, and benchmark-relative performance measurement.

## Decision

Add a new `institutional_real_estate` Rust module under `crates/corp-finance-core/src/` as Phase 22, containing 6 sub-modules with approximately 30 public functions. The module extends (does not replace) the existing `real_assets/real_estate` module and integrates with `lease_accounting` and `onshore_structures` (REIT) where appropriate.

### Module Structure

```
crates/corp-finance-core/src/institutional_real_estate/
  mod.rs
  rent_roll.rs
  comparable_sales.rs
  highest_best_use.rs
  replacement_cost.rs
  benchmark.rs
  acquisition.rs
```

Feature flag: `institutional_real_estate` (added to the workspace `Cargo.toml` feature list, gated under `--features institutional_real_estate`).

### Sub-module Specifications

#### 1. rent_roll -- Argus-Style Tenant Cash Flow Modeling

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `tenant_schedule` | `(TenantScheduleInput) -> TenantScheduleOutput` | Projects gross rent, reimbursements, free rent, and TI amortisation for a single tenant over a holding period. Handles base rent escalations (fixed step, CPI-linked, percentage rent), expense stops and pass-throughs, and renewal/vacancy assumptions. |
| `lease_rollover` | `(LeaseRolloverInput) -> LeaseRolloverOutput` | Aggregates tenant schedules into a property-level lease expiration profile. Computes rolling WALT, annual rollover exposure (% of NRA and % of base rent), and cumulative rollover risk by year. |
| `renewal_probability` | `(RenewalProbabilityInput) -> RenewalProbabilityOutput` | Estimates renewal likelihood per tenant using lease remaining term, tenant credit quality (S&P/Moody's rating or custom score), market vacancy, and in-place vs. market rent spread. Outputs probability-weighted vacancy and downtime costs. |
| `mark_to_market` | `(MarkToMarketInput) -> MarkToMarketOutput` | Compares in-place rent per SF to market rent per SF for each tenant. Computes aggregate mark-to-market delta (positive = below market / upside, negative = above market / risk), weighted by remaining lease term. |
| `weighted_avg_lease_term` | `(WaltInput) -> WaltOutput` | Computes WALT weighted by (a) base rent, (b) NRA, or (c) both. Returns WALT in years and months, plus lease term distribution histogram (0-1yr, 1-3yr, 3-5yr, 5-10yr, 10yr+). |

**Key types**: `Tenant` struct (name, suite, sf, lease_start, lease_end, base_rent_psf, escalation_type, expense_stop, ti_allowance, credit_rating), `EscalationType` enum (FixedStep, CpiLinked, PercentageRent, FlatRent).

#### 2. comparable_sales -- Quantitative Adjustment Grid

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `comp_adjustment_grid` | `(CompAdjustmentInput) -> CompAdjustmentOutput` | Applies quantitative adjustments to comparable sales along standardised dimensions: property rights conveyed, financing terms, conditions of sale, market conditions (time), location, physical characteristics (size, age, condition, quality), and economic characteristics (occupancy, lease terms, expense ratio). Each adjustment is a percentage or dollar-per-SF delta. Outputs adjusted price per SF and implied cap rate for each comparable. |
| `price_per_sf` | `(PricePerSfInput) -> PricePerSfOutput` | Normalises sale prices to price-per-SF (or per-unit for multifamily, per-key for hospitality, per-bed for senior housing). Handles GBA, NRA, and usable area bases. |
| `cap_rate_extraction` | `(CapRateExtractionInput) -> CapRateExtractionOutput` | Extracts going-in cap rate, trailing-twelve-month cap rate, and forward cap rate from comparable transactions. Applies NOI normalization (vacancy adjustment, expense ratio standardisation, CapEx reserve deduction). |
| `reconciliation` | `(ReconciliationInput) -> ReconciliationOutput` | Reconciles multiple adjusted comparable indicators into a single value conclusion. Supports equal weighting, quality-score weighting (based on comparability rating 1-5), and inverse-distance weighting. Outputs reconciled value, confidence interval, and coefficient of variation. |

**Adjustment categories** follow the Appraisal Institute's standard sequence (transactional adjustments before property adjustments).

#### 3. highest_best_use -- Four-Tests HBU Framework

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `hbu_analysis` | `(HbuAnalysisInput) -> HbuAnalysisOutput` | Orchestrates the full four-tests framework. Runs each test sequentially (legal, physical, financial, maximal) and returns the HBU conclusion with supporting rationale for each test. |
| `legal_permissible` | `(LegalPermissibleInput) -> LegalPermissibleOutput` | Evaluates zoning (use class, FAR, height limit, setbacks, lot coverage), deed restrictions, environmental restrictions (wetlands, flood zone, brownfield), and historic designation. Returns a list of legally permissible uses with constraint details. |
| `physically_possible` | `(PhysicallyPossibleInput) -> PhysicallyPossibleOutput` | Evaluates site constraints: lot size and shape, topography, soil/geotechnical conditions, utilities availability, access/ingress-egress, and environmental remediation cost. Filters legally permissible uses to those that are physically achievable. |
| `financially_feasible` | `(FinanciallyFeasibleInput) -> FinanciallyFeasibleOutput` | For each physically possible use, estimates development cost (hard + soft + land), stabilised NOI, residual land value (income approach minus cost-to-build), and development yield-on-cost. Uses with positive residual land value pass. |
| `maximally_productive` | `(MaximallyProductiveInput) -> MaximallyProductiveOutput` | Ranks financially feasible uses by residual land value per SF of site area. The use producing the highest residual land value is the HBU. Returns ranked alternatives with IRR, equity multiple, and residual value for each. |

**Compliance**: Output includes USPAP Standards Rule 1-3(b) and IVS 104 cross-references.

#### 4. replacement_cost -- Cost Approach Valuation

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `cost_approach` | `(CostApproachInput) -> CostApproachOutput` | Full cost approach: land value (from comparable land sales or residual extraction) + replacement cost new (RCN) - depreciation = indicated value. Supports both replacement cost (modern equivalent) and reproduction cost (exact replica). |
| `depreciation_schedule` | `(DepreciationScheduleInput) -> DepreciationScheduleOutput` | Computes total depreciation from three sources: physical deterioration (age-life method: effective age / total economic life), functional obsolescence (superadequacy, deficiency, or layout inefficiency), and external/economic obsolescence (market conditions, location deterioration). Each component expressed as percentage of RCN. |
| `land_residual` | `(LandResidualInput) -> LandResidualOutput` | Extracts land value by subtracting depreciated improvement value from total property value (or sale price). Used when comparable land sales are unavailable. Cross-validates against comparable land sales when provided. |
| `marshall_swift` | `(MarshallSwiftInput) -> MarshallSwiftOutput` | Applies Marshall & Swift / CoreLogic cost estimation methodology: base cost per SF by building class (A/B/C/D/S) and occupancy type (office, retail, industrial, multifamily, hospitality), adjusted by current cost multiplier, local cost modifier (city-specific), height/story multiplier, perimeter multiplier, and sprinkler/HVAC add-ons. Returns RCN and RCN less depreciation. |

**Building classes**: Class A (fireproof steel/concrete), Class B (reinforced concrete), Class C (masonry bearing walls), Class D (wood frame), Class S (pre-engineered metal).

#### 5. benchmark -- NCREIF/ODCE Performance Attribution

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `ncreif_attribution` | `(NcreifAttributionInput) -> NcreifAttributionOutput` | Decomposes total return into NCREIF NPI components: income return (NOI / market value), appreciation return (capital value change / beginning market value), and total return. Applies the NCREIF time-weighted methodology with quarterly chain-linking. Handles leverage adjustment for levered vs. unlevered comparison. |
| `odce_comparison` | `(OdceComparisonInput) -> OdceComparisonOutput` | Compares fund/property returns against the NCREIF ODCE (Open-End Diversified Core Equity) index. Computes excess return, tracking error, information ratio, and Sharpe ratio. Breaks down performance attribution into allocation effect (property type and geography weights vs. index), selection effect (within-sector outperformance), and interaction effect. |
| `property_index` | `(PropertyIndexInput) -> PropertyIndexOutput` | Constructs a property-level or portfolio-level total return index from periodic appraisal values, NOI, and capital expenditures. Supports both equal-weighted and value-weighted methodologies. Computes rolling 1/3/5/10-year returns, volatility (quarterly and annual), and drawdown metrics. |
| `relative_value` | `(RelativeValueInput) -> RelativeValueOutput` | Computes relative value metrics for acquisitions: cap rate spread to NCREIF average, cap rate spread to risk-free rate (10-year Treasury), implied risk premium, and price-per-SF vs. replacement cost ratio. Flags assets trading at a premium/discount to benchmark. |

**Standards**: NCREIF PREA Reporting Standards, GIPS for Real Estate (CFA Institute).

#### 6. acquisition -- Buy/Hold/Sell/Develop Decision Framework

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `acquisition_model` | `(AcquisitionModelInput) -> AcquisitionModelOutput` | Full acquisition underwriting: sources & uses, year-by-year pro forma (integrating rent roll projections from sub-module 1), annual debt service, cash-on-cash return, equity multiple, and levered/unlevered IRR (Newton-Raphson). Supports senior + mezzanine debt tranches with IO periods and amortisation schedules. |
| `hold_sell_analysis` | `(HoldSellInput) -> HoldSellOutput` | Compares holding NPV (present value of remaining cash flows + terminal value at assumed exit cap rate) vs. selling NPV (net sale proceeds after disposition costs and taxes). Computes break-even exit cap rate and optimal hold period (year maximising equity IRR). |
| `value_add_irr` | `(ValueAddIrrInput) -> ValueAddIrrOutput` | Models value-add business plan: renovation CapEx schedule, lease-up timeline, stabilised NOI target, and exit. Computes gross and net IRR, equity multiple, peak equity requirement, and return-on-cost. Supports GP/LP waterfall with preferred return, catch-up, and promote tiers (integrates with existing `pe/waterfall` module). |
| `development_feasibility` | `(DevelopmentFeasibilityInput) -> DevelopmentFeasibilityOutput` | Ground-up development analysis: land cost, hard costs (by building system), soft costs (architecture, engineering, legal, permits, financing), construction draw schedule, lease-up period, and stabilised operations. Computes development yield-on-cost, development spread (yield minus market cap rate), residual land value, and profit margin. |
| `refinancing` | `(RefinancingInput) -> RefinancingOutput` | Evaluates refinancing alternatives: compares existing debt terms to proposed terms, computes NPV of interest savings, prepayment penalty / defeasance cost, break-even period, and post-refinancing LTV/DSCR/debt yield. Supports rate-and-term refi and cash-out refi scenarios. |

### MCP Tools

6 new MCP tools (one per sub-module), registered in `packages/mcp-server/`:

| Tool Name | Sub-module | Description |
|-----------|-----------|-------------|
| `institutional_rent_roll` | rent_roll | Tenant-by-tenant cash flow projection and lease rollover analysis |
| `institutional_comparable_sales` | comparable_sales | Comparable sales adjustment grid and value reconciliation |
| `institutional_hbu_analysis` | highest_best_use | Four-tests highest-and-best-use framework |
| `institutional_replacement_cost` | replacement_cost | Cost approach valuation with Marshall & Swift methodology |
| `institutional_benchmark` | benchmark | NCREIF/ODCE performance attribution and relative value |
| `institutional_acquisition` | acquisition | Acquisition underwriting, hold/sell, value-add, and development feasibility |

This brings the total MCP tool count from 200 to 206.

### NAPI Bindings

12 new NAPI bindings in `crates/corp-finance-bindings/src/`:

| Binding | Maps To |
|---------|---------|
| `napi_tenant_schedule` | rent_roll::tenant_schedule |
| `napi_lease_rollover` | rent_roll::lease_rollover |
| `napi_comp_adjustment_grid` | comparable_sales::comp_adjustment_grid |
| `napi_comp_reconciliation` | comparable_sales::reconciliation |
| `napi_hbu_analysis` | highest_best_use::hbu_analysis |
| `napi_hbu_financially_feasible` | highest_best_use::financially_feasible |
| `napi_cost_approach` | replacement_cost::cost_approach |
| `napi_marshall_swift` | replacement_cost::marshall_swift |
| `napi_ncreif_attribution` | benchmark::ncreif_attribution |
| `napi_odce_comparison` | benchmark::odce_comparison |
| `napi_acquisition_model` | acquisition::acquisition_model |
| `napi_development_feasibility` | acquisition::development_feasibility |

All bindings follow the existing JSON string boundary pattern (`String -> String` via serde).

### Slash Commands

2 new CFA slash commands in `.claude/commands/cfa/`:

| Command | Routed To | Description |
|---------|-----------|-------------|
| `/cfa property-valuation` | `cfa-equity-analyst` | Comprehensive property valuation combining income, sales comparison, and cost approaches with HBU analysis |
| `/cfa acquisition-model` | `cfa-private-markets-analyst` | Full acquisition underwriting with sources & uses, pro forma, debt sizing, and hold/sell analysis |

This brings the total from 23 (post-ADR-012) to 25 CFA slash commands.

### Skill Updates

Update `.claude/skills/corp-finance-tools-core/SKILL.md` to include the 6 new MCP tools under a new "Institutional Real Estate" section. Update `AGENT_SKILLS` in `packages/agents/src/pipeline.ts`:

| Agent | Added Capabilities |
|-------|-------------------|
| `cfa-equity-analyst` | +institutional_rent_roll, +institutional_comparable_sales, +institutional_hbu_analysis, +institutional_replacement_cost, +institutional_benchmark |
| `cfa-private-markets-analyst` | +institutional_acquisition, +institutional_comparable_sales, +institutional_benchmark |

### Integration with Existing Modules

The `institutional_real_estate` module **extends** the existing codebase without replacing any module:

| Existing Module | Integration |
|----------------|-------------|
| `real_assets/real_estate` | `acquisition_model` calls `real_estate::property_valuation` for baseline DCF/DirectCap. `rent_roll::tenant_schedule` feeds NOI into existing valuation functions. |
| `real_assets/project_finance` | `development_feasibility` reuses `project_finance` draw schedule and IRR logic for construction-period modeling. |
| `lease_accounting/` | `rent_roll` lease structures align with IFRS 16 / ASC 842 lease classification inputs. Mark-to-market outputs feed lease modification analysis. |
| `onshore_structures/` (REIT) | `benchmark::ncreif_attribution` results feed REIT NAV calculations. REIT distribution requirements inform `hold_sell_analysis` tax assumptions. |
| `pe/waterfall` | `value_add_irr` reuses the existing GP/LP waterfall promote structure for PERE fund return modeling. |
| `fixed_income/bonds` | `refinancing` reuses bond math (present value, yield-to-maturity) for defeasance cost calculation. |

### Mathematical Standards

- All monetary and rate calculations use `rust_decimal::Decimal` with the `maths` feature for `powd()` where required
- IRR computation uses Newton-Raphson iteration (consistent with existing `pe` and `project_finance` modules)
- NPV discount factors use iterative multiplication (not `powd()`) to avoid precision drift
- Square root (for volatility in benchmark module) uses Newton's method (20 iterations)
- No `f64` except where explicitly noted (none anticipated in this module)

### Test Targets

| Sub-module | Test Count | Key Test Scenarios |
|-----------|-----------|-------------------|
| rent_roll | ~45 | Single tenant flat/stepped/CPI rent, multi-tenant rollover, renewal probability edge cases (expired lease, month-to-month), WALT by rent vs. NRA, free rent and TI amortisation |
| comparable_sales | ~40 | Positive/negative adjustments, zero-adjustment comp, cumulative net adjustment >25% warning, reconciliation with equal/quality/distance weighting, CoV threshold |
| highest_best_use | ~35 | Zoning constraint filtering, brownfield remediation cost, multiple feasible uses ranked by residual value, single feasible use, no feasible use (vacant land only) |
| replacement_cost | ~40 | Each building class (A/B/C/D/S), physical/functional/external depreciation, land residual extraction, Marshall & Swift local cost modifier, sprinkler/HVAC add-ons |
| benchmark | ~40 | Quarterly chain-linking, leverage adjustment, allocation/selection/interaction attribution, rolling return windows, drawdown calculation |
| acquisition | ~40 | Senior + mezz debt, IO period transition to amortisation, hold/sell break-even cap rate, value-add renovation CapEx, ground-up development with construction draws, cash-out refi |
| **Total** | **~240** | |

This brings the projected test count from 5,841 to approximately 6,081.

## Consequences

### Positive

- Institutional CRE analytics close the most significant remaining gap for pension fund, PERE, and REIT users
- Tenant-level rent roll modeling enables Argus-compatible underwriting workflows
- Four-tests HBU framework and cost approach satisfy USPAP and IVS appraisal standards
- NCREIF/ODCE benchmarking enables institutional performance reporting and GIPS compliance
- Acquisition module provides end-to-end deal lifecycle from screening through hold/sell/refi
- All 6 sub-modules integrate with existing modules (no duplication, no breaking changes)
- ~240 new tests maintain the project's >99% pass-rate standard
- 6 new MCP tools and 12 NAPI bindings follow established patterns (JSON string boundary)

### Negative

- Module adds approximately 3,000-4,000 lines of Rust across 6 sub-modules, increasing compile time by an estimated 5-8 seconds
- Marshall & Swift cost data (base costs per SF by class/occupancy) will be encoded as static const arrays; real-world users may need to update these annually as CoreLogic publishes new cost tables
- NCREIF benchmark comparisons require the user to supply benchmark return data (NCREIF does not provide a public API); the module computes attribution but does not fetch index data
- Comparable sales adjustments are inherently subjective; the quantitative adjustment grid systematises the process but professional judgment remains required for adjustment magnitude selection
- Adding 6 MCP tools increases the tool registry size, contributing to context window pressure for agents that carry the full tool list

## Options Considered

### Option 1: Extend existing `real_assets/real_estate.rs` in-place (Rejected)

- **Pros**: No new module, simpler dependency graph
- **Cons**: Would expand a single file from ~400 lines to ~4,000+ lines, violating the 500-line file limit. The tenant-level and benchmark functionality represents a distinct bounded context from property-level valuation.

### Option 2: Use a third-party CRE library via FFI (Rejected)

- **Pros**: Leverage existing institutional-grade implementations
- **Cons**: No mature open-source Rust CRE analytics library exists. Python libraries (e.g., `pyreal`) lack the precision guarantees of `rust_decimal`. FFI boundary adds latency and complexity incompatible with the MCP tool pattern.

### Option 3: Implement as TypeScript-only in the MCP server layer (Rejected)

- **Pros**: Faster iteration, no Rust compilation
- **Cons**: Violates the project's architecture principle that all financial math lives in Rust with `rust_decimal`. TypeScript `number` (IEEE 754 double) introduces floating-point errors in compound interest and NPV calculations that are unacceptable for institutional underwriting.

### Option 4: Split into two phases -- rent roll + comparable sales first, then the rest (Considered)

- **Pros**: Smaller increments, earlier delivery of highest-value features
- **Cons**: The 6 sub-modules are tightly integrated (acquisition model depends on rent roll; HBU depends on cost approach and comparable sales; benchmark depends on acquisition model outputs). Shipping partial functionality creates incomplete workflows. Single-phase delivery with parallel agent implementation (established pattern from Phases 10-19) is preferred.

## Related Decisions

- ADR-008: Financial Services Workflow Integration (workflow skills for institutional deliverables)
- ADR-009: Workflow Auditability (audit hashing for workflow definitions, applicable to new slash commands)
- ADR-012: Gap Analysis Remediation (skill wiring and slash command patterns)

## References

- [NCREIF Property Index (NPI) Methodology](https://www.ncreif.org/data-products/property/)
- [NCREIF ODCE Index](https://www.ncreif.org/data-products/funds/odce/)
- [Appraisal Institute - The Appraisal of Real Estate, 15th Edition](https://www.appraisalinstitute.org/)
- [USPAP Standards Rule 1-3: Highest and Best Use](https://www.uspap.org/)
- [IVS 104: Bases of Value](https://www.ivsc.org/standards/international-valuation-standards)
- [Marshall & Swift / CoreLogic Commercial Cost Handbook](https://www.corelogic.com/)
- [GIPS Standards for Real Estate (CFA Institute)](https://www.gipsstandards.org/)
- [Argus Enterprise DCF Methodology](https://www.altus.com/argus/)
- [IFRS 16 Leases / ASC 842 Leases](https://www.ifrs.org/issued-standards/list-of-standards/ifrs-16-leases/)
