import { z } from "zod";

export const CashManagementSchema = z.object({
  current_cash: z.coerce.number().describe("Current cash balance"),
  operating_cash_flows: z.array(z.coerce.number()).describe("Projected monthly operating cash flows (12 months)"),
  minimum_cash_buffer: z.coerce.number().min(0).describe("Minimum required cash balance"),
  credit_facility_size: z.coerce.number().min(0).describe("Available revolving credit facility size"),
  credit_facility_rate: z.coerce.number().min(0).describe("Annual rate charged on drawn facility"),
  investment_rate: z.coerce.number().min(0).describe("Annual rate earned on surplus cash (money market)"),
  overdraft_rate: z.coerce.number().describe("Annual penalty rate for going below the minimum buffer"),
  accounts_receivable: z.coerce.number().describe("Current accounts receivable"),
  accounts_payable: z.coerce.number().describe("Current accounts payable"),
  dso_days: z.coerce.number().describe("Days sales outstanding"),
  dpo_days: z.coerce.number().describe("Days payable outstanding"),
  annual_revenue: z.coerce.number().min(0).describe("Annual revenue (for DSO/DPO calculations)"),
  sweep_threshold: z.coerce.number().min(0).describe("Cash level above which auto-sweep to investment account"),
  target_cash_ratio: z.coerce.number().describe("Target cash as a percentage of revenue"),
});

export const HedgingSchema = z.object({
  hedge_type: z.enum(["FairValue", "CashFlow", "NetInvestment"]).describe("Type of hedge accounting relationship"),
  exposure_currency: z.string().describe("Currency of the underlying exposure"),
  hedge_currency: z.string().describe("Currency used for hedging"),
  notional_amount: z.coerce.number().min(0).describe("Notional amount of the exposure"),
  hedge_notional: z.coerce.number().min(0).describe("Notional amount of the hedge instrument"),
  hedge_instrument: z.enum(["Forward", "Option", "Swap", "Collar"]).describe("Type of hedge instrument"),
  exposure_changes: z.array(z.coerce.number()).describe("Period-to-period changes in hedged item value"),
  hedge_changes: z.array(z.coerce.number()).describe("Period-to-period changes in hedging instrument value"),
  risk_free_rate_domestic: z.coerce.number().describe("Domestic risk-free rate"),
  risk_free_rate_foreign: z.coerce.number().describe("Foreign risk-free rate"),
  spot_rate: z.coerce.number().positive().describe("Current spot FX rate"),
  forward_rate: z.coerce.number().describe("Contracted forward FX rate"),
  volatility: z.coerce.number().min(0).describe("Implied volatility for option-based hedges"),
  tenor_months: z.coerce.number().int().positive().describe("Hedge tenor in months"),
  confidence_level: z.coerce.number().describe("Confidence level for VaR (e.g. 0.95)"),
});
