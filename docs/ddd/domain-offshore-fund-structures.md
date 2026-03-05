# Domain-Driven Design Supplement: Offshore Fund Structures Expansion

## Overview

This supplement defines the **Offshore Fund Structuring Analytics** bounded context, an extension of the existing `offshore_structures` bounded context that broadens jurisdiction coverage from 2 (Cayman, Luxembourg/Ireland) to 10+ and adds cross-cutting multi-jurisdiction comparison and fund migration capabilities. It covers six sub-domains: Channel Islands fund analysis, Singapore VCC analysis, Hong Kong OFC/LPF analysis, Middle East fund structures, multi-jurisdiction comparison, and fund migration/redomiciliation.

The existing `offshore_structures::cayman` and `offshore_structures::luxembourg` modules provide single-jurisdiction analysis functions (`analyze_cayman_structure`, `analyze_lux_structure`) covering structure selection, fee economics, regulatory requirements, and master-feeder analysis. This context extends that foundation with additional jurisdictions and the analytical frameworks required by fund formation teams, tax advisors, and institutional allocators performing domicile selection and migration planning.

---

## Bounded Context: Offshore Fund Structuring Analytics

**Type**: Supporting Domain (extends existing Offshore Structures)

**Responsibility**: Provide institutional-quality offshore fund structuring analysis covering 10+ jurisdictions, including structure selection, regulatory analysis, tax incentive eligibility, substance scoring, multi-jurisdiction comparison with optimal domicile recommendation, and fund migration/redomiciliation feasibility with cost-benefit NPV.

### Relationship to Existing `offshore_structures`

This context does **not** replace the existing `offshore_structures` modules. It extends them:

- **Shared kernel**: `Decimal`, `CorpFinanceResult`, `CorpFinanceError` from `crate::types` and `crate::error`
- **Reuses**: `FeederInfo`, `ServiceProviders`, `CaymanFundInput`, `CaymanFundOutput`, `LuxFundInput`, `LuxFundOutput` as reference types for comparison and migration analysis
- **Reuses**: `substance_requirements::economic_substance` for substance scoring (5-dimension framework)
- **Reuses**: `tax_treaty::treaty_network` for treaty optimization during migration tax assessment
- **Adds**: Four new jurisdiction sub-modules (`channel_islands`, `singapore_vcc`, `hong_kong`, `middle_east`) and two cross-cutting sub-modules (`comparison`, `migration`) under `crate::offshore_structures`

### Anti-Corruption Layer

Jurisdiction-specific regulatory concepts (JFSC registration, MAS licensing tiers, SFC OFC authorization, DFSA notification, Sharia board requirements) must not leak into generic financial primitives or into other jurisdiction modules. The ACL operates at two boundaries:

| Boundary | Direction | Mechanism |
|----------|-----------|-----------|
| Jurisdiction -> Generic Finance | Outbound | Each jurisdiction analysis function returns `CorpFinanceResult<T>` wrapping jurisdiction-specific output structs; consumers see standard result envelopes |
| Jurisdiction -> Comparison/Migration | Outbound | Jurisdiction modules expose a normalized `JurisdictionProfile` value object consumed by the comparison and migration sub-domains; raw regulatory details remain internal to each jurisdiction module |
| External Contexts -> Offshore | Inbound | Offshore modules import substance scoring and treaty analysis via function calls but never expose substance or treaty internals to callers; all cross-context calculations remain behind the ACL |

---

## Aggregates

### 1. OffshoreVehicle (Root Aggregate)

The central aggregate representing an offshore fund vehicle in any supported jurisdiction. All sub-domain analyses produce or reference an OffshoreVehicle.

