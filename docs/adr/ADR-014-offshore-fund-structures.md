# ADR-014: Advanced Offshore Fund Structures & Multi-Jurisdiction Analytics

## Status: Proposed

## Date: 2026-03-05

## Context

The CFA agent platform includes an `offshore_structures` module (Phase 13) with two sub-modules: `cayman.rs` covering Cayman Exempted LP, SPC, Unit Trust, LLC, and BVI structures with master-feeder economics and CIMA registration analysis (`analyze_cayman_structure`), and `luxembourg.rs` covering SICAV-SIF, SICAV-RAIF, SCSp, ICAV, QIAIF, and Section 110 structures with AIFMD passport economics, subscription tax analysis, and treaty benefits (`analyze_lux_structure`). Adjacent modules provide economic substance scoring (`substance_requirements/economic_substance` and `substance_requirements/jurisdiction_tests`), treaty network optimization and WHT routing (`tax_treaty/treaty_network` and `tax_treaty/optimization`), FATCA/CRS entity classification and reporting (`fatca_crs/classification` and `fatca_crs/reporting`), and AML/KYC risk scoring with sanctions screening (`aml_compliance/kyc_scoring` and `aml_compliance/sanctions_screening`).

However, the platform lacks coverage of several major offshore fund domiciles and cross-jurisdictional analytics that institutional fund managers, prime brokers, and fund administrators require:

| Gap | Why It Matters |
|-----|---------------|
| No Channel Islands (Jersey/Guernsey) fund structures | Jersey and Guernsey are top-5 global fund domiciles; Jersey Private Funds (JPFs) have attracted over $500B AUM. Post-Brexit, Channel Islands funds require distinct regulatory analysis separate from EU/Cayman frameworks. JFSC and GFSC regulate under independent regimes with Protected Cell Company (PCC) and Incorporated Cell Company (ICC) structures unavailable elsewhere. |
| No Singapore VCC structures | The Variable Capital Company (VCC) framework (enacted 2018, operational 2020) is the fastest-growing Asian fund domicile. MAS licensing, S13O/S13U/S13D tax incentive schemes, and umbrella sub-fund ring-fencing are distinct from all existing structures. Singapore is the primary competitor to Cayman for Asia-Pacific fund managers. |
| No Hong Kong OFC/LPF structures | Hong Kong's Open-Ended Fund Company (OFC) Ordinance and Limited Partnership Fund (LPF) Ordinance (2020) provide 0% carried interest concession and profits tax exemption. These are the primary structures for Greater China and APAC-focused PE/VC funds. |
| No Middle East free zone fund structures | DIFC and ADGM are the two leading Middle Eastern fund domiciles with 0% corporate tax, independent common-law legal systems, and DFSA/FSRA regulation. Sharia-compliant fund structuring is absent from the platform entirely. Sovereign wealth funds and family offices in the GCC increasingly require local fund analytics. |
| No multi-jurisdiction comparison engine | Fund managers selecting a domicile must compare 10+ jurisdictions on cost, regulation, tax, substance requirements, distribution reach, and timeline. This comparison is currently manual and ad-hoc. No systematic total-cost-of-ownership or distribution passport analysis exists. |
| No fund migration/redomiciliation analytics | Post-Brexit migration from EU to non-EU domiciles (and vice versa), Cayman-to-Singapore redomiciliation, and BVI-to-Jersey continuations are increasingly common. Migration feasibility, cost-benefit analysis, regulatory approval timelines, and tax consequence modeling are absent. |

The existing `cayman.rs` and `luxembourg.rs` modules provide the correct architectural pattern (single input struct, comprehensive output with cost breakdown and regulatory analysis) but cover only 2 of the 8+ major global fund domiciles. Institutional fund managers require pan-jurisdictional coverage with systematic comparison and migration capabilities.

## Decision

Add 6 new sub-modules under the existing `crates/corp-finance-core/src/offshore_structures/` module as Phase 23, containing approximately 24 public functions. The new sub-modules extend (do not replace) the existing `cayman.rs` and `luxembourg.rs` modules and integrate with `substance_requirements`, `tax_treaty`, `fatca_crs`, `aml_compliance`, `onshore_structures`, and `regulatory_reporting` where appropriate.

### Module Structure

```
crates/corp-finance-core/src/offshore_structures/
  mod.rs                     (existing -- add 6 new pub mod declarations)
  cayman.rs                  (existing -- unchanged)
  luxembourg.rs              (existing -- unchanged)
  channel_islands.rs         (new)
  singapore_vcc.rs           (new)
  hong_kong_funds.rs         (new)
  middle_east_funds.rs       (new)
  jurisdiction_comparison.rs (new)
  fund_migration.rs          (new)
```

Feature flag: `offshore_structures` (already exists; new sub-modules are gated under the same feature flag, no new feature required).

### Sub-module Specifications

