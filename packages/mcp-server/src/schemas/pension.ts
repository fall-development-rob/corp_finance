import { z } from "zod";

const ParticipantSchema = z.object({
  name: z.string().describe("Participant name"),
  current_age: z.coerce.number().int().positive().describe("Current age"),
  retirement_age: z.coerce.number().int().positive().describe("Expected retirement age"),
  years_of_service: z.coerce.number().int().min(0).describe("Years of service"),
  current_salary: z.coerce.number().positive().describe("Current annual salary"),
});

const RetireeSchema = z.object({
  name: z.string().describe("Retiree name"),
  current_age: z.coerce.number().int().positive().describe("Current age"),
  life_expectancy: z.coerce.number().int().positive().describe("Life expectancy age"),
  annual_benefit: z.coerce.number().positive().describe("Annual benefit payment"),
});

const PlanProvisionsSchema = z.object({
  benefit_formula_pct: z.coerce.number().positive().describe("Benefit accrual rate per year of service"),
  early_retirement_age: z.coerce.number().int().positive().describe("Early retirement age"),
  normal_retirement_age: z.coerce.number().int().positive().describe("Normal retirement age"),
  vesting_years: z.coerce.number().int().min(0).describe("Vesting period in years"),
  cola_rate: z.coerce.number().min(0).optional().describe("Cost-of-living adjustment rate"),
});

const ContributionConstraintsSchema = z.object({
  minimum_funding_pct: z.coerce.number().positive().describe("Minimum funding % of PBO"),
  maximum_deductible_pct: z.coerce.number().positive().describe("Maximum deductible % of PBO"),
  corridor_pct: z.coerce.number().positive().optional().describe("Corridor percentage for amortization"),
});

export const PensionFundingSchema = z.object({
  plan_name: z.string().describe("Pension plan name"),
  plan_assets: z.coerce.number().min(0).describe("Fair value of plan assets"),
  discount_rate: z.coerce.number().positive().describe("Discount rate for PV of obligations"),
  expected_return_on_assets: z.coerce.number().describe("Expected return on plan assets"),
  salary_growth_rate: z.coerce.number().min(0).describe("Expected salary growth rate"),
  inflation_rate: z.coerce.number().min(0).describe("General inflation rate"),
  benefit_obligation_type: z.enum(["Pbo", "Abo"]).describe("PBO or ABO"),
  active_participants: z.array(ParticipantSchema).describe("Active employee participants"),
  retired_participants: z.array(RetireeSchema).describe("Retired participants"),
  plan_provisions: PlanProvisionsSchema.describe("Plan rules"),
  contribution_constraints: ContributionConstraintsSchema.optional().describe("Regulatory contribution constraints"),
});

const AssetAllocationSchema = z.object({
  asset_class: z.string().describe("Asset class name"),
  weight: z.coerce.number().min(0).max(1).describe("Portfolio weight"),
  expected_return: z.coerce.number().describe("Expected return"),
  duration: z.coerce.number().min(0).describe("Duration in years"),
});

const LdiInstrumentSchema = z.object({
  name: z.string().describe("Instrument name"),
  instrument_type: z.string().describe("Type (Government Bond, Corporate Bond, TIPS, Swap)"),
  duration: z.coerce.number().min(0).describe("Duration in years"),
  yield_rate: z.coerce.number().describe("Yield rate"),
  convexity: z.coerce.number().optional().describe("Convexity"),
});

const GlidePathSchema = z.object({
  current_funded_ratio: z.coerce.number().positive().describe("Current funded ratio"),
  target_funded_ratio: z.coerce.number().positive().describe("Target funded ratio"),
  years_to_target: z.coerce.number().int().positive().describe("Years to reach target"),
  growth_allocation_start: z.coerce.number().min(0).max(1).describe("Growth allocation at start"),
  growth_allocation_end: z.coerce.number().min(0).max(1).describe("Growth allocation at end"),
});

export const LdiStrategySchema = z.object({
  plan_name: z.string().describe("Pension plan name"),
  liability_pv: z.coerce.number().positive().describe("PV of pension liabilities"),
  liability_duration: z.coerce.number().positive().describe("Macaulay duration of liabilities"),
  liability_convexity: z.coerce.number().optional().describe("Convexity of liabilities"),
  plan_assets: z.coerce.number().positive().describe("Fair value of plan assets"),
  current_asset_duration: z.coerce.number().min(0).describe("Current portfolio duration"),
  current_asset_allocation: z.array(AssetAllocationSchema).describe("Current portfolio allocation"),
  available_instruments: z.array(LdiInstrumentSchema).describe("Available hedging instruments"),
  target_hedge_ratio: z.coerce.number().min(0).max(1).describe("Target hedge ratio"),
  rebalancing_trigger: z.coerce.number().positive().optional().describe("Rebalancing trigger (years)"),
  glide_path: GlidePathSchema.optional().describe("Glide-path schedule"),
});