| Field | Type | Description |
|-------|------|-------------|
| `vehicle_id` | `String` | Unique identifier |
| `fund_name` | `String` | Legal name of the fund |
| `jurisdiction` | `Jurisdiction` | Enum: Cayman, BVI, Luxembourg, Ireland, Jersey, Guernsey, Singapore, HongKong, DIFC, ADGM |
| `structure_type` | `String` | Vehicle type within jurisdiction (e.g., "JPF", "VCC_Umbrella", "OFC", "QIF") |
| `fund_strategy` | `String` | Investment strategy classification |
| `fund_size` | `Decimal` | Total AUM or committed capital |
| `management_fee_rate` | `Decimal` | Annual management fee as decimal |
| `performance_fee_rate` | `Decimal` | Carry/performance fee as decimal |
| `target_investors` | `Vec<String>` | Target investor profiles |
| `regulatory_status` | `RegulatoryStatus` | Registered, Notified, Authorized, Exempt |
| `substance_score` | `Option<SubstanceAssessment>` | 5-dimension substance evaluation |
| `setup_cost` | `Decimal` | Estimated one-time setup cost |
| `annual_cost` | `Decimal` | Estimated annual running cost |
| `distribution_reach` | `Vec<PassportRight>` | Markets accessible from this domicile |

**Invariants enforced by the aggregate root**:
- `fund_size > 0`
- `management_fee_rate >= 0` and `management_fee_rate < 1`
- `performance_fee_rate >= 0` and `performance_fee_rate < 1`
- `jurisdiction` must be a valid enum variant
- `structure_type` must be valid for the given `jurisdiction`

### 2. MultiJurisdictionComparison (Aggregate)

Encapsulates a side-by-side comparison of multiple jurisdictions for a given fund strategy and investor profile.

| Field | Type | Description |
|-------|------|-------------|
| `comparison_id` | `String` | Unique identifier |
| `fund_strategy` | `String` | Strategy being evaluated |
| `fund_size` | `Decimal` | Target AUM |
| `target_investors` | `Vec<String>` | Investor geography/type profile |
| `distribution_targets` | `Vec<String>` | Target distribution markets |
| `lifecycle_years` | `u32` | Fund lifecycle for TCO calculation |
| `jurisdictions` | `Vec<JurisdictionProfile>` | Normalized profiles per jurisdiction |
| `dimension_weights` | `Vec<DimensionWeight>` | User-configurable scoring weights |
| `recommendations` | `Vec<RankedJurisdiction>` | Top-N ranked results |

**Commands**:
- `CompareJurisdictions` -- scores all jurisdictions across dimensions, ranks by composite score
- `CalculateTCO` -- computes total cost of ownership per jurisdiction over fund lifecycle
- `MapDistribution` -- determines passport/NPPR/MoU access per jurisdiction to target markets

**Invariants**:
- At least 2 jurisdictions required for meaningful comparison
- All dimension weights must sum to 1.0 (normalized)
- Lifecycle years must be >= 1
- Fund size must be positive

### 3. MigrationPlan (Aggregate)

Encapsulates the analysis of a fund redomiciliation from one jurisdiction to another.

| Field | Type | Description |
|-------|------|-------------|
| `migration_id` | `String` | Unique identifier |
| `origin_jurisdiction` | `Jurisdiction` | Current domicile |
| `target_jurisdiction` | `Jurisdiction` | Proposed new domicile |
| `fund_size` | `Decimal` | Current AUM |
| `structure_type` | `String` | Current fund structure |
| `remaining_life_years` | `u32` | Remaining fund term |
| `investor_composition` | `Vec<InvestorBlock>` | Investors by tax status/geography |
| `current_annual_cost` | `Decimal` | Current annual running cost |
| `migration_mechanic` | `MigrationMechanic` | Continuation, Domestication, Merger |
| `migration_costs` | `MigrationCosts` | One-time cost breakdown |
| `annual_savings` | `Decimal` | Projected annual cost reduction |
| `tax_impact` | `TaxImpact` | Exit charges, step-up, withholding |
| `timeline_weeks` | `u32` | Estimated completion timeline |
| `feasibility` | `Feasibility` | Feasible, Conditional, NotFeasible |
| `npv` | `Decimal` | Cost-benefit NPV |

**Commands**:
- `AssessFeasibility` -- evaluates regulatory, legal, and practical feasibility of the migration
- `CalculateCostBenefit` -- computes NPV of migration costs vs annual savings over remaining fund life
- `AssessTaxImpact` -- evaluates exit charges, deemed disposal, treaty implications, stamp duty