#### 1. channel_islands -- Jersey & Guernsey Fund Structures

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `analyze_jersey_fund` | `(JerseyFundInput) -> JerseyFundOutput` | Comprehensive analysis of Jersey fund structures: Jersey Private Fund (JPF), Expert Fund, Listed Fund, and Qualifying Investor Fund (QIF). Covers JFSC regulatory classification, minimum investor thresholds, AIF designation for EU marketing under national private placement regimes (NPPRs), annual fees (JFSC + service providers), substance scoring (board composition, Jersey-resident directors, local administration), and post-Brexit EU distribution implications. Computes total cost of ownership and regulatory timeline. |
| `analyze_guernsey_fund` | `(GuernseyFundInput) -> GuernseyFundOutput` | Analysis of Guernsey fund structures: Private Investment Fund (PIF), Qualifying Investor Fund (QIF), Registered Qualifying Investor Fund (RQIF), and Authorised Fund. Covers GFSC registration vs. authorisation distinction, 50-investor PIF limit, NPPR access for EU marketing, annual regulatory and administration costs, substance requirements (Guernsey-resident functionaries), and comparison to Jersey equivalents. |
| `channel_islands_comparison` | `(ChannelIslandsComparisonInput) -> ChannelIslandsComparisonOutput` | Side-by-side comparison of Jersey vs. Guernsey for a given fund profile. Compares regulatory approval timeline, cost structure (formation + annual), substance requirements, investor access, distribution reach (NPPR availability by EU member state), tax treatment (0% corporate tax in both, but differing GST/TRP implications), and Brexit impact on EU investor access. Outputs recommendation with rationale. |
| `cell_company_analysis` | `(CellCompanyInput) -> CellCompanyOutput` | Analysis of Protected Cell Company (PCC) and Incorporated Cell Company (ICC) structures unique to the Channel Islands. Models cell segregation economics: shared core costs allocated across cells, per-cell regulatory fees, legal ring-fencing strength (PCC statutory vs. ICC corporate), insurance-linked securities (ILS) use case, and multi-manager platform economics. Computes break-even number of cells and cost-per-cell at various scales. |

**Key types**: `JerseyFundInput` (fund_name, structure_type: JPF/ExpertFund/ListedFund/QIF, fund_size, strategy, target_investors, jersey_directors_count, local_admin), `GuernseyFundInput` (fund_name, structure_type: PIF/QIF/RQIF/AuthorisedFund, fund_size, investor_count, guernsey_functionaries), `CellCompanyInput` (vehicle_type: PCC/ICC, num_cells, core_costs, per_cell_aum, strategy_per_cell), `ChannelIslandsComparisonInput` (fund_profile with strategy, size, investor_base, distribution_targets).

#### 2. singapore_vcc -- Singapore Variable Capital Company

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `analyze_vcc_structure` | `(VccInput) -> VccOutput` | Full analysis of Singapore VCC structures: standalone VCC and umbrella VCC with sub-funds. Covers VCC Act 2018 requirements, MAS licensing (LFMC/RFMC/A-LFMC), permitted fund manager types, capital variability mechanics, annual compliance (AGM, audited financials, ACRA filing), formation and ongoing costs, and substance requirements (Singapore-resident director, registered office, company secretary). Computes total cost of ownership for standalone vs. umbrella configurations. |
| `vcc_sub_fund_allocation` | `(SubFundAllocationInput) -> SubFundAllocationOutput` | Models umbrella VCC sub-fund economics: allocation of shared umbrella costs (board, company secretary, registered office, compliance officer) across sub-funds by AUM-weighted, equal-weighted, or hybrid methodology. Computes per-sub-fund total expense ratio (TER), marginal cost of adding a sub-fund, and break-even AUM per sub-fund. Handles cross-sub-fund investment restrictions and ring-fencing analysis. |
| `tax_incentive_analysis` | `(TaxIncentiveInput) -> TaxIncentiveOutput` | Evaluates eligibility and economic benefit of Singapore fund tax incentive schemes: Section 13O (onshore fund, formerly S13R, minimum S$10M AUM), Section 13U (enhanced tier, minimum S$50M AUM, 3 investment professionals), and Section 13D (offshore fund, no AUM minimum). Models qualifying income types (dividends, interest, gains), non-qualifying income leakage (Singapore-sourced trading income), withholding tax implications on inbound distributions, and net tax savings vs. a non-incentivised structure. |
| `vcc_vs_cayman_spc` | `(VccCaymanComparisonInput) -> VccCaymanComparisonOutput` | Head-to-head comparison of Singapore VCC umbrella vs. Cayman SPC for a given fund profile. Compares formation cost and timeline, annual operating cost, tax treatment (VCC with S13O/U/D vs. Cayman tax-neutral), substance requirements, FATCA/CRS reporting burden, Asian investor preferences, EU distribution access, and reputational considerations (FATF greylist exposure). Outputs quantified cost differential and qualitative recommendation. |

