import { z } from "zod";

export const RealOptionSchema = z.object({
  option_type: z.enum(["Expand", "Abandon", "Defer", "Switch", "Contract", "Compound"]),
  underlying_value: z.coerce.number(),
  exercise_price: z.coerce.number(),
  volatility: z.coerce.number(),
  risk_free_rate: z.coerce.number(),
  time_to_expiry: z.coerce.number(),
  steps: z.coerce.number().optional(),
  dividend_yield: z.coerce.number().optional(),
  expansion_factor: z.coerce.number().optional(),
  contraction_factor: z.coerce.number().optional(),
  switch_cost: z.coerce.number().optional(),
  switch_value_ratio: z.coerce.number().optional(),
});

const TreeNodeSchema = z.object({
  id: z.string(),
  name: z.string(),
  node_type: z.enum(["Decision", "Chance", "Terminal"]),
  value: z.coerce.number().optional(),
  cost: z.coerce.number().optional(),
  probability: z.coerce.number().optional(),
  children: z.array(z.string()),
  time_period: z.coerce.number().optional(),
});

export const DecisionTreeSchema = z.object({
  nodes: z.array(TreeNodeSchema),
  discount_rate: z.coerce.number(),
  risk_adjustment: z.coerce.number().optional(),
});