**Invariants**:
- Origin and target jurisdictions must differ
- Remaining fund life must be >= 1 year
- Fund size must be positive
- Migration mechanic must be valid for the origin-target corridor
- Investor consent threshold (typically 75%) must be achievable given investor composition

### 4. DistributionNetwork (Aggregate)

Maps cross-border distribution rights from a given domicile to target investor markets.

| Field | Type | Description |
|-------|------|-------------|
| `domicile` | `Jurisdiction` | Fund's home jurisdiction |
| `passport_rights` | `Vec<PassportRight>` | Markets with full passport access |
| `nppr_access` | `Vec<NpprAccess>` | Markets accessible via NPPR |
| `mou_bilateral` | `Vec<MouAccess>` | Markets via bilateral MoU |
| `reverse_solicitation` | `Vec<String>` | Markets where reverse solicitation is possible |
| `restricted_markets` | `Vec<String>` | Markets where distribution is not feasible |
| `total_addressable_market` | `u32` | Count of accessible markets |

**Commands**:
- `MapDistributionReach` -- determines all accessible markets from a given domicile
- `CompareDistributionReach` -- compares distribution reach across multiple domiciles

**Invariants**:
- A market cannot appear in more than one access category (passport supersedes NPPR, NPPR supersedes MoU)
- Passport rights are only available for EU/EEA domiciles (Luxembourg, Ireland) under AIFMD

---

## Value Objects

All value objects are immutable once constructed. They carry no identity; equality is determined by field values.

### JerseyFund

| Field | Type | Description |
|-------|------|-------------|
| `structure_type` | `JerseyStructure` | JPF, ExpertFund, ListedFund, QIF |
| `regulator` | `String` | "JFSC" |
| `max_investors` | `Option<u32>` | 50 for JPF, None for Expert/Listed/QIF |
| `minimum_investment` | `Decimal` | Varies by structure type |
| `approval_timeline_days` | `u32` | 2 days (JPF) to 8 weeks (Listed) |
| `annual_regulatory_fee` | `Decimal` | JFSC annual fee |

### GuernseyFund

| Field | Type | Description |
|-------|------|-------------|
| `structure_type` | `GuernseyStructure` | PIF, QIF, RQIF, AuthorisedFund |
| `regulator` | `String` | "GFSC" |
| `max_investors` | `Option<u32>` | 50 for PIF, None for others |
| `minimum_investment` | `Decimal` | Varies by structure type |
| `fast_track` | `bool` | True for RQIF (3 business days) |
| `approval_timeline_days` | `u32` | 3 days (RQIF) to 10 weeks (Authorised) |

### CellCompany

| Field | Type | Description |
|-------|------|-------------|
| `cell_type` | `CellType` | PCC (Protected Cell), ICC (Incorporated Cell) |
| `jurisdiction` | `Jurisdiction` | Jersey, Guernsey, or Cayman (SPC) |
| `num_cells` | `u32` | Number of segregated cells |
| `separate_legal_personality` | `bool` | True for ICC, false for PCC |
| `cell_liability_segregated` | `bool` | Always true |
| `per_cell_cost` | `Decimal` | Incremental cost per additional cell |

### VCC

| Field | Type | Description |
|-------|------|-------------|
| `vcc_type` | `VccType` | Standalone, Umbrella |
| `num_sub_funds` | `u32` | 1 for standalone, N for umbrella |
| `mas_license_type` | `MasLicenseType` | RFMC, LRFMC, ALFMC |
| `tax_incentive` | `Option<TaxIncentive>` | Section 13O, 13U, or 13D |
| `base_capital` | `Decimal` | MAS minimum base capital requirement |
| `sub_fund_segregation` | `bool` | Assets/liabilities segregated per sub-fund |

### TaxIncentive

| Field | Type | Description |
|-------|------|-------------|
| `scheme` | `String` | "Section13O", "Section13U", "Section13D" |
| `eligible` | `bool` | Whether fund meets eligibility criteria |
| `minimum_aum` | `Decimal` | SGD 50M (13O), SGD 200M (13U), None (13D) |
| `minimum_professionals` | `u32` | 3 (13O/13U), 0 (13D) |
| `minimum_local_spend` | `Decimal` | SGD 200k (13O/13U), None (13D) |
| `tax_rate` | `Decimal` | 0% if eligible, 17% otherwise |
| `conditions_met` | `Vec<String>` | List of conditions satisfied |
| `conditions_failed` | `Vec<String>` | List of conditions not met |