**Key types**: `VccInput` (fund_name, vcc_type: Standalone/Umbrella, sub_funds: Vec<SubFundInfo>, fund_manager_license: LFMC/RFMC/A_LFMC, target_aum, tax_incentive_scheme: Option<S13O/S13U/S13D>), `SubFundInfo` (name, strategy, target_aum, currency), `TaxIncentiveInput` (scheme, fund_aum, investment_professionals_count, qualifying_income_pct, singapore_sourced_income_pct), `MasLicenseType` enum (LFMC, RFMC, A_LFMC).

#### 3. hong_kong_funds -- Hong Kong OFC & LPF

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `analyze_ofc_structure` | `(OfcInput) -> OfcOutput` | Analysis of Hong Kong Open-Ended Fund Company (OFC) structures under the Securities and Futures (Open-ended Fund Companies) Rules. Covers SFC authorisation requirements, investment manager eligibility (Type 9 licensed), custodian requirements, umbrella OFC with sub-funds, capital variability, public vs. private OFC distinction, re-domiciliation into Hong Kong as OFC, and OFC grant scheme (up to HK$1M per OFC). Computes formation cost, annual operating cost, and regulatory timeline. |
| `analyze_lpf_structure` | `(LpfInput) -> LpfOutput` | Analysis of Hong Kong Limited Partnership Fund (LPF) structures under the Limited Partnership Fund Ordinance (Cap. 637). Covers eligible fund purposes (PE, VC, real estate, infrastructure, credit), GP/LP structure, investment manager requirements, responsible person obligation, auditor requirement (may be waived), Companies Registry filing, and anti-money laundering obligations. Computes cost structure and compares to Cayman LP for Asia-focused PE/VC strategies. |
| `carried_interest_concession` | `(CarriedInterestInput) -> CarriedInterestOutput` | Models the Hong Kong carried interest tax concession: 0% profits tax on eligible carried interest (vs. standard 16.5% rate) for qualifying PE funds. Evaluates eligibility criteria: fund must be certified by HKMA as qualifying fund, carried interest must relate to qualifying transactions (private company disposals, not public securities), and the fund must meet substance requirements (2 full-time employees in HK). Quantifies tax savings for GP economics and compares to Cayman/Singapore carry taxation. |
| `hk_vs_singapore` | `(HkSgComparisonInput) -> HkSgComparisonOutput` | Comparison of Hong Kong OFC/LPF vs. Singapore VCC for Asia-Pacific fund formation. Compares formation and annual costs, tax treatment (HK unified fund exemption + carry concession vs. SG S13O/U/D), substance requirements, Greater China market access (Stock Connect, Bond Connect, QDLP/QFLP), ASEAN market access, regulatory burden, and talent pool. Outputs quantified comparison matrix and recommendation by strategy type. |

**Key types**: `OfcInput` (fund_name, ofc_type: Public/Private, umbrella: bool, sub_funds: Vec<SubFundInfo>, investment_manager_type9: bool, grant_scheme_eligible: bool), `LpfInput` (fund_name, fund_purpose, gp_jurisdiction, target_aum, responsible_person, audit_waiver: bool), `CarriedInterestInput` (fund_type, qualifying_transactions_pct, hk_employees, carried_interest_amount, fund_certified: bool), `UnifiedExemptionInput` (fund_type, qualifying_asset_pct, hk_sourced_trading_pct).

#### 4. middle_east_funds -- DIFC & ADGM Fund Structures

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `analyze_difc_fund` | `(DifcFundInput) -> DifcFundOutput` | Analysis of Dubai International Financial Centre (DIFC) fund structures regulated by the Dubai Financial Services Authority (DFSA). Covers three fund categories: Qualified Investor Fund (QIF, minimum $500K per investor, streamlined registration), Exempt Fund ($50K minimum, DFSA notification), and Domestic Fund (retail, full DFSA authorisation). Models DFSA licensing costs (fund manager + fund vehicle), annual regulatory fees, 0% corporate tax within DIFC free zone, substance requirements (DIFC-registered office, compliance officer, MLRO), and distribution to GCC investors. |
| `analyze_adgm_fund` | `(AdgmFundInput) -> AdgmFundOutput` | Analysis of Abu Dhabi Global Market (ADGM) fund structures regulated by the Financial Services Regulatory Authority (FSRA). Covers Exempt Fund (qualified investors, simplified registration), Qualified Investor Fund, and Public Fund categories. Models FSRA licensing, ADGM registration fees, 0% tax (50-year guarantee from ADGM establishment in 2015), substance requirements, and comparison to DIFC for fund manager dual-licensing considerations. |
| `sharia_compliance_check` | `(ShariaComplianceInput) -> ShariaComplianceOutput` | Evaluates Sharia compliance for Islamic fund structuring. Covers prohibited activities screening (riba/interest, gharar/uncertainty, maysir/gambling, haram sectors), financial ratio screens (debt-to-assets <33%, interest income <5%, receivables <49%), Sharia board requirements (minimum 3 scholars), fund structure compliance (Mudarabah, Musharakah, Wakalah, Ijarah), and purification calculation for incidental non-compliant income. Outputs compliance score, purification amount, and remediation recommendations. |
| `free_zone_economics` | `(FreeZoneInput) -> FreeZoneOutput` | Comparative analysis of DIFC vs. ADGM vs. onshore UAE for fund management operations. Models total cost including license fees, visa costs (per employee), office space (per desk), regulatory fees, and compliance costs. Evaluates 0% tax guarantee duration, substance scoring, talent pool access, GCC distribution rights, and integration with UAE Federal laws (e.g., Economic Substance Regulations, Ultimate Beneficial Owner reporting). Computes 5-year and 10-year total cost of ownership. |

