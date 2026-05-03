---
name: Corp Finance Tools - Specialty & Regulatory
description: Use the corp-finance-mcp server tools for specialty finance, regulatory, and compliance calculations. Invoke when performing private credit (unitranche, direct lending, syndication), insurance (loss reserving, premium pricing, Solvency II SCR), FP&A (variance analysis, break-even, working capital, rolling forecast), wealth management (retirement planning, tax-loss harvesting, estate planning), restructuring (recovery analysis, distressed debt), real assets (property valuation, project finance), venture capital (dilution, convertible instruments, fund returns), ESG (scoring, climate/carbon, green bonds, SLL), regulatory capital (Basel III, LCR/NSFR, ALM), compliance (MiFID II best execution, GIPS reporting), credit derivatives (CDS pricing, CVA/DVA), convertible bonds (binomial tree pricing, scenario analysis), lease accounting (ASC 842/IFRS 16, sale-leaseback), pension & LDI (funding analysis, liability-driven investing), sovereign risk (bond analysis, country risk), real options (binomial valuation, decision trees), equity research (SOTP, target price), commodity trading (spread analysis, storage economics), treasury management (cash management, hedge effectiveness), infrastructure finance (PPP models, concession valuation), crypto (token valuation, DeFi analysis), municipal bonds (pricing, credit analysis), structured products (notes, exotic), trade finance (LC, supply chain), fund structuring (US onshore, UK/EU, Cayman/BVI offshore, Luxembourg/Ireland), transfer pricing (BEPS/Pillar Two, intercompany pricing), tax treaty (treaty network optimization, holding structures), FATCA/CRS (reporting, entity classification), economic substance (multi-jurisdiction testing), regulatory reporting (AIFMD Annex IV, SEC Form PF, CFTC CPO-PQR), AML compliance (KYC risk scoring, sanctions screening), fund of funds (J-curve, commitment pacing, manager selection, secondaries pricing), bank analytics (NIM analysis, CAMELS rating, CECL provisioning, deposit beta, loan book), carbon markets (credit pricing, ETS compliance, CBAM, offset valuation, shadow carbon price), private wealth (concentrated stock, philanthropic vehicles, wealth transfer, direct indexing, family governance). All computation uses 128-bit decimal precision.
requires_tools:
  - analyze_alm
  - analyze_beps_compliance
  - analyze_best_execution
  - analyze_breakeven
  - analyze_carbon_footprint
  - analyze_cash_management
  - analyze_cayman_structure
  - analyze_cbam
  - analyze_combined_ratio
  - analyze_commodity_spread
  - analyze_concentrated_stock
  - analyze_convertible
  - analyze_decision_tree
  - analyze_defi
  - analyze_deposit_beta
  - analyze_difc_fund
  - analyze_dilution
  - analyze_direct_indexing
  - analyze_distressed_debt
  - analyze_economic_substance
  - analyze_ets_compliance
  - analyze_fatca_crs_reporting
  - analyze_fof_portfolio
  - analyze_green_bond
  - analyze_hedging
  - analyze_intercompany
  - analyze_jersey_fund
  - analyze_loan_book
  - analyze_lux_structure
  - analyze_manager_selection
  - analyze_municipal
  - analyze_nim
  - analyze_ofc_structure
  - analyze_pension_funding
  - analyze_recovery
  - analyze_sale_leaseback
  - analyze_sovereign_bond
  - analyze_storage_economics
  - analyze_supply_chain_finance
  - analyze_syndication
  - analyze_treaty_network
  - analyze_uk_eu_fund
  - analyze_us_fund_structure
  - analyze_variance
  - analyze_vcc_structure
  - analyze_wealth_transfer
  - analyze_working_capital
  - assess_country_risk
  - assess_kyc_risk
  - build_rolling_forecast
  - calculate_camels_rating
  - calculate_cecl_provision
  - calculate_commitment_pacing
  - calculate_cva
  - calculate_esg_score
  - calculate_j_curve
  - calculate_lcr
  - calculate_regulatory_capital
  - calculate_scr
  - calculate_secondaries_pricing
  - calculate_shadow_carbon_price
  - calculate_sotp
  - calculate_target_price
  - classify_entity
  - classify_lease
  - compare_jurisdictions
  - compare_philanthropic_vehicles
  - convert_note
  - design_ldi_strategy
  - estimate_reserves
  - evaluate_family_governance
  - generate_aifmd_report
  - generate_gips_report
  - generate_sec_cftc_report
  - migration_feasibility
  - model_direct_loan
  - model_ppp
  - model_project_finance
  - model_venture_fund
  - optimize_treaty_structure
  - plan_estate
  - plan_retirement
  - price_carbon_credit
  - price_cds
  - price_convertible
  - price_exotic
  - price_letter_of_credit
  - price_muni_bond
  - price_premium
  - price_structured_note
  - price_unitranche
  - run_jurisdiction_substance_test
  - screen_sanctions
  - simulate_tax_loss_harvesting
  - test_sll_covenants
  - value_carbon_offset
  - value_concession
  - value_property
  - value_real_option
  - value_token
---
# Corp Finance MCP Tools - Specialty & Regulatory

You have access to 94 specialty finance, regulatory, and compliance MCP tools covering private credit, insurance, FP&A, wealth management, restructuring, real assets, venture capital, ESG, regulatory capital, compliance, credit derivatives, convertible bonds, lease accounting, pension & LDI, sovereign risk, real options, equity research, commodity trading, treasury management, infrastructure finance, crypto, municipal bonds, structured products, trade finance, fund structuring, transfer pricing, tax treaty, FATCA/CRS, economic substance, regulatory reporting, AML compliance, fund of funds, bank analytics, carbon markets, and private wealth. All tools return structured JSON with `result`, `methodology`, `assumptions`, `warnings`, and `metadata` fields. All monetary math uses `rust_decimal` (128-bit fixed-point) — never floating-point.

## Tool Reference

### Private Credit

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `price_unitranche` | Unitranche FOLO split pricing and blended yield | total_commitment, first_out_pct, spreads, OID, fees, borrower metrics |
| `model_direct_loan` | Direct loan modelling (PIK, delayed draw, amortisation) | loan_amount, base_rate, spread, pik_rate, amort_schedule, maturity, credit metrics |
| `analyze_syndication` | Loan syndication allocation and arranger economics | facility_size, arranger_hold, syndicate_members, arrangement_fee |

### Insurance & Actuarial

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `estimate_reserves` | Chain-ladder and Bornhuetter-Ferguson loss reserving | claims_triangle, method (ChainLadder/BF/Both), earned_premium, expected_loss_ratio, tail_factor |
| `price_premium` | Insurance premium pricing (freq x severity + loadings) | expected_frequency, expected_severity, expense_loading, profit_loading, trend_rates |
| `analyze_combined_ratio` | Multi-period combined ratio and operating ratio analysis | periods (premium, losses, expenses, investment_income) |
| `calculate_scr` | Solvency II Standard Formula SCR calculation | premium_reserve_risk, operational_risk, correlation_matrix, MCR_floor |

### FP&A

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_variance` | Budget vs actual variance with price/volume/mix decomposition | budget/actual revenue_lines, cost_lines, prior_period (optional) |
| `analyze_breakeven` | Break-even, DOL, and target volume analysis | selling_price, variable_cost_per_unit, fixed_costs, current_volume, scenarios |
| `analyze_working_capital` | Working capital efficiency (DSO/DIO/DPO/CCC) and benchmarking | periods (revenue, cogs, receivables, inventory, payables), cost_of_capital |
| `build_rolling_forecast` | Rolling financial forecast with driver-based projections | historical_periods, forecast_periods, revenue_growth_rate, driver_overrides |

### Wealth Management

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `plan_retirement` | Retirement planning with 4 withdrawal strategies | current_age, retirement_age, life_expectancy, income, savings, withdrawal_strategy |
| `simulate_tax_loss_harvesting` | Tax-loss harvesting simulation with wash-sale rules | positions (cost_basis, market_value, holding_days), realized_gains, tax_rates |
| `plan_estate` | Estate tax planning with trust analysis and gifting strategy | estate_value, gifts, trusts, life_insurance, exemption, tax_rates |

### Restructuring

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_recovery` | APR waterfall recovery by claim priority | enterprise_value, claims (priority, secured, collateral), DIP facility, admin costs |
| `analyze_distressed_debt` | Restructuring plan analysis with fulcrum ID | enterprise_value, exit_ev, capital_structure, proposed_treatments, DIP terms |