### OFC

| Field | Type | Description |
|-------|------|-------------|
| `ofc_type` | `OfcType` | Public, Private |
| `umbrella` | `bool` | Whether umbrella with sub-funds |
| `sfc_authorized` | `bool` | SFC authorization status |
| `unified_fund_exemption` | `bool` | Eligible for UFE profits tax exemption |
| `ofc_grant_eligible` | `bool` | Eligible for 70% cost subsidy up to HKD 1M |

### LPF

| Field | Type | Description |
|-------|------|-------------|
| `strategy` | `String` | PE, VC, RE, Credit |
| `general_partner` | `String` | GP entity description |
| `ufe_eligible` | `bool` | Unified Fund Exemption eligibility |
| `carried_interest_concession` | `bool` | 0% tax on qualifying carried interest |
| `effective_carry_tax_rate` | `Decimal` | 0% if concession applies, 16.5% otherwise |

### DifcFund

| Field | Type | Description |
|-------|------|-------------|
| `structure_type` | `DifcStructure` | QIF, ExemptFund, DomesticFund |
| `regulator` | `String` | "DFSA" |
| `minimum_investment` | `Decimal` | USD 500k (QIF), USD 50k (Exempt), varies (Domestic) |
| `max_investors` | `Option<u32>` | 100 for QIF/Exempt, None for Domestic |
| `sharia_compliant` | `bool` | Whether fund has Sharia board |

### AdgmFund

| Field | Type | Description |
|-------|------|-------------|
| `structure_type` | `AdgmStructure` | ExemptFund, QIF, PublicFund |
| `regulator` | `String` | "FSRA" |
| `minimum_investment` | `Decimal` | Varies by structure |
| `professional_investors_only` | `bool` | True for Exempt, varies for others |

### SubstanceScore

| Field | Type | Description |
|-------|------|-------------|
| `jurisdiction` | `Jurisdiction` | Jurisdiction being assessed |
| `personnel_score` | `Decimal` | 0-100 local personnel adequacy |
| `premises_score` | `Decimal` | 0-100 physical presence |
| `decision_making_score` | `Decimal` | 0-100 local decision authority |
| `expenditure_score` | `Decimal` | 0-100 local spend adequacy |
| `ciga_score` | `Decimal` | 0-100 core income-generating activities |
| `composite_score` | `Decimal` | Weighted composite 0-100 |
| `risk_level` | `String` | "Low", "Medium", "High" |

### MigrationRoute

| Field | Type | Description |
|-------|------|-------------|
| `origin` | `Jurisdiction` | Source jurisdiction |
| `target` | `Jurisdiction` | Destination jurisdiction |
| `mechanic` | `MigrationMechanic` | Continuation, Domestication, Merger |
| `feasibility_score` | `Decimal` | 0-100 regulatory/practical feasibility |
| `common_corridor` | `bool` | Whether this is a well-established route |
| `estimated_weeks` | `u32` | Typical completion timeline |

### PassportRight

| Field | Type | Description |
|-------|------|-------------|
| `target_market` | `String` | Country/region accessible |
| `access_type` | `AccessType` | Passport, NPPR, MoU, ReverseSolicitation |
| `restrictions` | `Vec<String>` | Any conditions or limitations |
| `investor_types` | `Vec<String>` | Eligible investor categories |

### InvestorBlock

| Field | Type | Description |
|-------|------|-------------|
| `investor_type` | `String` | "USTaxExempt", "USTaxable", "EU_Institutional", etc. |
| `allocation_pct` | `Decimal` | Percentage of total fund |
| `tax_status` | `String` | Tax classification |
| `consent_expected` | `bool` | Whether consent for migration is expected |

### MigrationCosts