**Key types**: `DifcFundInput` (fund_name, fund_category: QIF/Exempt/Domestic, fund_size, strategy, dfsa_license_type, sharia_compliant: bool, gcc_distribution: bool), `AdgmFundInput` (fund_name, fund_category, fund_size, fsra_license_type, dual_licensed_difc: bool), `ShariaComplianceInput` (fund_strategy, portfolio_holdings: Vec<HoldingInfo>, debt_to_assets, interest_income_pct, receivables_pct, sharia_board_count), `FreeZoneInput` (entity_type, headcount, office_desks, fund_aum, comparison_zones: Vec<String>).

#### 5. jurisdiction_comparison -- Multi-Jurisdiction Comparison Engine

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `compare_jurisdictions` | `(JurisdictionComparisonInput) -> JurisdictionComparisonOutput` | Comprehensive side-by-side comparison of up to 12 jurisdictions across standardised dimensions: formation cost, formation timeline (weeks), annual operating cost, corporate/fund tax rate, substance requirements (score 0-100 reusing `substance_requirements` module), regulatory burden (score 0-100), investor protection (score 0-100), distribution reach (number of accessible markets), FATF rating (compliant/enhanced monitoring/greylist/blacklist), and reputational tier (1-3). Outputs a ranked comparison matrix with per-dimension scores and a composite weighted score using user-supplied or default dimension weights. Covers: Cayman, BVI, Luxembourg, Ireland, Jersey, Guernsey, Singapore, Hong Kong, DIFC, ADGM, Delaware, and Netherlands. |
| `optimal_jurisdiction` | `(OptimalJurisdictionInput) -> OptimalJurisdictionOutput` | Recommends the optimal jurisdiction(s) for a given fund profile. Takes fund strategy, target AUM, investor base (geography and type), distribution requirements (EU passport, US private placement, Asian marketing), substance budget, and priority weights (cost vs. tax vs. distribution vs. speed vs. reputation). Applies a weighted scoring model against the jurisdiction profiles and returns top-3 recommendations with rationale, trade-off analysis, and risk factors. Integrates with `tax_treaty/optimization` for WHT-efficient holding structures. |
| `distribution_reach_analysis` | `(DistributionReachInput) -> DistributionReachOutput` | Analyzes marketing and distribution capabilities from each jurisdiction. Models: AIFMD passport (EU domiciles), NPPR access (non-EU domiciles, per EU member state), US private placement (Reg D/S, ERISA considerations), Asian distribution (MRF agreements, bilateral MOUs), GCC access, and Latin American distribution. For each jurisdiction, outputs accessible investor markets, regulatory requirements per market, and estimated marketing compliance costs. Integrates with `regulatory_reporting` for AIFMD Annex IV and SEC Form PF obligations triggered by distribution. |
| `total_cost_of_ownership` | `(TcoInput) -> TcoOutput` | Computes 5-year and 10-year total cost of ownership for a fund structure in each specified jurisdiction. Includes: formation costs (legal, regulatory filing, initial capital), annual regulatory fees, administration and audit fees, director/officer fees, substance costs (office, personnel, board meetings), tax costs (fund-level, investor-level WHT via `tax_treaty/treaty_network`), FATCA/CRS compliance costs (via `fatca_crs` module classification), and AML/KYC setup costs (via `aml_compliance` module). Discounts future costs to present value and outputs NPV comparison with sensitivity to AUM growth assumptions. |

**Key types**: `JurisdictionComparisonInput` (jurisdictions: Vec<String>, fund_profile: FundProfile, dimension_weights: Option<DimensionWeights>), `JurisdictionProfile` (jurisdiction, formation_cost, annual_cost, tax_rate, substance_score, regulatory_score, distribution_reach, fatf_status, reputation_tier), `DistributionPassport` (jurisdiction, passport_type: AIFMD/NPPR/RegD/MRF/Bilateral, accessible_markets: Vec<String>, compliance_cost), `CostBreakdown` (formation, annual_regulatory, administration, audit, directors, substance, tax, fatca_crs, aml_kyc, total_5yr_npv, total_10yr_npv).

#### 6. fund_migration -- Fund Migration & Redomiciliation