### Real Assets

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `value_property` | Real estate valuation (direct cap, DCF, GRM) | gross_rent, vacancy, opex, cap_rate, holding_period, financing terms, comparables |
| `model_project_finance` | Infrastructure project finance with debt sculpting | total_cost, construction/operating periods, revenue, debt (level/sculpted/bullet), DSCR target |

### Venture Capital

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_dilution` | Pre/post-money dilution and cap table modelling | rounds (pre_money, investment, option_pool_pct), founders_shares |
| `convert_note` | SAFE and convertible note conversion analysis | instrument_type (SAFE/Note), investment, valuation_cap, discount_rate, interest_rate |
| `model_venture_fund` | VC fund return analytics (IRR, TVPI, DPI, J-curve) | fund_size, investments (amount, entry/exit year, exit_multiple), management_fee, carry_rate, hurdle |

### ESG & Climate

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `calculate_esg_score` | ESG scoring with sector-specific materiality weights | company, sector, environmental/social/governance pillar scores |
| `analyze_carbon_footprint` | Carbon footprint analysis (Scope 1/2/3) | scope1/2/3 emissions, revenue, sector benchmarks |
| `analyze_green_bond` | Green bond framework analysis | proceeds_allocation, eligible_categories, impact_metrics |
| `test_sll_covenants` | Sustainability-linked loan covenant testing | kpi_targets, actual_performance, margin_adjustment |

### Regulatory Capital

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `calculate_regulatory_capital` | Basel III capital adequacy (CET1, Tier1, Total) with SA risk weights | exposures (asset_class, rating, amount), operational_risk, capital_buffers |
| `calculate_lcr` | Liquidity coverage ratio and net stable funding ratio | hqla_assets, cash_outflows/inflows, available/required_stable_funding |
| `analyze_alm` | Asset-liability management (gap, NII sensitivity, EVE) | assets/liabilities by repricing bucket, rate scenarios, beta pass-through |

### Compliance

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_best_execution` | MiFID II best execution (Perold implementation shortfall TCA) | trades (security, side, decision_price, execution_price, shares, benchmark_price), market_conditions, venue_data |
| `generate_gips_report` | GIPS-compliant performance reporting (Modified Dietz, geometric linking) | composite_name, periods (start_value, end_value, external_cash_flows, benchmark_return), accounts, firm_assets, currency |

### Credit Derivatives

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `price_cds` | Single-name CDS pricing (hazard-rate model) | reference_entity, notional, spread_bps, recovery_rate, risk_free_rate, maturity_years, payment_frequency |
| `calculate_cva` | CVA/DVA calculation with netting and collateral | trade_description, expected_exposure_profile, counterparty_default_probability, counterparty_recovery_rate, netting_benefit, collateral_threshold |

### Convertible Bonds

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `price_convertible` | Convertible bond pricing (CRR binomial tree) | bond_name, face_value, coupon_rate, maturity_years, stock_price, conversion_ratio, stock_volatility, call_price, put_price |
| `analyze_convertible` | Convertible scenario analysis (stock/vol/spread sensitivity) | bond_name, face_value, stock_price, conversion_ratio, stock_scenarios, vol_scenarios, spread_scenarios |

### Lease Accounting

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `classify_lease` | ASC 842 / IFRS 16 lease classification and measurement | lease_description, standard, lease_term_months, monthly_payment, fair_value_of_asset, useful_life_months, transfer_of_ownership, specialized_asset |
| `analyze_sale_leaseback` | Sale-leaseback transaction analysis (gain recognition) | description, standard, asset_carrying_value, sale_price, fair_value, lease_term_months, qualifies_as_sale |

### Pension & LDI

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_pension_funding` | Pension funding analysis (PBO, ABO, NPPC) | plan_name, plan_assets, discount_rate, expected_return_on_assets, active/retired_participants, plan_provisions |
| `design_ldi_strategy` | Liability-Driven Investing strategy design | plan_name, liability_pv, liability_duration, plan_assets, current_asset_allocation, available_instruments, target_hedge_ratio |

### Sovereign Risk

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_sovereign_bond` | Sovereign bond pricing, YTM, duration, convexity, spread decomposition, local currency risk | face_value, coupon_rate, maturity_years, sovereign_spread, currency, country, is_local_currency, inflation_rate |
| `assess_country_risk` | Multi-factor sovereign risk scoring, rating equivalent, CRP, implied default probability | country, gdp_growth_rate, inflation_rate, debt_to_gdp, current_account_pct_gdp, fx_reserves_months_imports, political_stability_score, rule_of_law_score |

### Real Options

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `value_real_option` | Real option valuation (expand, abandon, defer, switch, contract, compound) via CRR binomial tree with Greeks | option_type, underlying_value, exercise_price, volatility, risk_free_rate, time_to_expiry, expansion_factor, contraction_factor |
| `analyze_decision_tree` | Decision tree analysis with EMV rollback, EVPI, sensitivity, optimal path identification | nodes (id, name, node_type, value, cost, probability, children), discount_rate, risk_adjustment |

### Equity Research

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `calculate_sotp` | Sum-of-the-parts valuation: segment-level multiples, conglomerate discount, football field | company_name, segments (name, revenue, ebitda, method, multiple), net_debt, shares_outstanding, holding_company_discount |
| `calculate_target_price` | Multi-method target price: PE, PEG, PB, PS, DDM with football field and recommendation | current_price, shares_outstanding, earnings_per_share, earnings_growth_rate, book_value_per_share, peer_multiples, cost_of_equity |

### Commodity Trading

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_commodity_spread` | Commodity spread analysis: crack, crush, spark, calendar, location, quality spreads | spread_type, input_prices, output_prices, conversion_ratios, processing_cost, historical_spreads |
| `analyze_storage_economics` | Commodity storage economics: contango/backwardation, convenience yields, cash-and-carry arbitrage | spot_price, futures_prices, storage_cost_per_unit_month, financing_rate, commodity_name, seasonal_factors |

### Treasury Management

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_cash_management` | Corporate cash management: liquidity forecasting, cash pooling, sweep/facility draw | current_cash, operating_cash_flows, minimum_cash_buffer, credit_facility_size/rate, investment_rate, sweep_threshold, dso_days, dpo_days |
| `analyze_hedging` | Hedge effectiveness testing: dollar offset, regression, IAS 39/IFRS 9 compliance | hedge_type, notional_amount, hedge_notional, hedge_instrument, exposure_changes, hedge_changes, spot_rate, forward_rate, volatility |

### Infrastructure Finance

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `model_ppp` | PPP modelling: risk allocation, VfM analysis, PSC comparator, equity IRR, debt sizing | project_name, total_capex, concession_years, revenue_model, annual_availability_payment, senior_debt_pct/rate, equity_pct, discount_rate |
| `value_concession` | Infrastructure concession valuation: traffic risk, toll escalation, handback, extension option | concession_name, remaining_years, current_annual_revenue, revenue_growth_rate, handback_cost, discount_rate, terminal_value_approach |

### Crypto & Digital Assets

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `value_token` | Token/protocol valuation (NVT, P/S, FDV, DCF) | network_value, transaction_volume, revenue, supply, discount_rate, comparable_protocols |
| `analyze_defi` | DeFi yield analysis (farming, IL, staking, LP) | protocol_name, analysis_type, APR, principal, pool parameters |

### Municipal Bonds

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `price_muni_bond` | Municipal bond pricing with tax-equivalent yield | face_value, coupon_rate, bond_type, tax_bracket, call schedule |
| `analyze_municipal` | Municipal credit analysis (GO, revenue, scoring) | analysis_type, financial_data, debt_ratios, coverage metrics |

### Structured Products

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `price_structured_note` | Structured note pricing (capital-protected, yield enhancement) | note_type, face_value, maturity, underlying parameters |
| `price_exotic` | Exotic products (autocallable, barrier, digital) | product_type, underlying, barriers, observation schedule |

