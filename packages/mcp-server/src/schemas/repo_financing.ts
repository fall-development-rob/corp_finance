import { z } from "zod";

const RepoRateInputSchema = z.object({
  collateral_value: z.coerce.number().describe("Market value of collateral"),
  repo_rate: z.coerce.number().describe("Annualized repo rate"),
  term_days: z.coerce.number().int().describe("Term of repo in days"),
  day_count_basis: z.coerce.number().int().describe("Day count basis (360 or 365)"),
  haircut_pct: z.coerce.number().describe("Haircut percentage (0-1)"),
  initial_margin: z.coerce.number().describe("Initial margin (e.g. 1.02 for 102%)"),
  accrued_interest: z.coerce.number().describe("Accrued interest on collateral"),
});

const ImpliedRepoInputSchema = z.object({
  spot_clean_price: z.coerce.number().describe("Spot clean price"),
  forward_clean_price: z.coerce.number().describe("Forward clean price"),
  spot_accrued: z.coerce.number().describe("Spot accrued interest"),
  forward_accrued: z.coerce.number().describe("Forward accrued interest"),
  coupon_income: z.coerce.number().describe("Coupon income during repo term"),
  term_days: z.coerce.number().int().describe("Term in days"),
  day_count_basis: z.coerce.number().int().describe("Day count basis"),
});

const RepoTermPointSchema = z.object({
  term_days: z.coerce.number().int().describe("Term in days"),
  rate: z.coerce.number().describe("Repo rate for this term"),
});

const RepoTermInputSchema = z.object({
  overnight_rate: z.coerce.number().describe("Overnight repo rate"),
  term_rates: z.array(RepoTermPointSchema).describe("Term repo rate points"),
  collateral_type: z.enum(["Treasury", "Agency", "Corporate", "Equity"]).describe("Collateral type"),
});

const SecLendingInputSchema = z.object({
  security_value: z.coerce.number().describe("Market value of lent security"),
  lending_fee_bps: z.coerce.number().describe("Lending fee in basis points"),
  cash_reinvestment_rate: z.coerce.number().describe("Cash collateral reinvestment rate"),
  collateral_pct: z.coerce.number().describe("Collateral percentage (e.g. 1.02)"),
  term_days: z.coerce.number().int().describe("Lending term in days"),
  day_count_basis: z.coerce.number().int().describe("Day count basis"),
});

export const RepoAnalyticsSchema = z.object({
  model: z.discriminatedUnion("type", [
    z.object({ type: z.literal("Rate"), ...RepoRateInputSchema.shape }),
    z.object({ type: z.literal("ImpliedRepo"), ...ImpliedRepoInputSchema.shape }),
    z.object({ type: z.literal("TermStructure"), ...RepoTermInputSchema.shape }),
    z.object({ type: z.literal("SecLending"), ...SecLendingInputSchema.shape }),
  ]).describe("Repo analytics model selection"),
});

const CreditRating = z.enum(["AAA", "AA", "A", "BBB", "BB", "B", "CCC"]);
const CollateralType = z.enum(["Treasury", "Agency", "Corporate", "Equity"]);

const HaircutInputSchema = z.object({
  collateral_type: CollateralType.describe("Type of collateral"),
  credit_rating: CreditRating.describe("Credit rating"),
  remaining_maturity: z.coerce.number().describe("Remaining maturity in years"),
  price_volatility: z.coerce.number().describe("Price volatility as decimal"),
  market_liquidity_score: z.coerce.number().describe("Liquidity score (0-10, 10=most liquid)"),
  is_cross_currency: z.boolean().describe("Whether repo is cross-currency"),
});

const MarginCallInputSchema = z.object({
  initial_collateral_value: z.coerce.number().describe("Initial collateral value"),
  current_collateral_value: z.coerce.number().describe("Current collateral value"),
  loan_amount: z.coerce.number().describe("Loan/borrowed amount"),
  initial_margin_pct: z.coerce.number().describe("Initial margin percentage"),
  maintenance_margin_pct: z.coerce.number().describe("Maintenance margin percentage"),
  variation_margin_pct: z.coerce.number().describe("Variation margin percentage"),
  haircut_pct: z.coerce.number().describe("Haircut percentage"),
});

const RehypothecationInputSchema = z.object({
  total_collateral_received: z.coerce.number().describe("Total collateral received"),
  rehypothecation_limit_pct: z.coerce.number().describe("Rehypothecation limit (0-1)"),
  collateral_reuse_rate: z.coerce.number().describe("Reuse rate per chain"),
  funding_rate: z.coerce.number().describe("Funding rate benefit"),
  term_days: z.coerce.number().int().describe("Term in days"),
  day_count_basis: z.coerce.number().int().describe("Day count basis"),
  num_reuse_chains: z.coerce.number().int().describe("Number of reuse chains"),
});

export const CollateralSchema = z.object({
  model: z.discriminatedUnion("type", [
    z.object({ type: z.literal("Haircut"), ...HaircutInputSchema.shape }),
    z.object({ type: z.literal("MarginCall"), ...MarginCallInputSchema.shape }),
    z.object({ type: z.literal("Rehypothecation"), ...RehypothecationInputSchema.shape }),
  ]).describe("Collateral model selection"),
});