| Function | Signature (simplified) | Description |
|----------|----------------------|-------------|
| `migration_feasibility` | `(MigrationInput) -> MigrationFeasibilityOutput` | Evaluates whether a fund can migrate from one jurisdiction to another. Checks: statutory continuation/domestication availability (e.g., Cayman allows outbound continuation under Companies Act s.206), target jurisdiction acceptance of inbound continuation, fund structure compatibility (LP-to-LP, company-to-company), regulatory approval requirements (both origin and destination), investor consent thresholds (typically 75% by value for scheme of arrangement, simple majority for LP continuation), and material contract portability (ISDA, prime brokerage, custody). Outputs feasibility score, blocking issues, and required approvals. |
| `redomiciliation_cost_benefit` | `(RedomiciliationCostBenefitInput) -> RedomiciliationCostBenefitOutput` | Quantifies the economic case for redomiciliation. Compares: current jurisdiction total annual cost vs. target jurisdiction total annual cost (reusing `total_cost_of_ownership` logic), one-time migration costs (legal fees, regulatory filing, investor communication, contract novation), tax consequences of migration (via `tax_treaty` module for treaty continuity), distribution impact (gained/lost market access), substance cost differential, and break-even period (years until cumulative savings exceed migration costs). Outputs NPV of migration decision over 5-year and 10-year horizons. |
| `migration_timeline` | `(MigrationTimelineInput) -> MigrationTimelineOutput` | Projects the end-to-end migration timeline. Models parallel and sequential work streams: board/GP approval (2-4 weeks), investor notification and consent period (30-90 days depending on LPA/articles), origin regulator deregistration (4-12 weeks by jurisdiction), destination regulator registration/authorisation (4-24 weeks by jurisdiction), contract novation and counterparty notification (4-8 weeks), service provider transition (administrator, auditor, legal counsel), and NAV reconciliation/closing. Outputs Gantt-style timeline with critical path and total estimated duration. |
| `tax_consequence_analysis` | `(TaxConsequenceInput) -> TaxConsequenceOutput` | Analyzes tax implications of fund migration. Covers: deemed disposal rules (some jurisdictions treat migration as taxable event for investors), treaty continuity (whether existing treaty benefits survive migration), WHT rate changes on portfolio income (dividends, interest, royalties via `tax_treaty/treaty_network`), capital gains tax on unrealised gains at migration date, investor-level tax reporting changes (FATCA/CRS reclassification via `fatca_crs/classification`), and anti-avoidance provisions (GAAR/SAAR in origin and destination). Outputs per-investor-type tax impact matrix (US taxable, US tax-exempt, EU institutional, Asian SWF). |

**Key types**: `MigrationInput` (origin_jurisdiction, target_jurisdiction, fund_structure, fund_aum, investor_count, investor_types: Vec<String>, material_contracts: Vec<String>), `RedomiciliationRoute` (mechanism: Continuation/Domestication/Merger/Liquidation_Relaunch, statutory_basis, origin_approval, destination_approval), `MigrationCostBenefit` (migration_costs, annual_savings, break_even_years, npv_5yr, npv_10yr), `TaxConsequence` (deemed_disposal: bool, treaty_continuity: bool, wht_impact_per_investor_type: HashMap, unrealised_gains_tax, anti_avoidance_flags: Vec<String>).

### MCP Tools

6 new MCP tools (one per sub-module), registered in `packages/mcp-server/`:

| Tool Name | Sub-module | Description |
|-----------|-----------|-------------|
| `offshore_channel_islands` | channel_islands | Jersey & Guernsey fund structure analysis, PCC/ICC cell economics, and Channel Islands comparison |
| `offshore_singapore_vcc` | singapore_vcc | Singapore VCC analysis, sub-fund allocation, tax incentive evaluation, and VCC vs. Cayman SPC comparison |
| `offshore_hong_kong_funds` | hong_kong_funds | Hong Kong OFC/LPF analysis, carried interest concession, and HK vs. Singapore comparison |
| `offshore_middle_east_funds` | middle_east_funds | DIFC and ADGM fund analysis, Sharia compliance screening, and free zone economics |
| `offshore_jurisdiction_comparison` | jurisdiction_comparison | Multi-jurisdiction comparison engine, optimal jurisdiction recommendation, distribution reach, and TCO analysis |
| `offshore_fund_migration` | fund_migration | Migration feasibility, redomiciliation cost-benefit, timeline projection, and tax consequence analysis |

This brings the total MCP tool count from 206 to 212.

### NAPI Bindings

12 new NAPI bindings in `crates/corp-finance-bindings/src/`:

| Binding | Maps To |
|---------|---------|
| `napi_analyze_jersey_fund` | channel_islands::analyze_jersey_fund |
| `napi_analyze_guernsey_fund` | channel_islands::analyze_guernsey_fund |
| `napi_analyze_vcc_structure` | singapore_vcc::analyze_vcc_structure |
| `napi_vcc_tax_incentive` | singapore_vcc::tax_incentive_analysis |
| `napi_analyze_ofc_structure` | hong_kong_funds::analyze_ofc_structure |
| `napi_carried_interest_concession` | hong_kong_funds::carried_interest_concession |
| `napi_analyze_difc_fund` | middle_east_funds::analyze_difc_fund |
| `napi_sharia_compliance_check` | middle_east_funds::sharia_compliance_check |
| `napi_compare_jurisdictions` | jurisdiction_comparison::compare_jurisdictions |
| `napi_optimal_jurisdiction` | jurisdiction_comparison::optimal_jurisdiction |
| `napi_migration_feasibility` | fund_migration::migration_feasibility |
| `napi_redomiciliation_cost_benefit` | fund_migration::redomiciliation_cost_benefit |

