import { z } from "zod";

const TradeExecutionSchema = z.object({
  trade_id: z.string().describe("Unique trade identifier"),
  security: z.string().describe("Security name or ticker"),
  side: z.string().describe("Trade side ('Buy' or 'Sell')"),
  quantity: z.coerce.number().describe("Number of shares or units traded"),
  decision_price: z.coerce.number().describe("Price at time of investment decision"),
  arrival_price: z.coerce.number().describe("Market price at order arrival"),
  execution_price: z.coerce.number().describe("Actual execution price achieved"),
  vwap_price: z.coerce.number().describe("Volume-weighted average price over execution window"),
  twap_price: z.coerce.number().describe("Time-weighted average price over execution window"),
  close_price: z.coerce.number().describe("Closing price on execution date"),
  commission: z.coerce.number().describe("Commission cost per share or total"),
  market_impact_estimate: z.coerce.number().describe("Estimated market impact cost (decimal)"),
  order_size: z.coerce.number().describe("Total order size (may differ from executed quantity)"),
  adv_pct: z.coerce.number().describe("Order size as percentage of average daily volume (decimal)"),
});

export const BestExecutionSchema = z.object({
  trades: z.array(TradeExecutionSchema).describe("List of trade executions to analyze"),
  benchmark: z.string().describe("Benchmark type: 'VWAP', 'TWAP', 'ArrivalPrice', or 'Close'"),
  reporting_currency: z.string().describe("Reporting currency code (e.g. 'USD')"),
});

const CashFlowEventSchema = z.object({
  day_of_period: z.coerce.number().int().min(0).describe("Day within the period when cash flow occurred (integer)"),
  amount: z.coerce.number().describe("Cash flow amount (positive for inflow, negative for outflow)"),
  total_days: z.coerce.number().int().min(1).describe("Total number of days in the period (integer)"),
});

const PerformancePeriodSchema = z.object({
  period_name: z.string().describe("Period identifier (e.g. '2025-Q1')"),
  beginning_value: z.coerce.number().describe("Portfolio value at start of period"),
  ending_value: z.coerce.number().describe("Portfolio value at end of period"),
  external_cash_flows: z.array(CashFlowEventSchema).describe("External cash flows during the period"),
  income: z.coerce.number().describe("Income earned during the period (dividends, interest)"),
  fees_management: z.coerce.number().describe("Management fees charged during the period"),
  fees_performance: z.coerce.number().describe("Performance fees charged during the period"),
  fees_trading: z.coerce.number().describe("Trading costs incurred during the period"),
});

const AccountReturnSchema = z.object({
  account_name: z.string().describe("Account identifier"),
  returns: z.array(z.coerce.number()).describe("Period returns for this account (one per period, decimal)"),
});

export const GipsReportSchema = z.object({
  composite_name: z.string().describe("Name of the GIPS composite"),
  periods: z.array(PerformancePeriodSchema).describe("Performance measurement periods"),
  benchmark_returns: z.array(z.coerce.number()).describe("Benchmark returns per period (decimal, one per period)"),
  inception_date: z.string().describe("Composite inception date (ISO format, e.g. '2020-01-01')"),
  reporting_currency: z.string().describe("Reporting currency code (e.g. 'USD')"),
  fee_schedule: z.string().describe("Fee reporting basis: 'Gross', 'Net', or 'Both'"),
  composite_accounts: z.array(AccountReturnSchema).describe("Individual account returns within the composite"),
});