### Trade Finance

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `price_letter_of_credit` | LC pricing and risk assessment | lc_type, amount, tenor, issuing_bank, risk factors |
| `analyze_supply_chain_finance` | Supply chain finance (reverse factoring, forfaiting) | analysis_type, invoice parameters, discount rates |

### Onshore Fund Structures

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_us_fund_structure` | US onshore fund structure analysis (Delaware LP, LLC, REIT, MLP, BDC, QOZ) with tax analysis, ERISA compliance, investor suitability | structure_type, fund_size, strategy, investor_types, state, target_return, leverage_ratio, erisa_plan_assets_pct, qoz_investment_pct |
| `analyze_uk_eu_fund` | UK/EU onshore fund structure analysis (UK LP/LLP, OEIC, ACS, SICAV, FCP, KG) with AIFMD passport, VAT analysis, cross-border marketing | structure_type, domicile, fund_size, strategy, investor_types, aifmd_status, marketing_jurisdictions, vat_status |

### Offshore Fund Structures

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_cayman_structure` | Cayman/BVI offshore fund structure (Exempted LP, SPC, Unit Trust, BVI BCA) with master-feeder economics, CIMA registration, economic substance | structure_type, domicile, fund_size, strategy, master_feeder, feeder_jurisdictions, cima_category, economic_substance_activities |
| `analyze_lux_structure` | Luxembourg/Ireland fund structure (SICAV-SIF, RAIF, SCSp, ICAV, QIAIF, Section 110) with subscription tax, AIFMD passport, UCITS analysis | structure_type, domicile, fund_size, strategy, regulatory_status, subscription_tax_rate, aifmd_passport, ucits_compliant, target_investors |

### Offshore Fund Structures (Phase 23 Expansion)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_jersey_fund` | Jersey JPF/Expert/QIF, Guernsey PIF/QIF/RQIF, PCC/ICC cell company analysis with regulatory fees, substance requirements, and distribution capabilities | structure_type, domicile, fund_size, strategy, investor_types, cell_company, regulatory_category, substance_activities |
| `analyze_vcc_structure` | Singapore VCC standalone/umbrella, MAS licensing, S13O/S13U/S13D tax incentives, fund management company requirements | structure_type, fund_size, strategy, mas_license_type, tax_incentive, umbrella_sub_funds, fund_manager_type, investor_types |
| `analyze_ofc_structure` | Hong Kong OFC/LPF, unified fund exemption, carried interest concession, SFC licensing requirements | structure_type, fund_size, strategy, sfc_license, unified_exemption, carried_interest_concession, investor_types |
| `analyze_difc_fund` | DIFC/ADGM fund structures, sharia compliance, free zone economics, regulatory sandbox analysis | structure_type, domicile, fund_size, strategy, sharia_compliant, free_zone, regulatory_category, investor_types |
| `compare_jurisdictions` | 10+ jurisdiction side-by-side comparison with weighted scoring, optimal domicile selection, total cost of ownership, distribution passport mapping | jurisdictions, fund_size, strategy, investor_types, distribution_targets, priority_weights, comparison_horizon_years |
| `migration_feasibility` | Redomiciliation feasibility, cost-benefit NPV, tax consequences, migration corridors, regulatory approval timeline | source_jurisdiction, source_vehicle, target_jurisdiction, target_vehicle, fund_size, migration_driver, remaining_life_years |

### Transfer Pricing

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_beps_compliance` | OECD BEPS compliance analysis: CbCR reporting, Pillar Two GloBE 15% minimum tax, functional analysis, profit/substance alignment, risk scoring | entity_name, jurisdictions, revenue_by_jurisdiction, profit_by_jurisdiction, employees_by_jurisdiction, tangible_assets_by_jurisdiction, related_party_transactions, effective_tax_rates |
| `analyze_intercompany` | Transfer pricing analysis: CUP, RPM, CPLM, TNMM, Profit Split methods with arm's length range, CFC analysis (Subpart F/GILTI/ATAD), GAAR assessment | transaction_type, related_parties, transaction_value, pricing_method, comparable_data, functional_analysis, cfc_rules_applicable, jurisdiction_pair |

### Tax Treaty

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_treaty_network` | Tax treaty network analysis: WHT optimization, treaty conduit routing, LOB/PPT anti-avoidance scoring, entity-specific exemptions | source_jurisdiction, target_jurisdiction, income_type, entity_type, treaty_benefits_claimed, intermediary_jurisdictions, substance_indicators |
| `optimize_treaty_structure` | Multi-jurisdiction holding structure optimization: participation exemption, IP box, interest deduction limits, PE risk assessment, substance cost-benefit | parent_jurisdiction, operating_jurisdictions, holding_candidates, income_streams, ip_locations, debt_quantum, substance_requirements, annual_costs |

### FATCA/CRS

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_fatca_crs_reporting` | Analyze FATCA/CRS reporting obligations | institution, IGA model, account types, GIIN status |
| `classify_entity` | Classify entities under FATCA/CRS | entity type, income/asset ratios, controlling persons |

### Substance Requirements

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_economic_substance` | Score economic substance compliance | jurisdiction, entity type, employees, premises, CIGA |
| `run_jurisdiction_substance_test` | Run jurisdiction-specific substance tests | jurisdictions, comparison mode, treaty reliance |

### Regulatory Reporting

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `generate_aifmd_report` | Generate AIFMD Annex IV report | AUM, funds, leverage, stress tests, liquidity |
| `generate_sec_cftc_report` | Generate SEC Form PF / CFTC CPO-PQR | regulatory AUM, fund details, counterparties |

### AML Compliance

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `assess_kyc_risk` | Assess KYC/AML risk scoring | customer type, jurisdiction, PEP status, transactions |
| `screen_sanctions` | Screen against sanctions lists | entities, lists to check, threshold, transaction details |

### Fund of Funds

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `calculate_j_curve` | J-curve fund lifecycle: cash flow projection, TVPI/DPI/RVPI, PME (Kaplan-Schoar), net/gross IRR, trough analysis | fund_size, vintage_year, investment_period, fund_life, management_fee, carry_rate, hurdle, drawdown_schedule, exit_multiples, public_market_returns |
| `calculate_commitment_pacing` | Commitment pacing: vintage year allocation, drawdown modeling, NAV projection, over-commitment ratio | target_allocation, total_portfolio, vintage_commitments, drawdown_rates, distribution_rates, nav_growth, rebalancing_frequency |
| `analyze_manager_selection` | Manager due diligence: performance scoring, persistence analysis, alpha estimation, qualitative rating | manager_name, fund_returns, benchmark_returns, peer_quartiles, team_stability, strategy, operational_dd_scores |
| `calculate_secondaries_pricing` | Secondaries pricing: NAV discount, unfunded PV, IRR sensitivity at multiple exit multiples, breakeven | fund_nav, unfunded_commitment, remaining_life, expected_distributions, discount_rate, exit_multiple_scenarios |
| `analyze_fof_portfolio` | Fund of funds portfolio: diversification by strategy/vintage/geography, HHI, constraint monitoring | funds (name, strategy, vintage, geography, nav, commitment), constraints, rebalancing_targets |

