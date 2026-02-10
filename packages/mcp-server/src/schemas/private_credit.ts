import { z } from "zod";

export const UnitrancheSchema = z.object({
  deal_name: z.string().describe("Deal identifier"),
  total_commitment: z.number().positive().describe("Total unitranche facility commitment"),
  borrower_ebitda: z.number().positive().describe("Borrower LTM EBITDA"),
  borrower_revenue: z.number().positive().describe("Borrower LTM revenue"),
  first_out_pct: z.number().min(0).max(1).describe("Percentage of unitranche allocated to first-out (e.g. 0.60 = 60%)"),
  first_out_spread_bps: z.number().min(0).describe("First-out tranche spread in basis points"),
  last_out_spread_bps: z.number().min(0).describe("Last-out tranche spread in basis points"),
  base_rate: z.number().min(0).describe("Base rate (SOFR or equivalent, e.g. 0.05 = 5%)"),
  oid_pct: z.number().min(0).describe("Original issue discount (e.g. 0.02 = 2 points)"),
  upfront_fee_pct: z.number().min(0).describe("Upfront fee (e.g. 0.01 = 1%)"),
  commitment_fee_bps: z.number().min(0).describe("Commitment fee on undrawn portion in bps"),
  drawn_pct: z.number().min(0).max(1).describe("Percentage of facility currently drawn (0-1)"),
  maturity_years: z.number().positive().describe("Maturity in years"),
  amortization_pct: z.number().min(0).describe("Annual mandatory amortization (e.g. 0.01 = 1%)"),
  call_protection_years: z.number().int().min(0).describe("Years of call protection"),
  call_premium_pct: z.number().min(0).describe("Call premium during protection period (e.g. 0.02 = 2%)"),
  leverage_covenant: z.number().positive().optional().describe("Maximum leverage ratio covenant"),
  coverage_covenant: z.number().positive().optional().describe("Minimum coverage ratio covenant"),
});

export const DirectLoanSchema = z.object({
  loan_name: z.string().describe("Loan identifier"),
  commitment: z.number().positive().describe("Total commitment amount"),
  drawn_amount: z.number().positive().describe("Initial drawn amount"),
  base_rate: z.number().min(0).describe("Base reference rate (e.g. SOFR at 0.05)"),
  spread_bps: z.number().min(0).describe("Credit spread in basis points (e.g. 550 = 5.50%)"),
  pik_rate: z.number().min(0).optional().describe("PIK interest rate added to principal each period"),
  pik_toggle: z.boolean().describe("Whether borrower can elect PIK instead of cash pay"),
  delayed_draw_amount: z.number().min(0).optional().describe("Additional undrawn commitment for delayed draw"),
  delayed_draw_fee_bps: z.number().min(0).describe("Fee on undrawn delayed draw in bps"),
  maturity_years: z.number().int().positive().describe("Maturity in years"),
  amortization_schedule: z.union([
    z.literal("InterestOnly"),
    z.literal("BulletMaturity"),
    z.object({ LevelAmort: z.number() }),
    z.object({ Custom: z.array(z.number()) }),
  ]).describe("Repayment profile"),
  prepayment_penalty: z.array(z.object({
    year: z.number().int().positive().describe("Year"),
    premium_pct: z.number().min(0).describe("Prepayment premium percentage"),
  })).describe("Call protection by year"),
  floor_rate: z.number().min(0).optional().describe("Base rate floor (e.g. SOFR floor)"),
  projection_years: z.number().int().positive().describe("Number of years to project"),
  expected_default_rate: z.number().min(0).max(1).describe("Annual probability of default"),
  expected_loss_severity: z.number().min(0).max(1).describe("Loss-given-default"),
});

export const SyndicationSchema = z.object({
  total_facility: z.number().positive().describe("Total facility size"),
  arranger_hold: z.number().positive().describe("Amount retained by lead arranger"),
  syndicate_members: z.array(z.object({
    name: z.string().describe("Member name"),
    commitment: z.number().positive().describe("Member commitment"),
    is_lead: z.boolean().describe("Whether this member is the lead"),
  })).describe("Syndicate members"),
  arrangement_fee_bps: z.number().min(0).describe("Arrangement fee in bps on total facility"),
  participation_fee_bps: z.number().min(0).describe("Participation fee in bps on allocation"),
  coupon_spread_bps: z.number().min(0).describe("Ongoing coupon spread in bps"),
});