All bindings follow the existing JSON string boundary pattern (`String -> String` via serde).

### Slash Commands

2 new CFA slash commands in `.claude/commands/cfa/`:

| Command | Routed To | Description |
|---------|-----------|-------------|
| `/cfa jurisdiction-comparison` | `cfa-chief-analyst` | Multi-jurisdiction fund structure comparison with optimal domicile recommendation, distribution analysis, and total cost of ownership |
| `/cfa fund-migration` | `cfa-chief-analyst` | Fund redomiciliation feasibility analysis with cost-benefit, timeline, and tax consequence modeling |

This brings the total from 25 (post-ADR-013) to 27 CFA slash commands.

### Skill Updates

Update `.claude/skills/corp-finance-tools-regulatory/SKILL.md` to include the 6 new MCP tools under a new "Advanced Offshore Fund Structures" section. Update `AGENT_SKILLS` in `packages/agents/src/pipeline.ts`:

| Agent | Added Capabilities |
|-------|-------------------|
| `cfa-chief-analyst` | +offshore_jurisdiction_comparison, +offshore_fund_migration |
| `cfa-private-markets-analyst` | +offshore_channel_islands, +offshore_singapore_vcc, +offshore_hong_kong_funds, +offshore_middle_east_funds, +offshore_jurisdiction_comparison, +offshore_fund_migration |
| `cfa-quant-risk-analyst` | +offshore_jurisdiction_comparison (for portfolio-level domicile risk analysis) |

### Integration with Existing Modules

The 6 new sub-modules **extend** the existing codebase without replacing any module:

| Existing Module | Integration |
|----------------|-------------|
| `offshore_structures/cayman` | `jurisdiction_comparison::compare_jurisdictions` calls `cayman::analyze_cayman_structure` to populate the Cayman column in comparison matrices. `singapore_vcc::vcc_vs_cayman_spc` references Cayman SPC cost and regulatory data. `fund_migration` supports Cayman as both origin and destination jurisdiction. |
| `offshore_structures/luxembourg` | `jurisdiction_comparison::compare_jurisdictions` calls `luxembourg::analyze_lux_structure` for Luxembourg/Ireland columns. `distribution_reach_analysis` references AIFMD passport mechanics from Luxembourg module. |
| `substance_requirements/economic_substance` | `jurisdiction_comparison::compare_jurisdictions` calls substance scoring functions to compute per-jurisdiction substance scores. `channel_islands`, `singapore_vcc`, `hong_kong_funds`, and `middle_east_funds` each include substance evaluation aligned with the 5-dimension scoring model (personnel, premises, decision-making, expenditure, CIGA). |
| `substance_requirements/jurisdiction_tests` | `fund_migration::migration_feasibility` validates that the target jurisdiction's substance requirements can be met. `optimal_jurisdiction` filters recommendations by substance achievability. |
| `tax_treaty/treaty_network` | `jurisdiction_comparison::total_cost_of_ownership` uses treaty network data to compute investor-level WHT costs per jurisdiction. `fund_migration::tax_consequence_analysis` evaluates treaty continuity post-migration. `hong_kong_funds` and `singapore_vcc` reference treaty networks for dividend/interest WHT on portfolio income. |
| `tax_treaty/optimization` | `optimal_jurisdiction` integrates WHT-efficient holding structure routing from treaty optimization. `redomiciliation_cost_benefit` compares pre- and post-migration WHT efficiency. |
| `fatca_crs/classification` | `jurisdiction_comparison::total_cost_of_ownership` includes FATCA/CRS compliance cost per jurisdiction. `fund_migration::tax_consequence_analysis` evaluates reclassification impact (FFI/NFFE/NFE status changes). `singapore_vcc` and `hong_kong_funds` include FATCA/CRS reporting obligations in cost models. |
| `fatca_crs/reporting` | `distribution_reach_analysis` flags jurisdictions where CRS wider vs. narrower approach affects reporting burden. |
| `aml_compliance/kyc_scoring` | `total_cost_of_ownership` includes AML/KYC setup and ongoing compliance costs. `sharia_compliance_check` cross-references AML risk scoring for GCC-domiciled structures. |
| `aml_compliance/sanctions_screening` | `fund_migration::migration_feasibility` flags sanctions-related blocking issues for migrations involving restricted jurisdictions. |
| `onshore_structures/` | `jurisdiction_comparison` includes Delaware and Netherlands as onshore comparison points. `distribution_reach_analysis` models onshore/offshore hybrid structures (e.g., Delaware feeder into Cayman master, or Luxembourg GP with Cayman fund). |
| `regulatory_reporting/` | `distribution_reach_analysis` identifies AIFMD Annex IV and SEC Form PF filing obligations triggered by distribution into EU and US markets. `channel_islands` models NPPR reporting requirements per EU member state. |

