import { z } from "zod";

export const RentRollSchema = z.object({
  tenants: z.array(z.object({
    name: z.string().describe("Tenant name"),
    suite: z.string().describe("Suite/unit identifier"),
    leased_sf: z.coerce.number().positive().describe("Leased square footage"),
    base_rent_psf: z.coerce.number().positive().describe("Annual base rent per SF"),
    lease_start_year: z.coerce.number().int().describe("Lease commencement year"),
    lease_end_year: z.coerce.number().int().describe("Lease expiration year"),
    escalation_type: z.enum(["FixedStep", "CpiLinked", "PercentageRent", "FlatRent"]).describe("Rent escalation type"),
    annual_increase_pct: z.coerce.number().min(0).max(0.2).optional().describe("Annual escalation rate (for FixedStep)"),
    assumed_cpi: z.coerce.number().min(0).max(0.1).optional().describe("Assumed CPI rate (for CpiLinked)"),
    spread_over_cpi: z.coerce.number().min(0).max(0.05).optional().describe("Spread over CPI (for CpiLinked)"),
    renewal_probability: z.coerce.number().min(0).max(1).optional().describe("Probability tenant renews at expiry"),
    downtime_months: z.coerce.number().int().min(0).optional().describe("Vacancy months if tenant does not renew"),
    credit_score: z.coerce.number().min(1).max(10).optional().describe("Tenant credit quality 1-10"),
  })).min(1).describe("Array of tenant lease records"),
  total_building_sf: z.coerce.number().positive().describe("Total building square footage"),
  holding_period_years: z.coerce.number().int().min(1).max(30).describe("Analysis holding period in years"),
  market_rent_psf: z.coerce.number().positive().describe("Current market rent per SF"),
  operating_expenses_psf: z.coerce.number().min(0).describe("Operating expenses per SF"),
  expense_growth_rate: z.coerce.number().min(0).max(0.1).describe("Annual expense growth rate"),
});

export const ComparableSalesSchema = z.object({
  subject_property: z.object({
    address: z.string().describe("Subject property address"),
    sf: z.coerce.number().positive().describe("Subject gross building area SF"),
    noi: z.coerce.number().positive().optional().describe("Subject annual NOI"),
  }).describe("Subject property details"),
  comparables: z.array(z.object({
    address: z.string().describe("Comparable address"),
    sale_price: z.coerce.number().positive().describe("Sale price"),
    sale_date: z.string().describe("Sale date YYYY-MM-DD"),
    gross_building_area_sf: z.coerce.number().positive().describe("Gross building area SF"),
    net_rentable_area_sf: z.coerce.number().positive().optional().describe("Net rentable area SF"),
    year_built: z.coerce.number().int().describe("Year built"),
    noi: z.coerce.number().positive().optional().describe("Annual NOI at time of sale"),
    occupancy_pct: z.coerce.number().min(0).max(1).optional().describe("Occupancy rate at sale"),
    property_type: z.string().describe("Property type"),
    condition_rating: z.coerce.number().int().min(1).max(5).optional().describe("Condition rating 1-5"),
    adjustments: z.array(z.object({
      category: z.enum(["Location", "Condition", "Size", "Age", "Amenities", "MarketConditions", "FinancingTerms", "ConditionsOfSale", "PropertyRights"]).describe("Adjustment category"),
      pct_adjustment: z.coerce.number().min(-0.5).max(0.5).describe("Percentage adjustment (-50% to +50%)"),
      narrative: z.string().describe("Adjustment rationale"),
    })).optional().describe("Adjustment grid"),
  })).min(3).describe("Minimum 3 comparable sales"),
  reconciliation_method: z.enum(["EqualWeight", "QualityScore", "InverseDistance"]).optional().describe("Reconciliation method"),
});

export const HbuAnalysisSchema = z.object({
  site_area_sf: z.coerce.number().positive().describe("Site area in SF"),
  potential_uses: z.array(z.object({
    use_type: z.string().describe("Proposed use type (Office, Retail, etc.)"),
    max_buildable_sf: z.coerce.number().positive().describe("Maximum buildable SF"),
    estimated_noi_psf: z.coerce.number().describe("Estimated stabilised NOI per SF"),
    estimated_cap_rate: z.coerce.number().min(0.01).max(0.99).describe("Estimated market cap rate"),
    development_cost_psf: z.coerce.number().positive().describe("Total development cost per SF"),
    construction_months: z.coerce.number().int().min(1).describe("Construction period in months"),
  })).min(1).describe("Alternative uses to evaluate"),
  zoning: z.object({
    use_class: z.string().describe("Zoning classification (e.g. C-3, R-5)"),
    permitted_uses: z.array(z.string()).describe("List of permitted use types"),
    max_far: z.coerce.number().positive().describe("Maximum floor area ratio"),
    max_height_ft: z.coerce.number().positive().describe("Maximum building height in feet"),
    min_setback_ft: z.coerce.number().min(0).describe("Minimum setback in feet"),
    max_lot_coverage_pct: z.coerce.number().min(0).max(1).describe("Maximum lot coverage"),
  }).describe("Zoning constraints"),
  current_land_value: z.coerce.number().positive().describe("Current land value for feasibility comparison"),
});