### Bank Analytics

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_nim` | Net interest margin analysis: NIM calculation, rate/volume decomposition, asset/liability mix contribution, interest rate gap | interest_income, interest_expense, earning_assets, interest_bearing_liabilities, asset_mix, liability_mix, repricing_buckets, rate_scenarios |
| `calculate_camels_rating` | CAMELS bank rating: Capital adequacy, Asset quality, Management, Earnings, Liquidity, Sensitivity composite score (1-5) | cet1_ratio, tier1_ratio, total_capital_ratio, npl_ratio, provision_coverage, roa, roe, efficiency_ratio, lcr, nsfr, loan_to_deposit, rate_sensitivity, governance_scores |
| `calculate_cecl_provision` | CECL/IFRS 9 expected credit loss: multi-scenario weighted ECL by segment, stage classification, lifetime vs 12-month provision | loan_segments, pd_by_segment, lgd_by_segment, ead_by_segment, scenarios (base/upside/downside), scenario_weights, stage_classification, methodology (CECL/IFRS9) |
| `analyze_deposit_beta` | Deposit beta analysis: pass-through rate estimation, cumulative beta, asymmetry analysis (up vs down cycles), repricing lag | deposit_rates_history, policy_rates_history, deposit_types, observation_periods, cycle_direction |
| `analyze_loan_book` | Loan book analysis: sector/geography concentration (HHI), NPL analysis, provision adequacy, weighted average rate and maturity | loans (sector, geography, outstanding, rate, maturity, status, provision), benchmark_npl_ratios |

### Carbon Markets

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `price_carbon_credit` | Carbon credit pricing: forward price via cost-of-carry, vintage discount, registry premium, credit type adjustment | spot_price, risk_free_rate, tenor, vintage_year, registry, credit_type (compliance/voluntary), storage_cost, seasonal_factors |
| `analyze_ets_compliance` | ETS compliance analysis: allowance surplus/deficit, compliance cost, price volatility, carbon intensity vs benchmark | free_allocation, purchased_allowances, surrendered, verified_emissions, carbon_price_scenarios, sector_benchmark_intensity, revenue |
| `analyze_cbam` | EU CBAM analysis: certificate cost per good, net CBAM liability after origin carbon price credit, total exposure | goods (type, quantity, embedded_emissions), eu_ets_price, origin_carbon_prices, origin_jurisdictions, reporting_period |
| `value_carbon_offset` | Carbon offset valuation: quality-adjusted price, permanence/additionality/vintage/certification adjustments, co-benefit premium | base_price, permanence_score, additionality_score, vintage_year, certification_standard, co_benefits (social, biodiversity), project_type |
| `calculate_shadow_carbon_price` | Shadow carbon price analysis: carbon-adjusted NPV, abatement cost, project ranking with/without carbon pricing, breakeven carbon price | projects (name, npv, annual_emissions, abatement_cost), shadow_price_scenarios, discount_rate, time_horizon |

### Private Wealth

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `analyze_concentrated_stock` | Concentrated stock analysis: collar, exchange fund, prepaid forward, charitable strategies with tax-adjusted after-tax comparison | stock_position (shares, price, cost_basis), strategies (collar, exchange_fund, prepaid_forward, charitable), tax_rates, holding_period, volatility |
| `compare_philanthropic_vehicles` | Philanthropic vehicle comparison: CRT, CLT, DAF, private foundation with tax deduction, income stream, and remainder analysis | donation_amount, asset_type, tax_bracket, vehicles (CRT, CLT, DAF, foundation), discount_rate, payout_rate, term_years |
| `analyze_wealth_transfer` | Wealth transfer planning: estate tax, GST, annual exclusion, GRAT, grantor trust, dynasty trust, ILIT analysis with tax savings | estate_value, gift_amount, trust_types (GRAT, grantor, dynasty, ILIT), section_7520_rate, exemption_used, annual_exclusion_recipients, life_insurance_face |
| `analyze_direct_indexing` | Direct indexing analysis: tax-loss harvesting opportunities, wash sale compliance, tracking error, after-tax alpha estimation | portfolio_positions (ticker, shares, cost_basis, market_value, acquisition_date), target_index, tax_rates, wash_sale_window, rebalancing_frequency |
| `evaluate_family_governance` | Family governance evaluation: governance score, complexity assessment, structure recommendations, risk identification | family_members, entities (trusts, companies, foundations), jurisdictions, governance_practices, succession_plan, meeting_frequency, documentation_level |

---

## Response Envelope

Every tool returns this structure:

```json
{
  "result": { },
  "methodology": "DCF (FCFF, 2-stage)",
  "assumptions": { },
  "warnings": ["Terminal growth (3.5%) above long-term GDP"],
  "metadata": {
    "version": "0.1.0",
    "computation_time_us": 1200,
    "precision": "rust_decimal_128bit"
  }
}
```

Always check `warnings` — they flag suspicious inputs (beta > 3, ERP > 10%, WACC > 20%, too few comps, etc.).

---

## Tool Chaining Workflows

### Private Credit Analysis

1. `price_unitranche` — price unitranche with FOLO split
   - First-out/last-out economics, blended yield, borrower leverage metrics
2. `model_direct_loan` — model direct loan with PIK toggle and delayed draw
   - Amortisation schedules, rate floors, YTM via Newton-Raphson IRR, credit analytics
3. `analyze_syndication` — analyse loan syndication
   - Pro-rata scaling, arranger economics, participant allocations

### Insurance & Actuarial Analysis

1. `estimate_reserves` — estimate IBNR reserves
   - Chain-ladder: volume-weighted age-to-age factors, cumulative development to ultimate
   - Bornhuetter-Ferguson: blends a priori ELR with development for immature years
2. `price_premium` — price insurance premium
   - Frequency x severity, trend projections, expense/profit loadings
3. `analyze_combined_ratio` — analyse underwriting profitability
   - Loss ratio, expense ratio, combined ratio, operating ratio (with investment income)
4. `calculate_scr` — compute Solvency II capital requirement
   - Premium/reserve risk, operational risk, diversification benefit, MCR floor

### FP&A Analysis

1. `analyze_variance` — analyse budget vs actual
   - Revenue: price/volume/mix decomposition (always sum to total)
   - Cost: favorable/unfavorable by line item
   - YoY comparison with margin expansion in bps
2. `analyze_breakeven` — compute break-even point
   - Contribution margin, break-even units/revenue, DOL, target volume
   - Scenario analysis with price/cost changes
3. `analyze_working_capital` — analyse working capital efficiency
   - DSO, DIO, DPO, cash conversion cycle, NWC as % of revenue
   - Trend analysis, optimisation recommendations, peer benchmarking
4. `build_rolling_forecast` — build driver-based rolling forecast
   - Revenue compounding, COGS/OpEx ratios, FCF projection, CAGR

### Wealth Management

1. `plan_retirement` — plan retirement with accumulation and decumulation phases
   - 4 withdrawal strategies: Constant Dollar, Constant Percentage, Guardrails, RMD
   - Savings gap analysis, real vs nominal values, legacy projection
2. `simulate_tax_loss_harvesting` — simulate TLH opportunities
   - Candidate identification, ST/LT classification, wash-sale 30-day rule
   - Tax savings from offsetting gains, carry-forward of excess losses
3. `plan_estate` — analyse estate tax and planning strategies
   - Annual exclusion gifts, lifetime exemption usage, 7 trust types
   - Federal/state estate tax, GST tax on skip-person gifts, ILIT exclusion

### Restructuring & Distressed Debt

1. `analyze_recovery` — Absolute Priority Rule (APR) waterfall
   - Claim classes: DIP, admin, secured (1st/2nd lien), senior, sub, mezzanine, equity
   - Collateral deficiency -> unsecured deficiency claim
   - Fulcrum security identification, going-concern vs liquidation analysis
2. `analyze_distressed_debt` — restructuring plan analysis
   - Treatment types: reinstate, amend, exchange, equity conversion, cash paydown
   - Fulcrum identification with mispricing detection
   - IRR at market price, credit bid value, DIP analysis

### Real Assets

1. `value_property` — real estate valuation
   - Direct capitalisation (NOI / cap rate), DCF with exit cap rate, GRM from comparables
   - Leveraged returns: mortgage amortisation, DSCR, cash-on-cash, equity multiple, levered IRR
2. `model_project_finance` — infrastructure project finance
   - Construction + operating phases with debt sculpting (level/sculpted/bullet)
   - DSCR, LLCR, PLCR coverage ratios
   - Distribution waterfall: CFADS -> senior -> sub -> DSRA -> equity

### Venture Capital Analysis

1. `analyze_dilution` — model round-by-round dilution with option pool shuffle
   - Pre-money/post-money, option pool created pre-money (dilutes founders, not investor)
2. `convert_note` — analyse SAFE or convertible note conversion
   - Cap vs discount (take the more favorable), accrued interest, MFN provisions
3. `model_venture_fund` — analyse fund performance
   - J-curve, TVPI/DPI/RVPI, carry above hurdle, loss ratio, portfolio concentration

### ESG & Climate Analysis

1. `calculate_esg_score` — compute ESG score with sector-specific materiality
   - 9 sector-specific weighting schemes, 7-level rating bands (AAA->CCC)
2. `analyze_carbon_footprint` — analyse Scope 1/2/3 emissions and intensity
3. `analyze_green_bond` — assess green bond framework alignment
4. `test_sll_covenants` — test sustainability-linked loan KPI performance

### Regulatory Capital Analysis

1. `calculate_regulatory_capital` — compute Basel III capital ratios
   - Standardised approach risk weights, operational risk (BIA/SA), CRM via collateral
2. `calculate_lcr` — compute liquidity ratios
   - LCR: HQLA with L2 cap (40%), L2B cap (15%), inflow cap (75%)
   - NSFR: ASF/RSF factors by category
3. `analyze_alm` — asset-liability management
   - Repricing/maturity gap analysis, NII sensitivity, EVE duration of equity

### Compliance Analysis

1. `analyze_best_execution` — MiFID II best execution assessment
   - Perold implementation shortfall: decision price vs execution price decomposition
   - Delay cost: market drift between decision and execution
   - Market impact: price movement caused by order execution
   - Timing cost: explicit + implicit cost breakdown
   - Execution quality score: composite rating vs benchmark
   - Venue analysis: execution quality comparison across venues
2. `generate_gips_report` — GIPS-compliant performance reporting
   - Modified Dietz: time-weighted return with cash flow weighting
   - Geometric linking: chain-link sub-period returns for composite periods
   - Composite dispersion: asset-weighted standard deviation across accounts
   - Risk statistics: Sharpe ratio, Information ratio, tracking error
   - GIPS compliance checklist: mandatory disclosure items verification
   - Annualization: geometric annualization for periods > 1 year
3. **Key benchmarks**:
   - Implementation shortfall < 25bp: good execution quality
   - Market impact < 10bp: low-impact execution
   - GIPS requires 5+ years of history (or since inception if shorter)
   - Composite dispersion < 200bp: consistent management across accounts

### Credit Derivatives Analysis

1. `price_cds` — price a CDS with discrete hazard-rate model
   - Survival probabilities, risky PV01, premium/protection leg PVs
   - Breakeven spread, DV01, jump-to-default exposure
   - Mark-to-market with market spread vs contract spread
2. `calculate_cva` — compute CVA/DVA for counterparty risk
   - Unilateral CVA (counterparty only) and bilateral CVA (CVA - DVA)
   - Netting benefit reduces gross exposure; collateral threshold caps remaining
   - CVA as running spread in basis points

### Convertible Bond Analysis

1. `price_convertible` — price convertible with CRR binomial tree
   - Bond floor (straight debt value), conversion value (stock x ratio)
   - Conversion premium, investment premium, embedded option value
   - Greeks: delta (stock sensitivity), gamma, vega (vol sensitivity), theta (time decay)
   - Call/put provisions: callable CB capped at call price, puttable CB floored at put price
2. `analyze_convertible` — scenario analysis for convertibles
   - Stock sensitivity: price across range of stock prices
   - Vol sensitivity: value changes with volatility
   - Spread sensitivity: credit spread impact
   - Forced conversion analysis: in-the-money call trigger
   - Income advantage: bond yield vs stock dividend with breakeven years

### Lease Accounting Analysis

1. `classify_lease` — classify under ASC 842 or IFRS 16
   - Five-test classification: ownership transfer, purchase option, specialized asset, 75% economic life, 90% fair value
   - Finance lease: effective interest method for liability, separate depreciation for ROU
   - Operating lease (ASC 842): single straight-line expense
   - IFRS 16: all leases treated as finance (no operating classification for lessee)
   - Full amortization schedule with monthly detail
2. `analyze_sale_leaseback` — analyse sale-leaseback transactions
   - Qualifying sale: gain/loss with retained right ratio adjustment
   - Failed sale: financing obligation treatment (asset stays on books)
   - Above-FMV: excess deferred as financing component

### Pension & LDI Analysis

1. `analyze_pension_funding` — comprehensive DB pension analysis
   - PBO (projected with salary growth) vs ABO (current salaries)
   - Unit credit method with discount factor and salary projection
   - Funded status (assets - PBO), funding ratio (assets / PBO)
   - Service cost, interest cost, expected return on assets, NPPC
   - Minimum required contribution and maximum deductible
   - Liability by age cohort with duration estimates
2. `design_ldi_strategy` — design liability-driven investing strategy
   - Duration gap analysis: asset duration vs liability-weighted duration
   - Hedging portfolio construction: instrument selection to match liability duration
   - Immunization assessment: duration + convexity matching
   - Surplus-at-risk: P&L impact from 1% rate shift
   - Glide-path schedule: transition from growth to hedging allocation

### Sovereign Risk Analysis

1. `analyze_sovereign_bond` — analyse sovereign bonds
   - Pricing with sovereign spread decomposition (credit, liquidity, FX risk)
   - YTM, duration, convexity for sovereign securities
   - Local currency risk premium: inflation differential, FX volatility adjustment
   - Cross-currency comparison: USD, EUR, GBP, EM local currency
2. `assess_country_risk` — assess sovereign/country risk
   - 12-factor scoring model: GDP growth, inflation, fiscal balance, debt/GDP, current account, FX reserves, political stability, rule of law, external debt, short-term debt/reserves, default history, dollarization
   - Implied credit rating equivalent from composite score
   - Country risk premium (CRP) for use in WACC calculations
   - Default probability estimation from sovereign CDS-equivalent spreads
3. **Key benchmarks**:
   - AAA sovereign: debt/GDP < 60%, reserves > 6 months imports, no default history
   - EM investment grade: fiscal deficit < 3%, current account deficit < 4%
   - CRP: 0bp (AAA) to 800bp+ (distressed sovereign)

### Real Options Analysis

1. `value_real_option` — value real options via CRR binomial tree
   - 6 option types: Expand (scale up), Abandon (exit), Defer (wait), Switch (change mode), Contract (scale down), Compound (option on option)
   - Greeks: delta (sensitivity to underlying), gamma, vega (vol sensitivity), theta (time decay)
   - Expansion factor: underlying x factor if exercised; contraction factor: underlying x factor if contracted
   - Switch cost: cost to switch operating mode; switch value ratio: new mode value as ratio of current
2. `analyze_decision_tree` — decision tree with EMV rollback
   - Node types: Decision (choose best child), Chance (probability-weighted), Terminal (payoff)
   - Expected Monetary Value (EMV) rollback from terminal nodes to root
   - EVPI (Expected Value of Perfect Information): value of eliminating uncertainty
   - Sensitivity analysis on key probabilities
   - Optimal path identification through the tree
3. **Key benchmarks**:
   - Real option premium typically 10-30% above static NPV for volatile projects
   - Defer option most valuable when uncertainty high and irreversibility high
   - EVPI > 20% of EMV suggests high value in additional market research

### Equity Research Analysis

1. `calculate_sotp` — sum-of-the-parts valuation
   - 6 valuation methods per segment: EV/EBITDA, P/E, EV/Revenue, EV/EBIT, DCF, NAV-Based
   - Conglomerate discount: holding company discount on total enterprise value
   - Football field: min/base/max range from comparable multiple ranges
   - Per-share equity value: (total EV - net debt - minorities + unconsolidated) / shares
2. `calculate_target_price` — multi-method target price derivation
   - PE, PEG, P/B, P/S, DDM (dividend discount model) valuations simultaneously
   - Peer-relative: median and mean of peer multiples for each method
   - Football field: visual range of all method-derived target prices
   - Recommendation: Strong Buy / Buy / Hold / Sell / Strong Sell based on upside/downside
   - Analyst consensus: incorporate external analyst targets if available
3. **Key benchmarks**:
   - Conglomerate discount: typically 10-25% for diversified companies
   - SOTP unlocks value when market undervalues high-growth segments
   - Target price spread > 30% across methods = high uncertainty

### Commodity Trading Analysis

1. `analyze_commodity_spread` — analyse commodity processing/calendar/location spreads
   - 6 spread types: Crack (oil->products), Crush (soy->meal+oil), Spark (gas->power), Calendar (near vs far), Location (basis), Quality (grade differential)
   - Gross processing margin: output revenue - input cost - processing cost
   - Historical spread analysis: mean, standard deviation, z-score, percentile
   - Risk metrics: VaR, margin at risk, worst-case loss
2. `analyze_storage_economics` — commodity storage and carry analysis
   - Contango/backwardation decomposition across term structure
   - Implied convenience yield at each tenor
   - Cash-and-carry arbitrage: buy spot + store + sell forward; net profit = spread - carry cost
   - Seasonal factors: injection/withdrawal patterns (natural gas, agricultural)
   - Storage capacity utilization and injection/withdrawal rate constraints
3. **Key benchmarks**:
   - Crack spread (3-2-1): typical $5-20/bbl; negative signals refinery distress
   - Calendar spread z-score > 2: potential mean-reversion opportunity
   - Storage full-carry = spot + finance + storage + insurance; anything above = super-contango

### Treasury Management Analysis

1. `analyze_cash_management` — corporate treasury cash management
   - Month-by-month cash flow simulation over 12-month horizon
   - Sweep logic: excess above threshold invested at money-market rate
   - Facility draw: shortfall below minimum buffer drawn from revolving credit
   - Cash conversion cycle: DSO + DIO - DPO (overall efficiency measure)
   - Liquidity scoring: weighted assessment of cash buffer, facility headroom, CCC
   - Investment income from surplus cash, interest expense from facility draws
2. `analyze_hedging` — hedge accounting effectiveness testing
   - Dollar offset method: hedge change / exposure change (IAS 39: 80-125% range)
   - OLS regression: R-squared and slope for retrospective assessment (IFRS 9: R-squared > 0.80)
   - IAS 39 compliance: both dollar offset within 80-125% AND R-squared > 0.80
   - IFRS 9 compliance: qualitative + quantitative (R-squared > 0.80 sufficient)
   - VaR impact: hedged vs unhedged VaR at specified confidence level
   - Inverse normal via Abramowitz & Stegun approximation for VaR quantile
3. **Key benchmarks**:
   - Minimum cash buffer: typically 2-3 months operating expenses
   - CCC < 30 days: excellent; 30-60: good; > 90: needs improvement
   - Hedge ratio > 0.95 and R-squared > 0.90 = highly effective hedge

### Infrastructure Finance Analysis

1. `model_ppp` — public-private partnership financial model
   - 3 revenue models: Availability Payment (government pays), Demand-Based (tolls), Mixed
   - Year-by-year projection: revenue, opex, EBITDA, debt service, CFADS, equity distributions
   - Coverage ratios: DSCR (annual), LLCR (loan-life), PLCR (project-life)
   - Value for Money (VfM) score: PPP cost vs Public Sector Comparator
   - Risk allocation matrix: construction, demand, availability, maintenance, financing
   - Equity IRR via Newton-Raphson, project NPV at WACC
2. `value_concession` — infrastructure concession valuation
   - Year-by-year projections through remaining concession life
   - Handback cost provisioning in final years before concession end
   - Extension option value: probability-weighted additional cash flows
   - Equity IRR, project NPV, coverage ratios
   - Comparable metrics: EV/EBITDA, EV/capacity, EV/traffic
3. **Key benchmarks**:
   - Target equity IRR: 12-18% for infrastructure PPP
   - Minimum DSCR: 1.20x (availability), 1.30x (demand-based)
   - LLCR > 1.40x for investment-grade project finance
   - VfM > 10% typically justifies PPP over traditional procurement

### Fund Structuring Analysis

1. `analyze_us_fund_structure` — analyse US onshore structures
   - Delaware LP, LLC (Series), REIT, MLP, BDC, QOZ vehicle types
   - Tax analysis: pass-through vs entity-level taxation, UBTI exposure, state tax nexus
   - ERISA compliance: 25% plan assets test, VCOC/REOC operating company exemptions
   - Investor suitability: taxable, tax-exempt, non-US, sovereign wealth fund
2. `analyze_uk_eu_fund` — analyse UK/EU onshore structures
   - UK LP, LLP, OEIC, ACS; EU SICAV, FCP, KG
   - AIFMD passport analysis: marketing permissions across EU/EEA
   - VAT analysis: management fee exemption, sub-advisory VAT treatment
   - Cross-border marketing: NPPR vs passport, reverse solicitation risks
3. `analyze_cayman_structure` — analyse Cayman/BVI offshore structures
   - Exempted LP, SPC (segregated portfolio), Unit Trust, BVI BCA
   - Master-feeder economics: tax-efficiency for mixed investor base
   - CIMA registration categories: mutual fund, private fund, exempted
   - Economic substance requirements: directed and managed test, CIGA activities
4. `analyze_lux_structure` — analyse Luxembourg/Ireland structures
   - Luxembourg: SICAV-SIF, RAIF, SCSp; Ireland: ICAV, QIAIF, Section 110
   - Subscription tax: 0.01% (institutional SIF) vs 0.05% (retail)
   - AIFMD passport: full-scope AIFM cross-border marketing
   - UCITS analysis: eligible assets, diversification, leverage limits
5. **Key benchmarks**:
   - Delaware LP: most common US PE/VC structure, no entity-level tax
   - Cayman Exempted LP: dominant offshore hedge fund vehicle
   - Luxembourg RAIF: fastest setup (no CSSF approval), AIFMD passport via AIFM
   - Ireland ICAV: preferred for US-facing strategies (check-the-box eligible)

### Transfer Pricing Analysis

1. `analyze_beps_compliance` — OECD BEPS/Pillar Two compliance
   - CbCR (Country-by-Country Reporting): revenue, profit, tax, employees per jurisdiction
   - Pillar Two GloBE: 15% minimum effective tax rate, top-up tax calculation
   - Functional analysis: functions performed, assets used, risks assumed per entity
   - Profit/substance alignment: profit vs economic activity indicators
   - Risk scoring: low/medium/high BEPS exposure rating
2. `analyze_intercompany` — transfer pricing method selection and analysis
   - 5 OECD methods: CUP, RPM, CPLM, TNMM, Profit Split
   - Arm's length range: interquartile range from comparable data
   - CFC analysis: Subpart F (US), GILTI (US), ATAD (EU) rule application
   - GAAR assessment: general anti-avoidance rule risk factors
   - Documentation requirements: master file, local file, CbCR thresholds
3. **Key benchmarks**:
   - Pillar Two GloBE: 15% minimum ETR (effective 2024 for large MNEs)
   - CbCR threshold: EUR 750M consolidated revenue
   - TNMM: most commonly used method for routine service/distribution entities
   - Arm's length range: interquartile (25th-75th percentile) is OECD standard

### Tax Treaty Optimization

1. `analyze_treaty_network` — analyse treaty network and WHT optimization
   - WHT rate matrix: dividend, interest, royalty rates by treaty pair
   - Conduit routing: identify optimal intermediary jurisdictions
   - LOB (Limitation on Benefits): qualified person, active trade/business, derivative benefits tests
   - PPT (Principal Purpose Test): anti-avoidance scoring under MLI
   - Entity-specific exemptions: pension funds, sovereign wealth, charities
2. `optimize_treaty_structure` — multi-jurisdiction holding structure design
   - Participation exemption: dividend/capital gains exemption thresholds per jurisdiction
   - IP box regimes: reduced rates for qualifying IP income (nexus approach)
   - Interest deduction limits: EBITDA-based caps (30% ATAD, 163(j) US)
   - Permanent establishment risk: activity thresholds, dependent agent PE
   - Substance cost-benefit: director fees, office costs, local employees vs tax savings
3. **Key benchmarks**:
   - Netherlands: 0% participation exemption on dividends and capital gains (5% holding)
   - Luxembourg: 0% participation exemption (10% holding or EUR 1.2M cost)
   - Ireland: 12.5% trading rate, IP box 6.25% (being replaced by Pillar Two)
   - Singapore: 0-5% WHT on dividends (most treaties), no capital gains tax
   - Interest deduction cap: 30% EBITDA is the global standard (BEPS Action 4)

### FATCA/CRS Compliance Workflow

1. **FATCA reporting assessment**: call `analyze_fatca_crs_reporting` with institution and account data
   - IGA model determines reporting path: Model 1 (via local authority), Model 2 (direct to IRS), Non-IGA (30% withholding risk)
   - GIIN registration: mandatory for FFIs, critical for compliance scoring
   - US indicia: birthplace, address, phone, standing instructions, POA
   - Reporting thresholds: $50k individuals, $250k entities
   - CRS: wider vs narrower approach, due diligence by balance
2. **Entity classification**: call `classify_entity` with entity details
   - FATCA: FFI > DeemedCompliant > ExemptBeneficialOwner > ActiveNFFE > PassiveNFFE
   - CRS: FinancialInstitution > ActiveNFE > PassiveNFE
   - Passive test: >=50% passive income OR >=50% passive assets
   - Controlling persons: >=25% ownership threshold for passive entities
3. **Key benchmarks**: Non-IGA 30% withholding; FATCA compliance score >80 = low risk; CRS 100+ jurisdictions; entity classification drives documentation burden

### Economic Substance Workflow

1. **Substance analysis**: call `analyze_economic_substance` with entity and jurisdiction data
   - 5-dimension scoring (0-100): personnel (25), premises (20), decision-making (25), expenditure (15), CIGA (15)
   - Cayman/BVI ES Act: CIGA must be in-jurisdiction, IP holding = highest substance bar
   - Luxembourg: no specific law but TP/ATAD substance required
   - Ireland: central management and control test
   - Penalties: CI$10k year 1, CI$100k year 2, strike-off year 3
2. **Jurisdiction comparison**: call `run_jurisdiction_substance_test` with multiple jurisdictions
   - Compare substance costs vs tax savings across jurisdictions
   - Net benefit = tax savings - substance cost, payback ratio analysis
   - Treaty reliance amplifies risk by 30%
3. **Key benchmarks**: substance score >70 = compliant; annual substance cost EUR 50-150k; typical payback <2 years for well-structured holdings

### Regulatory Reporting Workflow

1. **AIFMD reporting**: call `generate_aifmd_report` with fund and AIFM data
   - Frequency: quarterly (>=EUR 1B), semi-annual (>=EUR 100M), annual (<EUR 100M)
   - Leverage: gross (no netting) vs commitment (hedging allowed); >3x triggers enhanced reporting
   - Stress tests: equity -30%, rates +250bps, FX -20%, credit spreads +400bps
   - Liquidity profile: 7 time buckets (1d through >365d)
2. **SEC/CFTC reporting**: call `generate_sec_cftc_report` with adviser and fund data
   - Form PF: large (>$1.5B quarterly), small (>$150M annual), exempt (<$150M)
   - Sections 1-4: all advisers (S1), large HF (S2), large liquidity (S3), large PE (S4)
   - CFTC CPO-PQR: large (>$1.5B or pool >$500M), small (below thresholds)
   - Counterparty concentration via HHI
3. **Key benchmarks**: AIFMD leverage >3x = enhanced; Form PF $1.5B = quarterly; filing deadlines 60d (large quarterly) / 120d (small annual)

### AML/KYC Compliance Workflow

1. **KYC risk assessment**: call `assess_kyc_risk` with customer and transaction data
   - FATF 5-dimension scoring (0-100): customer type (25), geographic (25), product (20), transaction (15), source of wealth (15)
   - PEP categories: domestic, foreign, international org, family, close associate
   - Due diligence: SDD (low risk), CDD (standard), EDD (PEPs, high-risk jurisdictions)
   - Red flags: shell company indicators, structuring, jurisdiction mismatch, adverse media
2. **Sanctions screening**: call `screen_sanctions` with entities and list selection
   - Fuzzy matching via Levenshtein distance (0-100 score)
   - Lists: OFAC SDN, EU Consolidated, HMT UK, UN UNSC, FATF grey/black
   - Match types: exact (100), strong (>90), possible (70-90), weak (50-70), no match (<50)
   - Country risk: comprehensive embargo, sectoral sanctions, FATF monitoring
3. **Key benchmarks**: risk score >70 = EDD required; PEP always EDD; match score >70 = manual review; SAR filing 24h (terrorism) / 30d (other)

### Fund of Funds Analysis

1. `calculate_j_curve` — model PE fund lifecycle cash flows
   - Cash flow projection by vintage year
   - TVPI/DPI/RVPI multiples, PME (Kaplan-Schoar) vs public market
   - J-curve trough: typically year 3-4 for PE, year 2-3 for VC
   - Net vs gross IRR spread
2. `calculate_commitment_pacing` — plan vintage year allocation
   - Drawdown modelling: pace new commitments to smooth exposure
   - NAV projection: forecast portfolio value over time
   - Over-commitment ratio: total commitments / target allocation (typically 1.3-1.6x)
3. `analyze_manager_selection` — evaluate and score fund managers
   - Performance: quartile ranking, persistence, alpha vs benchmark
   - Qualitative: strategy, team stability, operational due diligence
4. `calculate_secondaries_pricing` — value secondary market transactions
   - NAV discount/premium, unfunded commitment PV
   - IRR sensitivity at different exit multiples
   - Breakeven analysis
5. `analyze_fof_portfolio` — portfolio-level diversification analysis
   - By strategy, vintage, geography, sector
   - HHI concentration, constraint monitoring
6. **Key benchmarks**: PE J-curve trough year 3-4; top-quartile PE TVPI > 2.0x; over-commitment ratio 1.3-1.6x; secondaries NAV discount 5-15% (2024 market); FoF management fee 0.5-1.0% on top of underlying GP fees

### Bank Analytics Workflow

1. `analyze_nim` — analyse bank profitability driver
   - NIM = (interest income - interest expense) / average earning assets
   - Rate/volume decomposition: separate impact of rate changes vs balance changes
   - Asset/liability mix contribution: which products drive NIM?
   - Interest rate gap by repricing bucket
2. `calculate_camels_rating` — composite bank health assessment
   - Capital adequacy (C): CET1, tier 1, total capital ratios
   - Asset quality (A): NPL ratio, provision coverage, write-off rate
   - Management (M): governance, risk management, strategic planning scores
   - Earnings (E): ROA, ROE, efficiency ratio, core earnings stability
   - Liquidity (L): LCR, NSFR, loan-to-deposit ratio
   - Sensitivity (S): interest rate risk, FX exposure, equity risk
   - Composite 1-5 (1=strong, 5=critically deficient)
3. `calculate_cecl_provision` — calculate expected credit losses
   - Multi-scenario weighted ECL: base/upside/downside with probability weights
   - Stage classification: Stage 1 (performing, 12-month ECL), Stage 2 (significant increase in credit risk, lifetime ECL), Stage 3 (credit-impaired, lifetime ECL)
   - CECL (US) vs IFRS 9 (international) approach
4. `analyze_deposit_beta` — analyse deposit repricing behavior
   - Pass-through rate: how much of rate hike reaches depositors
   - Cumulative beta: total deposit rate change / total policy rate change
   - Asymmetry: deposit rates rise slower than they fall (up beta < down beta)
   - Repricing lag in months
5. `analyze_loan_book` — assess loan portfolio quality
   - Sector/geography concentration (HHI)
   - NPL by segment, provision adequacy, WAR (weighted average rate), WAM (weighted average maturity)
6. **Key benchmarks**: NIM 2.5-3.5% (commercial banks); CAMELS 1-2 = well-capitalized; CECL day-1 impact +20-40% vs incurred loss; deposit beta 40-60% in rate hike cycles; NPL ratio < 2% = healthy

### Carbon Markets Workflow

1. `price_carbon_credit` — price carbon credits
   - Forward pricing via cost-of-carry: F = S * (1+r)^T
   - Vintage discount: older credits trade at discount (5-15% per year)
   - Registry premium: Gold Standard, VCS, ACR pricing differentials
   - Credit type: compliance (EU ETS) vs voluntary
2. `analyze_ets_compliance` — assess ETS position
   - Allowance surplus/deficit: free allocation + purchases - surrendered
   - Compliance cost projection at multiple carbon price scenarios
   - Carbon intensity vs sector benchmark
3. `analyze_cbam` — EU carbon border adjustment
   - Certificate cost: embedded emissions * EU ETS price
   - Credit for carbon price already paid at origin
   - Net CBAM liability = EU certificate cost - origin carbon credit
4. `value_carbon_offset` — value carbon offsets with quality adjustments
   - Quality-adjusted price based on permanence, additionality, vintage, certification
   - Co-benefit premium for social/biodiversity co-benefits
5. `calculate_shadow_carbon_price` — internal carbon pricing
   - Carbon-adjusted NPV: base NPV - PV of carbon cost at shadow price
   - Abatement cost curve: marginal cost of emission reduction
   - Breakeven carbon price: price at which project NPV = 0
6. **Key benchmarks**: EU ETS price EUR 60-100/tCO2; voluntary market EUR 5-50/tCO2; CBAM phases in 2026-2034; shadow carbon price $50-100 (corporate best practice); vintage discount 5-15%/year

### Private Wealth Workflow

1. `analyze_concentrated_stock` — manage single-stock risk
   - Hedging strategies: costless collar (cap upside, floor downside), prepaid forward (monetize without sale), exchange fund (diversify tax-free)
   - Charitable strategies: donor-advised fund, charitable remainder trust
   - Tax-adjusted after-tax comparison across strategies
2. `compare_philanthropic_vehicles` — compare giving vehicles
   - CRT (Charitable Remainder Trust): income stream to donor, remainder to charity
   - CLT (Charitable Lead Trust): income to charity, remainder to heirs
   - DAF (Donor-Advised Fund): immediate deduction, grant over time
   - Private foundation: maximum control, 5% minimum distribution
   - Compare: tax deduction, income stream, administrative cost, control
3. `analyze_wealth_transfer` — estate and gift tax planning
   - Federal estate tax, GST tax, annual exclusion gifting
   - GRAT (Grantor Retained Annuity Trust): transfer appreciation above hurdle rate
   - Dynasty trust: multi-generational tax-exempt wealth transfer
   - ILIT: life insurance outside estate
4. `analyze_direct_indexing` — tax-efficient portfolio management
   - Tax-loss harvesting opportunities across individual stock positions
   - Wash sale compliance: 30-day rule monitoring
   - Tracking error vs target index
   - After-tax alpha: TLH benefit in basis points
5. `evaluate_family_governance` — evaluate family office governance
   - Governance score across 5 dimensions
   - Complexity assessment: number of entities, jurisdictions, family members
   - Structure recommendations and risk identification
6. **Key benchmarks**: concentrated stock > 10% of NW = significant risk; GRAT annuity rate = Section 7520 rate + 1-2%; TLH adds 50-150bps annually; family governance score > 70 = well-governed; DAF minimum $5-25k initial contribution

---

## CLI Equivalent

The same calculations are available via the `cfa` binary:

```bash
cfa unitranche --input unitranche.json --output table