| Field | Type | Description |
|-------|------|-------------|
| `legal_fees` | `Decimal` | Legal counsel in both jurisdictions |
| `regulatory_fees` | `Decimal` | Filing and registration fees |
| `tax_advisory` | `Decimal` | Tax structuring and opinions |
| `investor_communication` | `Decimal` | Investor relations and consent |
| `operational_transition` | `Decimal` | Admin, audit, service provider transition |
| `total` | `Decimal` | Sum of all one-time costs |

### TaxImpact

| Field | Type | Description |
|-------|------|-------------|
| `exit_charge` | `Decimal` | Exit tax in origin jurisdiction |
| `basis_treatment` | `String` | "StepUp", "CarryOver", "DeemedDisposal" |
| `withholding_tax` | `Decimal` | WHT on deemed distribution |
| `stamp_duty` | `Decimal` | Transfer/stamp duty if applicable |
| `treaty_benefit` | `Decimal` | Treaty reduction if available |
| `net_tax_cost` | `Decimal` | Total tax cost of migration |

### DimensionWeight

| Field | Type | Description |
|-------|------|-------------|
| `dimension` | `String` | Comparison dimension name |
| `weight` | `Decimal` | Weight (0 to 1), all must sum to 1.0 |

### RankedJurisdiction

| Field | Type | Description |
|-------|------|-------------|
| `jurisdiction` | `Jurisdiction` | Jurisdiction |
| `composite_score` | `Decimal` | Weighted composite 0-100 |
| `rank` | `u32` | Position in ranking |
| `dimension_scores` | `Vec<(String, Decimal)>` | Score per dimension |
| `rationale` | `String` | Recommendation narrative |

---

## Domain Events

Events are raised by aggregate commands and consumed by downstream contexts or stored in the event log.

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `FundStructureAnalyzed` | OffshoreVehicle | `vehicle_id`, `jurisdiction`, `structure_type`, `fund_size`, `regulatory_status`, `substance_score`, `setup_cost`, `annual_cost`, `timestamp` | Financial Memory, Analysis Orchestration |
| `JurisdictionCompared` | MultiJurisdictionComparison | `comparison_id`, `jurisdiction_count`, `top_recommendation`, `composite_score`, `tco_range`, `timestamp` | Financial Memory, Analysis Orchestration |
| `MigrationFeasibilityAssessed` | MigrationPlan | `migration_id`, `origin`, `target`, `feasibility`, `npv`, `timeline_weeks`, `migration_mechanic`, `timestamp` | Financial Memory, Analysis Orchestration |
| `DistributionReachMapped` | DistributionNetwork | `domicile`, `passport_count`, `nppr_count`, `total_markets`, `timestamp` | MultiJurisdictionComparison (informs scoring) |
| `SubstanceRiskFlagged` | OffshoreVehicle | `vehicle_id`, `jurisdiction`, `composite_score`, `risk_level`, `failing_dimensions`, `timestamp` | Substance Requirements (compliance alert), Analysis Orchestration |

---

## Invariants

These invariants are enforced at the aggregate boundary. Violations result in `CorpFinanceError::InvalidInput`.

| ID | Invariant | Aggregate | Rationale |
|----|-----------|-----------|-----------|
| OS-INV-001 | Fund size must be positive | OffshoreVehicle, MultiJurisdictionComparison, MigrationPlan | A zero or negative fund size is economically meaningless |
| OS-INV-002 | Structure type must be valid for jurisdiction | OffshoreVehicle | Each jurisdiction has a defined set of vehicle types; e.g., JPF is only valid for Jersey, VCC only for Singapore |
| OS-INV-003 | Minimum investment thresholds must be met per structure type | OffshoreVehicle | Regulatory requirement; e.g., DIFC QIF requires USD 500k minimum, Singapore 13O requires SGD 50M AUM |
| OS-INV-004 | Investor count must not exceed structure limits | OffshoreVehicle | JPF/PIF capped at 50 investors; DIFC QIF/Exempt capped at 100 |
| OS-INV-005 | Substance score dimensions must each be 0-100 | OffshoreVehicle | Scoring boundary; composite is weighted average of bounded inputs |
| OS-INV-006 | Comparison dimension weights must sum to 1.0 | MultiJurisdictionComparison | Normalization constraint; non-normalized weights produce misleading composite scores |
| OS-INV-007 | At least 2 jurisdictions for comparison | MultiJurisdictionComparison | Single-jurisdiction comparison is degenerate |
| OS-INV-008 | Origin and target jurisdictions must differ for migration | MigrationPlan | Self-migration is not meaningful |
| OS-INV-009 | Migration mechanic must be valid for the corridor | MigrationPlan | Not all mechanics are available for all origin-target pairs; statutory continuation requires target jurisdiction recognition |
| OS-INV-010 | Passport rights only for EU/EEA domiciles under AIFMD | DistributionNetwork | Non-EU jurisdictions cannot claim AIFMD marketing passport; they must use NPPR or bilateral arrangements |