export const ReplacementCostSchema = z.object({
  building_class: z.enum(["A", "B", "C", "D", "S"]).describe("Building class (A=steel, B=concrete, C=masonry, D=wood, S=metal)"),
  occupancy_type: z.enum(["Office", "Retail", "Industrial", "Multifamily", "Hospitality"]).describe("Building occupancy type"),
  gross_building_area_sf: z.coerce.number().positive().describe("Gross building area SF"),
  land_value: z.coerce.number().positive().describe("Land value"),
  effective_age_years: z.coerce.number().min(0).describe("Effective age of building in years"),
  total_economic_life_years: z.coerce.number().positive().describe("Total expected economic life in years"),
  local_cost_modifier: z.coerce.number().min(0.5).max(2.0).optional().describe("Local cost modifier (default 1.0)"),
  functional_obsolescence_pct: z.coerce.number().min(0).max(1).optional().describe("Functional obsolescence as % of RCN"),
  external_obsolescence_pct: z.coerce.number().min(0).max(1).optional().describe("External obsolescence as % of RCN"),
});

export const BenchmarkSchema = z.object({
  quarterly_returns: z.array(z.object({
    period: z.string().describe("Period label (e.g. 2024-Q1)"),
    beginning_value: z.coerce.number().positive().describe("Beginning market value"),
    ending_value: z.coerce.number().positive().describe("Ending market value"),
    noi: z.coerce.number().min(0).describe("Net operating income for the period"),
    capex: z.coerce.number().min(0).describe("Capital expenditures for the period"),
  })).min(1).describe("Quarterly property return data"),
  benchmark_returns: z.array(z.object({
    period: z.string().describe("Period label"),
    total_return: z.coerce.number().describe("Benchmark total return"),
    income_return: z.coerce.number().describe("Benchmark income return"),
    appreciation_return: z.coerce.number().describe("Benchmark appreciation return"),
  })).optional().describe("Benchmark index returns for comparison"),
  cost_of_debt: z.coerce.number().min(0).max(0.2).optional().describe("Cost of debt for leverage adjustment"),
  ltv: z.coerce.number().min(0).max(0.95).optional().describe("Loan-to-value ratio"),
});

export const AcquisitionSchema = z.object({
  property_name: z.string().describe("Property name"),
  purchase_price: z.coerce.number().positive().describe("Acquisition price"),
  closing_costs_pct: z.coerce.number().min(0).max(0.1).optional().describe("Closing costs as % of purchase price"),
  year_one_noi: z.coerce.number().positive().describe("Year 1 net operating income"),
  noi_growth_rate: z.coerce.number().min(-0.05).max(0.1).describe("Annual NOI growth rate"),
  holding_period_years: z.coerce.number().int().min(1).max(30).describe("Investment holding period"),
  exit_cap_rate: z.coerce.number().min(0.01).max(0.2).describe("Exit capitalisation rate"),
  disposition_cost_pct: z.coerce.number().min(0).max(0.1).optional().describe("Disposition costs as % of sale price"),
  debt_tranches: z.array(z.object({
    name: z.string().describe("Tranche name (Senior, Mezzanine)"),
    amount: z.coerce.number().positive().describe("Loan amount"),
    interest_rate: z.coerce.number().min(0).max(0.2).describe("Annual interest rate"),
    term_years: z.coerce.number().int().min(1).describe("Loan term in years"),
    amortization_years: z.coerce.number().int().min(1).optional().describe("Amortization period (if different from term)"),
    io_period_years: z.coerce.number().int().min(0).optional().describe("Interest-only period in years"),
  })).optional().describe("Debt structure"),
  discount_rate: z.coerce.number().min(0).max(0.3).describe("Discount rate for NPV"),
  target_irr: z.coerce.number().min(0).max(0.5).optional().describe("Target IRR for go/no-go"),
  target_dscr: z.coerce.number().min(1).optional().describe("Minimum DSCR threshold"),
});