### Mathematical Standards

- All monetary and rate calculations use `rust_decimal::Decimal` with the `maths` feature for `powd()` where required
- NPV discount factors use iterative multiplication (not `powd()`) to avoid precision drift, consistent with existing modules
- Weighted scoring models (jurisdiction comparison, substance scoring, Sharia compliance) use Decimal arithmetic throughout
- Cost projections with inflation escalation use iterative compounding
- Break-even period computation uses iterative year-by-year cumulative savings comparison (not algebraic inversion) to handle non-uniform cost profiles
- No `f64` except where explicitly noted (none anticipated in this module)

### Test Targets

| Sub-module | Test Count | Key Test Scenarios |
|-----------|-----------|-------------------|
| channel_islands | ~50 | JPF/Expert/Listed/QIF structures, PCC with 1/5/20 cells (break-even), ICC vs. PCC comparison, Jersey vs. Guernsey side-by-side, NPPR availability per EU state, substance scoring with varying director counts, post-Brexit distribution restrictions |
| singapore_vcc | ~55 | Standalone vs. umbrella VCC, 1/3/10 sub-fund allocation, S13O/S13U/S13D eligibility and tax savings, non-qualifying income leakage, VCC vs. Cayman SPC cost comparison, MAS license type impact, FATCA/CRS reporting cost |
| hong_kong_funds | ~45 | Public vs. private OFC, LPF with/without audit waiver, carried interest concession eligibility (all criteria met, partial, none), unified fund exemption qualifying asset threshold, HK vs. Singapore comparison by strategy (PE, VC, hedge, credit), OFC grant scheme economics |
| middle_east_funds | ~50 | DIFC QIF/Exempt/Domestic, ADGM fund categories, DIFC vs. ADGM comparison, Sharia compliance pass/fail/partial (sector screen, financial ratio screen, purification), free zone TCO with varying headcount, dual-licensing economics, GCC distribution modeling |
| jurisdiction_comparison | ~55 | 2-jurisdiction and 12-jurisdiction comparisons, custom vs. default dimension weights, optimal jurisdiction for PE/hedge/VC/credit strategies, distribution reach with AIFMD passport vs. NPPR, TCO 5-year and 10-year NPV with AUM growth sensitivity, substance-constrained optimization, FATF greylist impact on scoring |
| fund_migration | ~45 | Cayman-to-Singapore continuation, BVI-to-Jersey domestication, Luxembourg-to-Ireland merger, infeasible migration (no statutory mechanism), investor consent threshold (above/below 75%), break-even at 2/5/10 years, treaty continuity (maintained/broken), deemed disposal tax impact per investor type, critical path timeline calculation |
| **Total** | **~300** | |

This brings the projected test count from approximately 6,127 to approximately 6,427.

## Consequences

### Positive

- Expands offshore coverage from 2 domiciles (Cayman, Luxembourg/Ireland) to 10+ (adding Jersey, Guernsey, Singapore, Hong Kong, DIFC, ADGM, BVI, Delaware, Netherlands), covering the vast majority of global fund formation activity
- Multi-jurisdiction comparison engine enables systematic domicile selection, replacing ad-hoc spreadsheet analysis with quantified, auditable recommendations
- Fund migration analytics address the growing post-Brexit redomiciliation market and Asia-Pacific fund formation shift
- Sharia compliance screening opens the platform to Islamic finance use cases, a $3.9T global market
- Channel Islands coverage fills a critical gap for UK-linked fund managers who lost EU passporting rights
- Singapore VCC and Hong Kong OFC/LPF modules serve the fastest-growing fund domicile markets in Asia-Pacific
- Deep integration with 6 existing modules (substance, treaty, FATCA/CRS, AML, onshore, regulatory reporting) creates a cohesive cross-jurisdictional analytics layer
- ~300 new tests maintain the project's comprehensive test coverage standard
- 6 new MCP tools and 12 NAPI bindings follow established patterns (JSON string boundary, no architectural changes)

### Negative

- Module adds approximately 5,000-6,000 lines of Rust across 6 sub-modules, increasing compile time by an estimated 8-12 seconds
- Jurisdiction-specific regulatory data (fee schedules, approval timelines, minimum thresholds) is encoded as constants and will require periodic updates as regulators change fee structures; this is particularly relevant for DIFC/ADGM which adjust fees more frequently than mature jurisdictions
- Sharia compliance screening is a simplification of what is in practice a scholarly-judgment-driven process; the quantitative screens (debt ratios, sector exclusions) are well-defined, but purification calculations and structuring guidance should be reviewed by a qualified Sharia board
- Tax consequence analysis for fund migration is inherently jurisdiction-pair-specific; the module covers the most common migration routes but cannot exhaustively model every origin-destination combination
- Adding 6 MCP tools brings the total to 212, contributing to context window pressure for agents carrying the full tool list
- Multi-jurisdiction comparison requires maintaining accurate profiles for 12 jurisdictions; data staleness in any single jurisdiction degrades comparison quality

