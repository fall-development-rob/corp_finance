import { z } from "zod";

export const CurrencySchema = z
  .enum(["GBP", "USD", "EUR", "CHF", "JPY", "CAD", "AUD", "HKD", "SGD"])
  .describe("ISO 4217 currency code");

export const OutputFormatSchema = z
  .enum(["json", "table", "csv", "minimal"])
  .optional()
  .describe("Output format for the response");

export const ProjectionPeriodSchema = z.object({
  year: z.coerce.number().int().describe("Calendar or relative year"),
  label: z.string().describe("Period label, e.g. FY2025 or Year 3"),
  is_terminal: z.coerce.boolean().describe("Whether this is the terminal period"),
});

export const CashFlowSchema = z.object({
  date: z.string().describe("ISO 8601 date string (YYYY-MM-DD)"),
  amount: z.coerce.number().describe("Cash flow amount (negative for outflows)"),
  label: z.string().optional().describe("Optional label for the cash flow"),
});

export const CashFlowSeriesSchema = z.object({
  flows: z.array(CashFlowSchema).describe("Ordered list of cash flows"),
  currency: CurrencySchema,
});

export const SensitivityVariableSchema = z.object({
  name: z.string().describe("Variable name matching an input field"),
  min: z.coerce.number().describe("Minimum value for the sensitivity range"),
  max: z.coerce.number().describe("Maximum value for the sensitivity range"),
  step: z.coerce.number().positive().describe("Step size between values"),
});
