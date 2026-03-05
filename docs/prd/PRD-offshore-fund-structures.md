# Product Requirements Document: Offshore Fund Structures Expansion

**Product**: Autonomous CFA Analyst Platform
**Package**: @robotixai/corp-finance-mcp
**Version**: 1.0
**Date**: 2026-03-05
**Author**: RobotixAI Engineering

---

## 1. Overview

The existing `offshore_structures` module provides analysis for two jurisdictions: Cayman Islands (Exempted LP, SPC, Unit Trust, LLC, BVI BCA, BVI LP) via `cayman.rs` and Luxembourg/Ireland (SICAV-SIF, SICAV-RAIF, SCSp, ICAV, QIAIF, Section 110) via `luxembourg.rs`. Each module offers a single analysis function (`analyze_cayman_structure`, `analyze_lux_structure`) covering structure selection, fee economics, regulatory requirements, and master-feeder analysis.

Institutional fund managers, however, increasingly deploy capital through a broader set of offshore jurisdictions -- Channel Islands (Jersey, Guernsey), Singapore (VCC), Hong Kong (OFC, LPF), and Middle Eastern free zones (DIFC, ADGM). They also require multi-jurisdiction comparison tooling for optimal domicile selection and fund migration/redomiciliation analysis when regulatory or commercial circumstances change.

This PRD specifies six sub-modules that extend the `offshore_structures` bounded context from 2 jurisdictions to 10+, adding structured comparison frameworks and migration planning capabilities suitable for fund formation teams, tax advisors, and institutional allocator due diligence.

---

## 2. Requirements

### OS-001: Channel Islands Fund Analysis

**Priority**: P0 (Critical)
**Description**: Comprehensive fund structuring analysis for Jersey and Guernsey regulated and unregulated vehicles, including cell company structures and post-Brexit EU market access implications.
**Acceptance Criteria**:
- Accepts fund parameters (name, structure type, strategy, AUM, fee structure, target investor base, distribution targets)
- Supports Jersey structures: Jersey Private Fund (JPF) under 50-investor limit, Expert Fund (requires professional investor minimum EUR 100k), Listed Fund (Channel Islands Securities Exchange), Qualifying Investor Fund (QIF, institutional only)
- Supports Guernsey structures: Private Investment Fund (PIF) under 50-investor limit, Qualifying Investor Fund (QIF, EUR 100k minimum), Registered Qualifying Investor Fund (RQIF, fast-track 3 business days), Authorised Fund (retail-capable, GFSC full authorization)
- Models cell company structures: Protected Cell Company (PCC) with legally segregated cells, Incorporated Cell Company (ICC) with separate legal personality per cell
- Calculates setup costs, annual running costs (audit, admin, regulatory fees, directors), and total cost of ownership over fund lifecycle
- Evaluates post-Brexit EU third-country equivalence: AIFMD marketing passport availability (currently unavailable), National Private Placement Regime (NPPR) access by EU member state, reverse solicitation analysis
- Computes substance scoring using 5-dimension framework (personnel, premises, decision-making, expenditure, CIGA) consistent with existing `substance_requirements` module
- Returns structure recommendation with regulatory timeline, cost comparison, distribution reach map, and substance risk assessment

### OS-002: Singapore VCC Analysis

**Priority**: P0 (Critical)
**Description**: Variable Capital Company analysis covering standalone and umbrella configurations, MAS licensing tiers, and Singapore tax incentive eligibility.
**Acceptance Criteria**:
- Accepts fund parameters (name, structure type, AUM, strategy, sub-fund count, target investors, MAS license type, tax incentive scheme)
- Models VCC standalone (single fund entity) and VCC umbrella with up to N sub-funds (segregated assets and liabilities per sub-fund)
- Evaluates MAS licensing tiers: Registered Fund Management Company (RFMC, AUM <= SGD 250M, max 30 qualified investors), Licensed Fund Management Company with reduced capital (LRFMC, AUM <= SGD 250M, retail permitted), Accredited/Licensed FMC (A-LFMC, no AUM limit)
- Calculates tax incentive eligibility for Section 13O (Singapore-resident fund, AUM >= SGD 50M, minimum 3 investment professionals), Section 13U (enhanced tier, AUM >= SGD 200M, minimum spending SGD 200k), and Section 13D (offshore fund, non-resident, no minimum AUM)
- Computes sub-fund allocation economics: management fee apportionment, expense sharing, cross-sub-fund liability segregation analysis
- Produces VCC vs Cayman SPC side-by-side comparison (regulatory, tax, cost, redomiciliation ease, distribution reach)
- Returns licensing analysis, tax incentive assessment, sub-fund structure recommendation, and cost projection over fund lifecycle