---

## Context Map

```
+-----------------------------------------------------------------------+
|                  OFFSHORE FUND STRUCTURING ANALYTICS                    |
|                                                                        |
|  +------------------+     +------------------+     +----------------+  |
|  | Channel Islands  |     | Singapore VCC    |     | Hong Kong      |  |
|  | (Jersey/Guernsey)|     | (13O/13U/13D)    |     | (OFC/LPF)     |  |
|  +--------+---------+     +--------+---------+     +-------+--------+  |
|           |                        |                        |          |
|           v                        v                        v          |
|  +--------+------------------------+------------------------+-------+  |
|  |                    OffshoreVehicle (root)                        |  |
|  |              (normalized JurisdictionProfile)                    |  |
|  +--------+------------------------+------------------------+-------+  |
|           ^                        ^                        ^          |
|           |                        |                        |          |
|  +--------+---------+     +--------+---------+     +-------+--------+  |
|  | Middle East      |     | Multi-Juris      |     | Migration      |  |
|  | (DIFC/ADGM)      |     | Comparison       |     | Plan           |  |
|  +------------------+     +--------+---------+     +-------+--------+  |
|                                    |                        |          |
|                                    v                        v          |
|                           +--------+---------+     +-------+--------+  |
|                           | Distribution     |     | Tax Impact     |  |
|                           | Network          |     | Assessment     |  |
|                           +------------------+     +----------------+  |
|                                                                        |
+---+----------+----------+----------+----------+----------+-------------+
    |          |          |          |          |          |
    v          v          v          v          v          v
 Cayman/Lux  Substance  Tax Treaty  FATCA/CRS  AML       Regulatory
 (existing)  Reqmts     (treaty     (reporting  Compliance Reporting
 (extends)   (scoring)  network)    classif.)  (KYC)     (AIFMD/SEC)
```

### Context Relationships

| Upstream | Downstream | Relationship | Integration Detail |
|----------|------------|-------------|-------------------|
| **Existing Offshore Structures** | Offshore Expansion | Shared Kernel (extends) | New jurisdiction modules import shared types (`FeederInfo`, `ServiceProviders`) and follow the same input/output pattern (`CorpFinanceResult<T>`). Comparison module consumes outputs from both existing and new jurisdiction analyzers. |
| **Substance Requirements** | Offshore Expansion | Customer/Supplier | Offshore modules call `economic_substance::assess_substance()` for 5-dimension substance scoring. Offshore is the customer; substance module supplies the scoring framework. The 9-jurisdiction coverage in substance aligns with offshore jurisdictions. |
| **Tax Treaty** | Offshore Expansion | Customer/Supplier | Migration module calls `treaty_network::optimize_treaty_route()` for WHT analysis during redomiciliation tax assessment. Treaty module supplies conduit routing and LOB/PPT analysis. |
| **FATCA/CRS** | Offshore Expansion | Published Language | Fund structure analysis outputs inform FATCA entity classification (FFI/NFFE) and CRS entity type (Investment Entity/Financial Institution). ACL translates offshore `OffshoreVehicle` into FATCA/CRS classification inputs. |
| **AML Compliance** | Offshore Expansion | Published Language | Jurisdiction risk scores from offshore analysis feed into AML geographic risk dimension. `SubstanceRiskFlagged` events are consumed by AML for enhanced due diligence triggers. |
| **Onshore Structures** | Offshore Expansion | Partnership | Bidirectional: onshore US fund structures (Delaware LP, BDC) may feed into offshore master-feeder analysis; offshore vehicles may be feeder entities into onshore masters. Comparison module evaluates onshore-offshore hybrid structures. |
| **Regulatory Reporting** | Offshore Expansion | Customer/Supplier | AIFMD Annex IV reporting requirements inform jurisdiction analysis (which domiciles trigger which reporting obligations). Offshore modules consume AIFMD threshold data. |
| **Offshore Expansion** | Financial Memory | Publisher/Subscriber | All domain events (`FundStructureAnalyzed`, `JurisdictionCompared`, `MigrationFeasibilityAssessed`, `DistributionReachMapped`, `SubstanceRiskFlagged`) published to Financial Memory for retrieval-augmented analysis. |
| **Offshore Expansion** | Hosted MCP Gateway | Conformist | Offshore tools conform to MCP tool registration schema. Each sub-domain exposes tools via NAPI bindings following the `JSON string -> JSON string` boundary pattern. |