cfa direct-lending --input loan.json --output table

cfa syndication --input syndication.json --output json

cfa loss-reserving --input triangle.json --output table

cfa premium-pricing --input premium.json --output json

cfa combined-ratio --input ratio.json --output table

cfa solvency-scr --input scr.json --output table

cfa variance --input variance.json --output table

cfa breakeven --input breakeven.json --output json

cfa working-capital --input wc.json --output table

cfa rolling-forecast --input forecast.json --output table

cfa retirement --input retirement.json --output table

cfa tax-loss-harvest --input tlh.json --output json

cfa estate-plan --input estate.json --output table

cfa recovery --input recovery.json --output table

cfa distressed-debt --input distressed.json --output table

cfa property-valuation --input property.json --output table

cfa project-finance --input project.json --output table

cfa venture-dilution --input rounds.json --output table

cfa convertible-instrument --input safe.json --output json

cfa venture-fund-returns --input fund.json --output table

cfa esg-score --input esg.json --output table

cfa carbon-footprint --input carbon.json --output json

cfa green-bond --input green.json --output json

cfa sll-covenants --input sll.json --output table

cfa basel-capital --input capital.json --output table

cfa lcr-nsfr --input liquidity.json --output table

cfa alm --input alm.json --output table

cfa best-execution --input execution.json --output table