### OS-003: Hong Kong OFC & LPF Analysis

**Priority**: P1 (High)
**Description**: Open-Ended Fund Company and Limited Partnership Fund analysis under Hong Kong's fund-friendly regime, including unified fund exemption and carried interest concessions.
**Acceptance Criteria**:
- Accepts fund parameters (name, structure type, AUM, strategy, SFC licensing status, target investors, carried interest arrangements)
- Models Open-Ended Fund Company (OFC) under Cap 571AA: public and private OFC, umbrella with sub-funds, SFC authorization requirements, investment scope restrictions
- Models Limited Partnership Fund (LPF) under Cap 637: PE/VC/RE strategies, no SFC licensing for LPF itself, general partner requirements, annual audit obligation
- Evaluates Unified Fund Exemption (UFE): profits tax exemption for qualifying funds meeting specified conditions (qualifying fund, qualifying transactions, carried out by qualifying persons)
- Calculates carried interest concession: 0% profits tax on eligible carried interest (vs standard 16.5%), qualifying conditions (holding period test, committed capital test, return hurdle test), clawback implications
- Computes OFC grant scheme: SFC subsidy of 70% of eligible expenses up to HKD 1M per OFC (establishment costs, legal, audit, tax advisory)
- Returns structure recommendation, tax exemption eligibility assessment, carried interest analysis with effective tax rate, and cost comparison vs Cayman/Singapore alternatives

### OS-004: Middle East Fund Structures

**Priority**: P1 (High)
**Description**: Fund structuring analysis for DIFC (Dubai) and ADGM (Abu Dhabi) free zone fund vehicles, including zero-tax economics and Sharia-compliant structure options.
**Acceptance Criteria**:
- Accepts fund parameters (name, structure type, AUM, strategy, free zone, target investors, Sharia compliance requirement)
- Models DIFC fund structures under DFSA regulation: Qualified Investor Fund (QIF, minimum USD 500k, max 100 investors, 5-day DFSA notification), Exempt Fund (minimum USD 50k, max 100 investors, DFSA registration), Domestic Fund (retail-eligible, full DFSA authorization)
- Models ADGM fund structures under FSRA regulation: Exempt Fund (professional investors only, notification-based), Qualified Investor Fund (USD 500k minimum), Public Fund (retail-eligible, FSRA full authorization)
- Calculates free zone tax economics: 0% corporate tax, 0% personal income tax, 0% withholding tax on distributions, 50-year tax holiday guarantees, UAE federal corporate tax (9%) interaction for non-free-zone income
- Evaluates Sharia-compliant fund structures: Mudarabah (profit-sharing), Musharakah (partnership), Wakalah (agency), fund-level Sharia board requirements, AAOIFI compliance scoring
- Computes setup and running costs including free zone license fees, DFSA/FSRA regulatory fees, registered office costs, local director requirements
- Returns regulatory analysis, free zone economics projection, Sharia compliance assessment (if applicable), and comparison vs other offshore jurisdictions

### OS-005: Multi-Jurisdiction Comparison

**Priority**: P0 (Critical)
**Description**: Side-by-side quantitative comparison of 10+ offshore fund jurisdictions with optimal domicile recommendation engine and total cost of ownership modeling.
**Acceptance Criteria**:
- Accepts comparison parameters (fund strategy, AUM, target investors by geography, distribution targets, Sharia requirement, preferred language/legal system, cost sensitivity)
- Compares minimum 10 jurisdictions: Cayman, BVI, Luxembourg, Ireland, Jersey, Guernsey, Singapore, Hong Kong, DIFC, ADGM (extensible to others)
- Comparison dimensions (scored 0-100 per dimension): setup cost, annual running cost, regulatory approval timeline (days), minimum capital requirement, tax treatment (fund-level, investor-level, withholding), substance requirements difficulty, distribution reach (passport/NPPR access), legal framework maturity, service provider ecosystem depth, redomiciliation flexibility
- Calculates total cost of ownership (TCO) over configurable fund lifecycle (default 10 years) including: setup costs, annual admin/audit/legal, regulatory fees, substance costs (local staff, office, directors), tax leakage
- Produces optimal jurisdiction recommendation via weighted scoring: user-configurable dimension weights with sensible defaults, top-3 recommendation with rationale
- Generates distribution passport mapping: which target investor jurisdictions can be accessed from each domicile (EU AIFMD passport, NPPR access, bilateral MoU, reverse solicitation)
- Returns comparison matrix, TCO projection table, ranked recommendation with composite score, and distribution heat map

### OS-006: Fund Migration & Redomiciliation

