import { z } from "zod";

export const RecoverySchema = z.object({
  enterprise_value: z.coerce.number().positive().describe("Going-concern enterprise value"),
  liquidation_value: z.coerce.number().positive().describe("Liquidation (fire-sale) value of assets"),
  valuation_type: z.enum(["GoingConcern", "Liquidation", "Both"]).describe("Whether to run GC, liquidation, or both waterfalls"),
  claims: z.array(z.object({
    name: z.string().describe("Human-readable claim identifier"),
    amount: z.coerce.number().positive().describe("Face (par) value of the claim"),
    priority: z.enum([
      "SuperPriority", "Administrative", "Priority",
      "SecuredFirst", "SecuredSecond", "Senior",
      "SeniorSubordinated", "Subordinated", "Mezzanine", "Equity",
    ]).describe("Priority class in the absolute priority rule"),
    is_secured: z.coerce.boolean().describe("Whether the claim is backed by collateral"),
    collateral_value: z.coerce.number().min(0).optional().describe("Value of collateral (secured claims only)"),
    interest_rate: z.coerce.number().min(0).max(0.3).optional().describe("Contractual interest rate"),
    accrued_months: z.coerce.number().int().min(0).optional().describe("Months of unpaid interest to accrue"),
  })).describe("Capital structure claims ordered by priority"),
  administrative_costs: z.coerce.number().min(0).describe("Chapter 11 administrative costs"),
  dip_facility: z.object({
    amount: z.coerce.number().positive().describe("Total DIP commitment drawn"),
    priming: z.coerce.boolean().describe("Whether DIP primes existing secured debt"),
    roll_up_amount: z.coerce.number().min(0).describe("Portion that rolls up pre-petition claims"),
  }).optional().describe("Optional DIP financing facility"),
  cash_on_hand: z.coerce.number().min(0).describe("Cash on hand available for distribution"),
});

export const DistressedDebtSchema = z.object({
  enterprise_value: z.coerce.number().positive().describe("Current enterprise value estimate"),
  exit_enterprise_value: z.coerce.number().positive().describe("Post-restructuring / exit enterprise value"),
  exit_timeline_years: z.coerce.number().positive().describe("Expected time to exit or resolution (years)"),
  capital_structure: z.array(z.object({
    name: z.string().describe("Tranche name (must be unique)"),
    face_value: z.coerce.number().positive().describe("Outstanding face / par value"),
    market_price: z.coerce.number().min(0).max(2).describe("Secondary market price (cents on dollar, e.g. 0.65)"),
    coupon_rate: z.coerce.number().min(0).max(0.3).describe("Annual coupon rate"),
    maturity_years: z.coerce.number().positive().describe("Remaining years to maturity"),
    seniority: z.enum(["DIP", "FirstLien", "SecondLien", "Senior", "SeniorSub", "Subordinated", "Mezzanine"]).describe("Position in capital structure"),
    is_secured: z.coerce.boolean().describe("Whether the tranche is secured"),
  })).describe("Current debt stack ordered by seniority"),
  proposed_treatment: z.array(z.object({
    tranche_name: z.string().describe("Must match a capital_structure tranche name"),
    treatment_type: z.enum(["Reinstate", "Amend", "Exchange", "EquityConversion", "CashPaydown", "Combination"]).describe("Type of restructuring treatment"),
    new_face_value: z.coerce.number().positive().optional().describe("New face value (for Exchange)"),
    new_coupon: z.coerce.number().min(0).max(0.3).optional().describe("New coupon rate (for Amend or Exchange)"),
    equity_conversion_pct: z.coerce.number().min(0).max(1).optional().describe("% of reorganized equity received"),
    cash_paydown: z.coerce.number().min(0).optional().describe("Cash paid at closing"),
  })).describe("Proposed restructuring treatment for each tranche"),
  dip_facility: z.object({
    commitment: z.coerce.number().positive().describe("Total commitment amount"),
    drawn: z.coerce.number().min(0).describe("Amount currently drawn"),
    rate: z.coerce.number().min(0).max(0.3).describe("Annual interest rate on drawn amounts"),
    fees_pct: z.coerce.number().min(0).max(0.1).describe("Upfront and commitment fees as percentage"),
    term_months: z.coerce.number().int().min(1).describe("Facility term in months"),
    converts_to_exit: z.coerce.boolean().describe("Whether DIP converts to exit financing"),
  }).optional().describe("Optional DIP financing terms"),
  operating_assumptions: z.object({
    annual_ebitda: z.coerce.number().describe("Annual EBITDA"),
    maintenance_capex: z.coerce.number().min(0).describe("Annual maintenance capex"),
    working_capital_change: z.coerce.number().describe("Annual working capital change (positive = use of cash)"),
    restructuring_costs: z.coerce.number().min(0).describe("One-time restructuring costs"),
  }).describe("Operating assumptions for the restructured entity"),
});