---

## Sub-Domain Breakdown

### 1. Channel Islands (`channel_islands`)

**Purpose**: Fund structuring analysis for Jersey (JFSC) and Guernsey (GFSC) regulated and unregulated vehicles, including cell company structures.

**Key Computations**:
- Structure selection scoring per investor profile and strategy fit
- Regulatory timeline estimation (2 days JPF to 10 weeks Authorised)
- Cost modeling: setup (legal, regulatory filing, incorporation) + annual running costs (audit, admin, directors, regulatory fee)
- Cell company economics: per-cell incremental cost, liability segregation analysis, PCC vs ICC vs SPC comparison
- Post-Brexit EU distribution access: NPPR availability by EU member state, reverse solicitation risk scoring
- Substance scoring integration: 5-dimension assessment via `substance_requirements` module

### 2. Singapore VCC (`singapore_vcc`)

**Purpose**: Variable Capital Company analysis with MAS licensing, tax incentive eligibility, and sub-fund structuring.

**Key Computations**:
- VCC umbrella sub-fund allocation: expense apportionment, fee splitting, cross-liability segregation
- MAS license tier assessment: AUM threshold check, investor qualification, capital adequacy
- Tax incentive eligibility: Section 13O (SGD 50M AUM, 3 professionals, SGD 200k spend), 13U (SGD 200M AUM, enhanced conditions), 13D (offshore, no minimum)
- VCC vs Cayman SPC comparison matrix: regulatory burden, tax efficiency, redomiciliation ease, distribution reach
- Total cost of ownership: setup + annual running + substance/staffing + tax leakage over fund lifecycle

### 3. Hong Kong OFC & LPF (`hong_kong`)

**Purpose**: Open-Ended Fund Company and Limited Partnership Fund analysis with Unified Fund Exemption and carried interest concession.

**Key Computations**:
- OFC structure analysis: public vs private, umbrella with sub-funds, SFC authorization requirements
- LPF structure analysis: GP requirements, audit obligations, strategy-specific considerations
- Unified Fund Exemption eligibility: qualifying fund test, qualifying transactions test, qualifying persons test
- Carried interest concession: 0% vs 16.5% effective tax rate, qualifying conditions (holding period >= 2 years, committed capital >= HKD 2M, return hurdle), clawback treatment
- OFC grant calculation: 70% of eligible expenses capped at HKD 1M
- Comparison vs Cayman/Singapore: regulatory, tax, cost, distribution reach

### 4. Middle East (`middle_east`)

**Purpose**: DIFC and ADGM free zone fund structuring with Sharia-compliant options.

**Key Computations**:
- DIFC fund structure selection: QIF (5-day notification), Exempt (DFSA registration), Domestic (full authorization)
- ADGM fund structure selection: Exempt, QIF, Public fund regulatory pathways
- Free zone tax economics: 0% corporate/personal/withholding, 50-year guarantee, UAE federal 9% interaction
- Sharia compliance scoring: Mudarabah/Musharakah/Wakalah structure assessment, AAOIFI standards checklist, Sharia board cost and governance
- Cost modeling: free zone license, regulatory fees, registered office, local directors, Sharia board fees
- GCC distribution reach and investor qualification

