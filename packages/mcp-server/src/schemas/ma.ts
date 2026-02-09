import { z } from "zod";

export const MergerSchema = z.object({
  acquirer_name: z.string().describe("Acquirer company name"),
  acquirer_net_income: z.number().describe("Acquirer annual net income"),
  acquirer_shares_outstanding: z.number().positive().describe("Acquirer shares outstanding"),
  acquirer_share_price: z.number().positive().describe("Acquirer current share price"),
  acquirer_tax_rate: z.number().min(0).max(0.5).describe("Acquirer corporate tax rate"),
  target_name: z.string().describe("Target company name"),
  target_net_income: z.number().describe("Target annual net income"),
  target_shares_outstanding: z.number().positive().describe("Target shares outstanding"),
  target_share_price: z.number().positive().describe("Target current share price"),
  offer_price_per_share: z.number().positive().describe("Offer price per target share"),
  consideration: z.object({
    type: z.enum(["AllCash", "AllStock", "Mixed"]).describe("Type of consideration"),
    cash_pct: z.number().min(0).max(1).optional().describe("Cash percentage for Mixed deals"),
  }).describe("Deal consideration structure"),
  revenue_synergies: z.number().optional().describe("Expected annual revenue synergies"),
  cost_synergies: z.number().optional().describe("Expected annual cost synergies"),
  synergy_phase_in_pct: z.number().min(0).max(1).optional().describe("% of synergies realized in year 1"),
  integration_costs: z.number().optional().describe("One-time integration costs"),
  debt_financing_rate: z.number().min(0).max(0.2).optional().describe("Interest rate on acquisition debt"),
  foregone_interest_rate: z.number().min(0).max(0.1).optional().describe("Rate earned on cash used"),
  transaction_fees: z.number().optional().describe("Advisory and transaction fees"),
});
