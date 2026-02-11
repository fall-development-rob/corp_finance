import { z } from "zod";

const SegmentInputSchema = z.object({
  name: z.string(),
  revenue: z.coerce.number(),
  ebitda: z.coerce.number(),
  ebit: z.coerce.number(),
  net_income: z.coerce.number().optional(),
  assets: z.coerce.number().optional(),
  method: z.enum(["EvEbitda", "PeRatio", "EvRevenue", "EvEbit", "Dcf", "NavBased"]),
  multiple: z.coerce.number(),
  comparable_range: z.tuple([z.coerce.number(), z.coerce.number()]).optional(),
  growth_rate: z.coerce.number().optional(),
  margin: z.coerce.number().optional(),
});

export const SotpSchema = z.object({
  company_name: z.string(),
  segments: z.array(SegmentInputSchema),
  net_debt: z.coerce.number(),
  shares_outstanding: z.coerce.number(),
  holding_company_discount: z.coerce.number().optional(),
  minority_interests: z.coerce.number().optional(),
  unconsolidated_investments: z.coerce.number().optional(),
});

const PeerMultipleSchema = z.object({
  company: z.string(),
  pe_ratio: z.coerce.number(),
  pb_ratio: z.coerce.number(),
  ps_ratio: z.coerce.number(),
  ev_ebitda: z.coerce.number().optional(),
  peg_ratio: z.coerce.number().optional(),
});

export const TargetPriceSchema = z.object({
  current_price: z.coerce.number(),
  shares_outstanding: z.coerce.number(),
  earnings_per_share: z.coerce.number(),
  earnings_growth_rate: z.coerce.number(),
  book_value_per_share: z.coerce.number(),
  revenue_per_share: z.coerce.number(),
  dividend_per_share: z.coerce.number(),
  peer_multiples: z.array(PeerMultipleSchema),
  analyst_targets: z.array(z.coerce.number()).optional(),
  cost_of_equity: z.coerce.number(),
  terminal_growth: z.coerce.number(),
  projection_years: z.coerce.number(),
});