cfa gips-report --input gips.json --output json

cfa cds-pricing --input cds.json --output table

cfa cva-calculation --input cva.json --output json

cfa convertible-pricing --input cb.json --output table

cfa convertible-analysis --input cb_analysis.json --output json

cfa lease-classification --input lease.json --output table

cfa sale-leaseback --input slb.json --output json

cfa pension-funding --input pension.json --output table

cfa ldi-strategy --input ldi.json --output json

cfa sovereign-bond --input sovereign.json --output table

cfa country-risk --input country.json --output json

cfa real-option --input option.json --output table

cfa decision-tree --input tree.json --output json

cfa sotp --input sotp.json --output table

cfa target-price --input equity.json --output json

cfa commodity-spread --input spread.json --output table

cfa storage-economics --input storage.json --output json

cfa cash-management --input treasury.json --output table

cfa hedge-effectiveness --input hedge.json --output json

cfa ppp-model --input ppp.json --output table

cfa concession --input concession.json --output json

cfa token-valuation --input token.json --output json

cfa defi-analysis --input defi.json --output json

cfa muni-bond --input muni.json --output table

cfa muni-analysis --input muni_analysis.json --output table

cfa structured-note --input note.json --output json

cfa exotic-product --input exotic.json --output json

cfa letter-of-credit --input lc.json --output table

