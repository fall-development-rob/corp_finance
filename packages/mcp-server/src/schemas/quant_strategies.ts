import { z } from "zod";

export const PairsTradingSchema = z.object({
  asset_a_name: z.string().describe("Name of asset A"),
  asset_b_name: z.string().describe("Name of asset B"),
  asset_a_prices: z.array(z.coerce.number()).describe("Historical prices for asset A (at least 20)"),
  asset_b_prices: z.array(z.coerce.number()).describe("Historical prices for asset B (at least 20)"),
  lookback_period: z.coerce.number().int().positive().describe("Lookback period for z-score calculation (default 20)"),
  entry_z_score: z.coerce.number().positive().describe("Z-score threshold to enter a trade (default 2.0)"),
  exit_z_score: z.coerce.number().min(0).describe("Z-score threshold to exit a trade (default 0.5)"),
  stop_loss_z_score: z.coerce.number().positive().describe("Z-score threshold for stop loss (default 3.5)"),
  capital: z.coerce.number().positive().describe("Total capital allocated to the strategy"),
  transaction_cost_bps: z.coerce.number().describe("Transaction cost in basis points"),
});

const MomentumAssetSchema = z.object({
  name: z.string().describe("Asset name or ticker"),
  monthly_returns: z.array(z.coerce.number()).describe("Monthly returns (e.g. 0.05 = 5%)"),
});

export const MomentumSchema = z.object({
  assets: z.array(MomentumAssetSchema).describe("List of assets with historical monthly returns"),
  lookback_months: z.coerce.number().int().positive().describe("Lookback period in months for momentum calculation (default 12)"),
  skip_months: z.coerce.number().int().min(0).describe("Number of most recent months to skip (default 1)"),
  rebalance_frequency: z.string().describe("Rebalance frequency: 'Monthly' or 'Quarterly'"),
  top_n: z.coerce.number().int().positive().describe("Number of top momentum assets to hold"),
  risk_free_rate: z.coerce.number().describe("Annualized risk-free rate"),
});
