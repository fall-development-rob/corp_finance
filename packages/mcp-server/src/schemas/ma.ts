import { z } from "zod";

// --- ConsiderationType ---
// Rust enum (externally tagged serde default):
//   AllCash                    -> "AllCash"
//   AllStock                   -> "AllStock"
//   Mixed { cash_pct: Rate }   -> { "Mixed": { "cash_pct": 0.5 } }
const ConsiderationTypeSchema = z.union([
  z.literal("AllCash"),
  z.literal("AllStock"),
  z.object({
    Mixed: z.object({
      cash_pct: z
        .number()
        .min(0)
        .max(1)
        .describe("Cash percentage for mixed deals (0..1)"),
    }),
  }),
]);

// --- MergerInput ---
// Rust struct: MergerInput in ma/merger_model.rs
export const MergerSchema = z.object({
  // Acquirer
  acquirer_name: z.string().describe("Acquirer company name"),
  acquirer_net_income: z.number().describe("Acquirer annual net income"),
  acquirer_shares_outstanding: z
    .number()
    .positive()
    .describe("Acquirer shares outstanding"),
  acquirer_share_price: z
    .number()
    .positive()
    .describe("Acquirer current share price"),
  acquirer_tax_rate: z
    .number()
    .min(0)
    .max(1)
    .describe("Acquirer corporate tax rate"),

  // Target
  target_name: z.string().describe("Target company name"),
  target_net_income: z.number().describe("Target annual net income"),
  target_shares_outstanding: z
    .number()
    .positive()
    .describe("Target shares outstanding"),
  target_share_price: z
    .number()
    .positive()
    .describe("Target current share price"),

  // Deal terms
  offer_price_per_share: z
    .number()
    .positive()
    .describe("Offer price per target share"),
  consideration: ConsiderationTypeSchema.describe(
    'Deal consideration: "AllCash", "AllStock", or {"Mixed": {"cash_pct": 0.5}}'
  ),

  // Synergies
  revenue_synergies: z
    .number()
    .optional()
    .describe("Pre-tax revenue synergies expected (annual run-rate)"),
  cost_synergies: z
    .number()
    .optional()
    .describe("Pre-tax cost synergies expected (annual run-rate)"),
  synergy_phase_in_pct: z
    .number()
    .min(0)
    .max(1)
    .optional()
    .describe("Fraction of synergies realised in year 1 (0..1)"),
  integration_costs: z
    .number()
    .optional()
    .describe("One-time integration / restructuring costs"),

  // Financing (cash portion)
  debt_financing_rate: z
    .number()
    .min(0)
    .optional()
    .describe("Interest rate on new debt raised to fund the cash component"),
  foregone_interest_rate: z
    .number()
    .min(0)
    .optional()
    .describe("Rate earned on cash balances that are foregone when paying cash"),

  // Optional adjustments
  goodwill_amortisation: z
    .number()
    .optional()
    .describe("Annual goodwill amortisation charge (non-cash, pre-tax)"),
  transaction_fees: z
    .number()
    .optional()
    .describe("One-time transaction / advisory fees"),
});
