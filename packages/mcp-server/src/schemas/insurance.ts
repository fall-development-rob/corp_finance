import { z } from "zod";

export const ReservingSchema = z.object({
  line_of_business: z.string().describe("Line of business, e.g. 'Auto Liability'"),
  triangle: z.object({
    accident_years: z.array(z.number().int()).describe("Accident years"),
    development_periods: z.array(z.number().int()).describe("Development periods in years"),
    values: z.array(z.array(z.number().nullable())).describe("Cumulative claim amounts (null for unemerged cells)"),
  }).describe("Cumulative claims triangle"),
  method: z.enum(["ChainLadder", "BornhuetterFerguson", "Both"]).describe("Reserving method"),
  earned_premium: z.array(z.number()).optional().describe("Earned premium by accident year (required for BF)"),
  expected_loss_ratio: z.number().min(0).max(1).optional().describe("A priori expected loss ratio (required for BF, e.g. 0.65)"),
  tail_factor: z.number().positive().optional().describe("Tail factor for development beyond last column (default 1.0)"),
  discount_rate: z.number().min(0).optional().describe("Discount rate for present-valuing reserves"),
});

export const PremiumPricingSchema = z.object({
  line_of_business: z.string().describe("Line of business (e.g. 'Motor', 'Liability')"),
  exposure_units: z.number().positive().describe("Number of policies or units of exposure"),
  claim_frequency: z.number().min(0).describe("Claims per exposure unit per year (e.g. 0.05)"),
  average_severity: z.number().min(0).describe("Average claim size"),
  severity_trend: z.number().describe("Annual severity inflation (e.g. 0.03 = 3%)"),
  frequency_trend: z.number().describe("Annual frequency change (e.g. -0.01 = -1%)"),
  projection_years: z.number().int().positive().describe("Years to project trends forward"),
  expense_ratio_target: z.number().min(0).max(1).describe("Target expense ratio (e.g. 0.30)"),
  profit_margin_target: z.number().min(0).max(1).describe("Target profit margin (e.g. 0.05)"),
  reinsurance_cost_pct: z.number().min(0).max(1).describe("Reinsurance cost as % of premium"),
  investment_income_credit: z.number().min(0).describe("Investment income reducing needed premium"),
  large_loss_load_pct: z.number().min(0).describe("Loading for catastrophe/large losses"),
});

export const CombinedRatioSchema = z.object({
  company_name: z.string().describe("Company name"),
  periods: z.array(z.object({
    year: z.number().int().describe("Year"),
    net_earned_premium: z.number().positive().describe("Net earned premium"),
    net_incurred_losses: z.number().min(0).describe("Claims + IBNR change"),
    loss_adjustment_expenses: z.number().min(0).describe("Loss adjustment expenses"),
    underwriting_expenses: z.number().min(0).describe("Commissions, overhead, etc."),
    policyholder_dividends: z.number().min(0).describe("Policyholder dividends"),
    net_investment_income: z.number().min(0).describe("Net investment income"),
    realized_gains: z.number().describe("Realised gains on investments"),
  })).describe("Insurance periods for analysis"),
});

export const ScrSchema = z.object({
  company_name: z.string().describe("Company name"),
  premium_risk: z.object({
    net_earned_premium: z.number().positive().describe("Net earned premium"),
    net_best_estimate_reserves: z.number().min(0).describe("Net best-estimate reserves"),
    premium_risk_factor: z.number().min(0).max(1).describe("Premium risk factor by LoB (e.g. 0.10)"),
    reserve_risk_factor: z.number().min(0).max(1).describe("Reserve risk factor by LoB (e.g. 0.08)"),
    geographic_diversification: z.number().min(0).max(1).describe("Geographic diversification factor"),
  }).describe("Premium and reserve risk parameters"),
  catastrophe_risk: z.number().min(0).describe("Catastrophe risk capital charge"),
  market_risk: z.number().min(0).describe("Investment/market risk charge"),
  credit_risk: z.number().min(0).describe("Counterparty default risk"),
  operational_risk_premium: z.number().min(0).describe("Gross written premium for op risk calc"),
  eligible_own_funds: z.number().positive().describe("Total available capital"),
  mcr_factor: z.number().min(0).max(1).describe("MCR as proportion of SCR (0.25-0.45)"),
});