**Priority**: P1 (High)
**Description**: Fund redomiciliation feasibility analysis covering migration mechanics, common corridors, regulatory approval requirements, tax consequences, and cost-benefit NPV framework.
**Acceptance Criteria**:
- Accepts migration parameters (current domicile, target domicile, fund AUM, fund structure type, remaining fund life, investor composition by tax status, current annual costs)
- Models three migration mechanics: statutory continuation (fund re-registers in new jurisdiction with same legal entity), domestication (recognized by target jurisdiction as local entity), merger/scheme of arrangement (new entity in target jurisdiction absorbs old entity assets)
- Evaluates common corridors with regulatory feasibility scoring: Cayman to Luxembourg (AIFMD passport), BVI to Cayman (upgrade regulatory profile), Ireland to Luxembourg (UCITS/AIFMD optimization), Cayman to Singapore (Asia-Pacific presence), Cayman to Hong Kong (China market access), Jersey/Guernsey to Luxembourg (post-Brexit EU access)
- Calculates regulatory approval timelines per corridor: estimated weeks to complete, key regulatory milestones, investor consent requirements (typically 75% threshold), side pocket treatment for illiquid assets
- Assesses tax consequences: exit charges in origin jurisdiction, step-up or carry-over of cost basis, withholding tax on deemed disposal, treaty implications during transition, stamp duty or transfer taxes
- Computes cost-benefit NPV: one-time migration costs (legal, regulatory, tax advisory, investor communication) vs annual savings from lower running costs or improved distribution access, discounted at fund's cost of capital over remaining fund life
- Returns migration feasibility assessment (feasible/conditional/not feasible), cost-benefit NPV, timeline estimate, tax impact summary, and recommended migration mechanic

---

## 3. Technical Constraints

- All financial math in `rust_decimal::Decimal` (no f64 except where explicitly noted)
- Target ~300 new unit tests (~50 per sub-module) in `crates/corp-finance-core/src/offshore_structures/`
- 6 new MCP tools registered in `packages/mcp-server/` (one per sub-module)
- ~12 new NAPI bindings in `crates/corp-finance-bindings/`
- 2 new slash commands in `.claude/commands/cfa/`: `offshore-comparison.md`, `fund-migration.md`
- Must integrate with existing `offshore_structures` module; extend `mod.rs` with 4 new sub-modules (`channel_islands`, `singapore_vcc`, `hong_kong`, `middle_east`) plus 2 cross-cutting modules (`comparison`, `migration`)
- Reuse shared types from existing `cayman.rs` and `luxembourg.rs` (e.g., `FeederInfo`, `ServiceProviders`) where applicable
- Leverage existing `substance_requirements` module for substance scoring (do not duplicate logic)
- Leverage existing `tax_treaty` module for treaty network analysis during migration tax assessment
- Feature flag: `offshore_structures` (existing feature, no new feature flag needed)

---

## 4. Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| Offshore jurisdiction coverage | 2 (Cayman, Lux/Ireland) | 10+ (+ Jersey, Guernsey, Singapore, HK, DIFC, ADGM, BVI) |
| Offshore fund MCP tools | 2 | 8 |
| Offshore fund unit tests | ~140 | ~440 |
| NAPI bindings (offshore) | ~4 | ~16 |
| CFA slash commands (offshore) | 0 | 2 |
| Multi-jurisdiction comparison | Not supported | 10+ jurisdictions, weighted scoring, TCO |
| Fund migration analysis | Not supported | 6 corridors, cost-benefit NPV, tax impact |
| Cell company modeling | Not supported | PCC/ICC (Jersey/Guernsey), SPC extension |
| Singapore VCC analysis | Not supported | Standalone/umbrella, 13O/13U/13D tax incentive |
| Hong Kong OFC/LPF analysis | Not supported | UFE, carried interest concession, OFC grant |
| Middle East fund structures | Not supported | DIFC/ADGM, Sharia compliance, free zone economics |
| Distribution passport mapping | Not supported | EU AIFMD, NPPR, bilateral MoU coverage |

---

## 5. Out of Scope

- Fund administration operations (NAV calculation, investor accounting, transfer agency)
- Investor onboarding and subscription/redemption processing
- Prime brokerage relationship management and margin analysis
- Real-time regulatory filing generation (AIFMD Annex IV, Form PF handled by existing `regulatory_reporting` module)
- AML/KYC due diligence workflows (handled by existing `aml_compliance` module)
- FATCA/CRS classification and reporting (handled by existing `fatca_crs` module)
- Fund accounting and audit preparation
- Legal document drafting (PPM, LPA, subscription agreements)
- Onshore fund structures (US, UK, EU domestic vehicles handled by `onshore_structures` module)
