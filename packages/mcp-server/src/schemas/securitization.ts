import { z } from "zod";

export const AbsMbsSchema = z.object({
  pool_balance: z.number().positive().describe("Initial pool balance (unpaid principal balance)"),
  weighted_avg_coupon: z.number().min(0).max(1).describe("Weighted average coupon rate (e.g. 0.055 = 5.5%)"),
  weighted_avg_maturity_months: z.number().int().positive().describe("Weighted average maturity in months"),
  weighted_avg_age_months: z.number().int().min(0).describe("Weighted average loan age in months (WALA)"),
  num_loans: z.number().int().positive().describe("Number of loans in the pool"),
  prepayment_model: z.union([
    z.object({ Cpr: z.number().min(0).max(1).describe("Constant Prepayment Rate (annual)") }),
    z.object({ Psa: z.number().positive().describe("PSA speed (100 = 100% PSA)") }),
    z.object({ Smm: z.number().min(0).max(1).describe("Single Monthly Mortality rate") }),
  ]).describe("Prepayment model: CPR, PSA, or SMM"),
  default_model: z.union([
    z.object({ Cdr: z.number().min(0).max(1).describe("Constant Default Rate (annual)") }),
    z.object({ Sda: z.number().positive().describe("SDA speed (100 = 100% SDA)") }),
    z.literal("None"),
  ]).describe("Default model: CDR, SDA, or None"),
  loss_severity: z.number().min(0).max(1).describe("Loss given default (e.g. 0.40 = 40%)"),
  recovery_lag_months: z.number().int().min(0).describe("Months to recover from defaulted loans"),
  servicing_fee_rate: z.number().min(0).max(0.05).describe("Annual servicing fee rate (e.g. 0.0025 = 25bps)"),
  projection_months: z.number().int().positive().describe("Number of months to project"),
});

export const TranchingSchema = z.object({
  deal_name: z.string().describe("Deal name / identifier"),
  collateral_balance: z.number().positive().describe("Total collateral pool balance"),
  collateral_cashflows: z.array(z.object({
    period: z.number().int().positive().describe("Period number (1-indexed)"),
    interest: z.number().min(0).describe("Interest collected in this period"),
    principal: z.number().min(0).describe("Principal collected in this period"),
    losses: z.number().min(0).describe("Losses in this period"),
  })).describe("Period-by-period cash flows from the collateral"),
  tranches: z.array(z.object({
    name: z.string().describe("Tranche name (e.g. 'AAA', 'BBB', 'Equity')"),
    balance: z.number().positive().describe("Par/face amount"),
    coupon_rate: z.number().min(0).max(0.5).describe("Annual coupon rate (decimal)"),
    seniority: z.number().int().positive().describe("Seniority: 1 = most senior"),
    is_fixed_rate: z.boolean().describe("Whether the coupon is fixed-rate"),
    payment_frequency: z.number().int().positive().describe("Coupon payments per year (4=quarterly, 12=monthly)"),
  })).describe("Tranche specifications"),
  reserve_account: z.number().min(0).describe("Initial cash reserve account balance"),
  oc_trigger: z.number().positive().optional().describe("Overcollateralisation trigger ratio (e.g. 1.20)"),
  ic_trigger: z.number().positive().optional().describe("Interest coverage trigger ratio (e.g. 1.05)"),
  reinvestment_period_months: z.number().int().min(0).describe("Months during which principal can be reinvested"),
});