## Options Considered

### Option 1: Add jurisdiction modules incrementally, one per phase (Rejected)

- **Pros**: Smaller per-phase scope, lower risk per delivery
- **Cons**: The jurisdiction comparison engine and fund migration module depend on having multiple jurisdictions available simultaneously. Delivering Channel Islands alone without comparison capability provides limited value over a static reference document. The established two-wave parallel agent pattern (proven in Phases 10-22) handles 6 sub-modules efficiently within a single phase.

### Option 2: Implement jurisdiction data as configuration files rather than Rust code (Considered)

- **Pros**: Jurisdiction profiles (fee schedules, timelines, thresholds) could be updated without recompilation by storing them in JSON/YAML configuration. Easier for non-developers to maintain.
- **Cons**: Regulatory logic (eligibility criteria, substance tests, distribution rules) is not purely data-driven; it requires conditional logic (e.g., Jersey PIF 50-investor limit, Singapore S13U 3-professional requirement, Hong Kong carry concession qualification tests). Splitting data from logic creates a maintenance boundary that complicates testing. The existing pattern (Cayman and Luxembourg modules) encodes regulatory parameters as Rust constants, and consistency with this pattern is preferred. A future ADR could address configuration-driven jurisdiction data if update frequency warrants it.

### Option 3: Build comparison engine only, without new jurisdiction modules (Rejected)

- **Pros**: Comparison engine could use simplified jurisdiction profiles without detailed per-jurisdiction analysis
- **Cons**: Users need both detailed single-jurisdiction analysis (for fund formation) and comparative analysis (for domicile selection). A comparison engine without underlying analytical modules would provide shallow rankings without the detailed cost breakdowns, substance evaluations, and regulatory assessments that inform the comparison scores. The detailed modules also serve standalone use cases (e.g., a fund already committed to Singapore VCC needs only `analyze_vcc_structure`, not a comparison).

### Option 4: Implement as a separate `jurisdiction_analytics` top-level module (Considered)

- **Pros**: Clean separation between the new multi-jurisdiction modules and the existing `offshore_structures` module
- **Cons**: The new sub-modules are logically part of offshore fund structuring -- they extend the same bounded context. Creating a separate top-level module would fragment related functionality and require cross-module imports that are better expressed as intra-module references. The `mod.rs` pattern of listing sub-modules is well-established and accommodates 8 sub-modules (2 existing + 6 new) without complexity concerns.

## Related Decisions

- ADR-013: Institutional Commercial Real Estate Analytics (immediate predecessor; established the sub-module pattern, test targets, and MCP/NAPI/slash-command expansion methodology used in this ADR)
- ADR-008: Financial Services Workflow Integration (workflow skills and slash command routing patterns)
- ADR-009: Workflow Auditability (audit hashing for workflow definitions, applicable to new slash commands)

## References

- [Jersey Financial Services Commission (JFSC) - Fund Regulation](https://www.jerseyfsc.org/industry/sectors/funds/)
- [Guernsey Financial Services Commission (GFSC) - Investment Funds](https://www.gfsc.gg/industry-sectors/investment)
- [Jersey Private Fund (JPF) Guide](https://www.jerseyfinance.je/funds/)
- [Singapore Variable Capital Companies Act 2018](https://sso.agc.gov.sg/Act/VCCA2018)
- [Monetary Authority of Singapore (MAS) - Fund Management](https://www.mas.gov.sg/regulation/fund-management)
- [Singapore S13O/S13U/S13D Tax Incentive Schemes](https://www.mas.gov.sg/schemes-and-initiatives)
- [Hong Kong Open-Ended Fund Company (OFC) Rules](https://www.sfc.hk/en/Rules-and-standards/Codes-and-guidelines/OFC)
- [Hong Kong Limited Partnership Fund Ordinance (Cap. 637)](https://www.elegislation.gov.hk/)
- [Hong Kong Carried Interest Tax Concession (Inland Revenue Amendment Ordinance 2021)](https://www.ird.gov.hk/)
- [DIFC - Dubai Financial Services Authority (DFSA) Rulebook](https://www.dfsa.ae/rulebook)
- [ADGM - Financial Services Regulatory Authority (FSRA)](https://www.adgm.com/operating-in-adgm/financial-services-regulatory-authority)
- [AAOIFI Sharia Standards (Islamic Finance)](https://aaoifi.com/shariaa-standards/)
- [FATF Jurisdictions under Increased Monitoring](https://www.fatf-gafi.org/en/countries/black-and-grey-lists.html)
- [Cayman Islands Companies Act s.206 - Continuation](https://legislation.gov.ky/)
- [AIFMD National Private Placement Regime (NPPR) Status by Member State](https://www.esma.europa.eu/)
