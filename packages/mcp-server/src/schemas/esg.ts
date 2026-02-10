import { z } from "zod";

export const EsgScoreSchema = z.object({
  company_name: z.string().describe("Company name"),
  sector: z.string().describe("Industry sector"),
  environmental: z.object({
    carbon_intensity: z.coerce.number().min(0).describe("tCO2e per $M revenue"),
    renewable_energy_pct: z.coerce.number().min(0).max(1).describe("Renewable energy percentage (0-1)"),
    water_intensity: z.coerce.number().min(0).describe("Megalitres per $M revenue"),
    waste_recycling_rate: z.coerce.number().min(0).max(1).describe("Waste recycling rate (0-1)"),
    biodiversity_policy: z.coerce.boolean().describe("Has biodiversity policy"),
    environmental_fines_amount: z.coerce.number().min(0).describe("Total environmental fines"),
    science_based_targets: z.coerce.boolean().describe("Has SBTi approved targets"),
  }).describe("Environmental metrics"),
  social: z.object({
    employee_turnover_rate: z.coerce.number().min(0).max(1).describe("Employee turnover rate (0-1)"),
    gender_diversity_pct: z.coerce.number().min(0).max(100).describe("% female in workforce"),
    board_diversity_pct: z.coerce.number().min(0).max(100).describe("% female/minority on board"),
    living_wage_compliance: z.coerce.boolean().describe("Living wage compliance"),
    health_safety_incident_rate: z.coerce.number().min(0).describe("Incidents per 200k hours"),
    community_investment_pct: z.coerce.number().min(0).max(100).describe("% of pre-tax profit to community"),
    supply_chain_audit_pct: z.coerce.number().min(0).max(100).describe("% of suppliers audited"),
  }).describe("Social metrics"),
  governance: z.object({
    board_independence_pct: z.coerce.number().min(0).max(1).describe("% independent directors (0-1)"),
    ceo_chair_separation: z.coerce.boolean().describe("CEO/Chair roles separated"),
    executive_pay_ratio: z.coerce.number().positive().describe("CEO pay / median employee pay"),
    anti_corruption_policy: z.coerce.boolean().describe("Has anti-corruption policy"),
    whistleblower_mechanism: z.coerce.boolean().describe("Has whistleblower mechanism"),
    audit_committee_independence: z.coerce.boolean().describe("Audit committee fully independent"),
    related_party_transactions: z.coerce.number().min(0).describe("Related party transaction amount"),
  }).describe("Governance metrics"),
  pillar_weights: z.object({
    environmental: z.coerce.number().min(0).max(1).describe("Environmental weight"),
    social: z.coerce.number().min(0).max(1).describe("Social weight"),
    governance: z.coerce.number().min(0).max(1).describe("Governance weight"),
  }).optional().describe("Custom pillar weights (must sum to 1)"),
  peer_scores: z.array(z.object({
    company_name: z.string().describe("Peer company name"),
    esg_score: z.coerce.number().min(0).max(100).describe("Overall ESG score"),
    e_score: z.coerce.number().min(0).max(100).describe("Environmental score"),
    s_score: z.coerce.number().min(0).max(100).describe("Social score"),
    g_score: z.coerce.number().min(0).max(100).describe("Governance score"),
  })).optional().describe("Peer scores for benchmarking"),
});

export const CarbonFootprintSchema = z.object({
  company_name: z.string().describe("Company name"),
  revenue: z.coerce.number().positive().describe("Annual revenue"),
  scope1_emissions: z.coerce.number().min(0).describe("Scope 1 direct emissions (tCO2e)"),
  scope2_emissions: z.coerce.number().min(0).describe("Scope 2 location-based emissions (tCO2e)"),
  scope2_market_based: z.coerce.number().min(0).optional().describe("Market-based Scope 2 emissions"),
  scope3_categories: z.array(z.object({
    category: z.coerce.number().int().min(1).max(15).describe("GHG Protocol category 1-15"),
    name: z.string().describe("Category name"),
    emissions: z.coerce.number().min(0).describe("Emissions in tCO2e"),
  })).describe("Scope 3 breakdown by GHG Protocol category"),
  carbon_price: z.coerce.number().min(0).describe("Carbon price in $/tCO2e"),
  reduction_target_pct: z.coerce.number().min(0).max(1).describe("Target reduction (e.g. 0.42 = 42%)"),
  baseline_year_emissions: z.coerce.number().positive().describe("Total emissions in baseline year"),
  target_year: z.coerce.number().int().positive().describe("Target year (e.g. 2030)"),
});

export const GreenBondSchema = z.object({
  bond_name: z.string().describe("Bond name / identifier"),
  face_value: z.coerce.number().positive().describe("Face value of the bond"),
  coupon_rate: z.coerce.number().min(0).describe("Annual coupon rate"),
  maturity_years: z.coerce.number().positive().describe("Years to maturity"),
  green_bond_yield: z.coerce.number().min(0).describe("Yield of the green bond"),
  conventional_yield: z.coerce.number().min(0).describe("Yield of comparable conventional bond"),
  use_of_proceeds: z.array(z.object({
    project_name: z.string().describe("Project name"),
    allocation: z.coerce.number().min(0).describe("Amount allocated from proceeds"),
    category: z.string().describe("Category: Renewable Energy, Energy Efficiency, etc."),
    expected_co2_avoided: z.coerce.number().min(0).describe("Expected annual CO2 avoided (tCO2e)"),
  })).describe("Projects funded by green bond proceeds"),
  framework: z.enum(["Icma", "Cbi", "EuTaxonomy"]).describe("Applicable green bond framework"),
});

export const SllSchema = z.object({
  loan_name: z.string().describe("Loan name / identifier"),
  facility_amount: z.coerce.number().positive().describe("Total facility amount"),
  base_margin_bps: z.coerce.number().min(0).describe("Base margin in basis points (e.g. 200)"),
  spts: z.array(z.object({
    kpi_name: z.string().describe("KPI name"),
    baseline_value: z.coerce.number().describe("Baseline value"),
    target_value: z.coerce.number().describe("Target value"),
    current_value: z.coerce.number().describe("Current value"),
    margin_adjustment_bps: z.coerce.number().min(0).describe("Margin adjustment in bps if target met"),
    direction: z.enum(["Lower", "Higher"]).describe("Whether lower or higher is better"),
  })).describe("Sustainability performance targets"),
});