### 5. Multi-Jurisdiction Comparison (`comparison`)

**Purpose**: Quantitative multi-jurisdiction comparison with optimal domicile recommendation and distribution mapping.

**Key Computations**:
- Per-jurisdiction scoring across 10 dimensions (setup cost, running cost, timeline, minimum capital, tax, substance, distribution, legal maturity, service ecosystem, redomiciliation flexibility)
- Weighted composite scoring with user-configurable weights (default: cost 25%, tax 20%, distribution 20%, substance 15%, timeline 10%, flexibility 10%)
- Total cost of ownership: sum of discounted setup + annual costs over fund lifecycle per jurisdiction
- Distribution passport mapping: EU AIFMD passport (Lux/Ireland), NPPR per EU state, bilateral MoU, reverse solicitation
- Optimal domicile recommendation: top-3 with composite score, rationale, and trade-off analysis

### 6. Fund Migration (`migration`)

**Purpose**: Fund redomiciliation feasibility, cost-benefit NPV, and tax impact analysis.

**Key Computations**:
- Corridor feasibility scoring: statutory continuation availability, regulatory recognition, precedent transactions
- Migration cost estimation: legal (both jurisdictions), regulatory filing, tax advisory, investor communication, operational transition
- Annual savings projection: running cost differential, tax savings, distribution access value
- Cost-benefit NPV: one-time costs vs discounted annual savings over remaining fund life, using fund cost of capital as discount rate
- Tax impact: exit charges (origin), deemed disposal, WHT on transition, treaty benefits, stamp duty
- Timeline estimation: regulatory milestones, investor consent process (75% threshold), service provider transition
- Investor consent analysis: investor composition review, consent likelihood by block, side pocket treatment for illiquid assets

---

## MCP Tool Mapping

Each sub-domain maps to one or more MCP tools following the existing naming convention.

| Sub-Domain | Tool Name | Description |
|------------|-----------|-------------|
| channel_islands | `analyze_channel_islands_fund` | Jersey/Guernsey structure analysis with cell company modeling |
| singapore_vcc | `analyze_singapore_vcc` | VCC analysis with MAS licensing and 13O/13U/13D tax incentive |
| hong_kong | `analyze_hong_kong_fund` | OFC/LPF analysis with UFE and carried interest concession |
| middle_east | `analyze_middle_east_fund` | DIFC/ADGM analysis with free zone economics and Sharia compliance |
| comparison | `compare_offshore_jurisdictions` | Multi-jurisdiction comparison with optimal recommendation |
| comparison | `map_distribution_reach` | Distribution passport/NPPR mapping from any domicile |
| migration | `assess_fund_migration` | Migration feasibility, cost-benefit NPV, and tax impact |
| migration | `compare_migration_corridors` | Side-by-side corridor comparison for a given origin |

---

## Domain Model Impact Summary

| Bounded Context | Change Type | Impact |
|----------------|-------------|--------|
| Offshore Structures | Extension (6 new sub-modules) | High -- significant new aggregate and value object surface area across 4 new jurisdictions + 2 cross-cutting frameworks |
| Substance Requirements | Existing supplier | Low -- offshore modules call existing substance scoring functions; no changes needed |
| Tax Treaty | Existing supplier | Low -- migration module calls existing treaty optimization functions; no changes needed |
| FATCA/CRS | New event consumer | Low -- receives `FundStructureAnalyzed` events via ACL for entity classification |
| AML Compliance | New event consumer | Low -- receives `SubstanceRiskFlagged` events for EDD trigger; no schema changes |
| Onshore Structures | Partnership integration | Low -- bidirectional data flow for master-feeder hybrid structures |
| Regulatory Reporting | Existing supplier | None -- offshore consumes existing AIFMD threshold data |
| Hosted MCP Gateway | 8 new tools registered | Medium -- new NAPI bindings and MCP tool registrations |
| Analysis Orchestration | New routing targets | Low -- SemanticRouter gains offshore intent classifications |
| Financial Memory | New event types stored | Low -- standard event storage, no schema changes |