cfa supply-chain-finance --input scf.json --output json

cfa us-fund-structure --input us_fund.json --output table

cfa uk-eu-fund-structure --input uk_eu_fund.json --output table

cfa cayman-fund-structure --input cayman_fund.json --output table

cfa lux-ireland-fund-structure --input lux_ireland_fund.json --output table

cfa beps-compliance --input beps.json --output table

cfa intercompany-pricing --input tp.json --output json

cfa treaty-network --input treaty.json --output table

cfa treaty-structure-optimization --input structure.json --output json

cfa fatca-crs-reporting --input fatca.json --output json

cfa entity-classification --input entity.json --output json

cfa economic-substance --input substance.json --output table

cfa jurisdiction-substance-test --input jurisdictions.json --output table

cfa aifmd-reporting --input aifmd.json --output json

cfa sec-cftc-reporting --input sec.json --output json

cfa kyc-risk-assessment --input kyc.json --output table

cfa sanctions-screening --input screening.json --output json

cfa j-curve --input jcurve.json --output table

cfa commitment-pacing --input pacing.json --output json

cfa manager-selection --input manager.json --output table

cfa secondaries-pricing --input secondaries.json --output json

cfa fof-portfolio --input fof.json --output table

cfa nim-analysis --input nim.json --output table

cfa camels-rating --input camels.json --output json

cfa cecl-provisioning --input cecl.json --output table

cfa deposit-beta --input beta.json --output json

cfa loan-book --input loanbook.json --output table

cfa carbon-pricing --input carbon.json --output table

cfa ets-compliance --input ets.json --output json

cfa cbam --input cbam.json --output table

cfa offset-valuation --input offset.json --output json

cfa shadow-carbon --input shadow.json --output table

cfa concentrated-stock --input stock.json --output table

cfa philanthropic-vehicles --input philanthropy.json --output json

cfa wealth-transfer --input transfer.json --output table

cfa direct-indexing --input indexing.json --output json

cfa family-governance --input governance.json --output table
```

Output formats: `--output json` (default), `--output table`, `--output csv`, `--output minimal`.

Pipe support: `cat data.json | cfa credit-metrics --output table`

---

## Input Conventions

- **Rates as decimals**: 5% = `0.05`, never `5`
- **Money as raw numbers**: $1M = `1000000`, not `"$1M"`
- **Currency**: specify with `currency` field (default: USD)
- **Dates**: ISO 8601 format (`"2026-01-15"`)
- **Weights must sum to 1.0**: `debt_weight + equity_weight = 1.0`

## Error Handling

Tools return structured errors for:
- **InvalidInput**: field-level validation (e.g., negative beta, weights not summing to 1.0)
- **FinancialImpossibility**: terminal growth >= WACC, negative enterprise value
- **ConvergenceFailure**: IRR/XIRR Newton-Raphson didn't converge (reports iterations and last delta)
- **InsufficientData**: too few data points for statistical calculations
- **DivisionByZero**: zero interest expense for coverage ratios, etc.

Always validate tool error responses and report them clearly to the user.
