import { z } from "zod";
import { SensitivityVariableSchema } from "./common.js";

export const SensitivitySchema = z.object({
  model: z
    .enum(["Dcf", "Lbo", "Bond", "CreditMetrics"])
    .describe("Financial model to run sensitivity on"),
  variable_1: SensitivityVariableSchema.describe(
    "First sensitivity axis (row variable)"
  ),
  variable_2: SensitivityVariableSchema.describe(
    "Second sensitivity axis (column variable)"
  ),
  base_inputs: z
    .record(z.unknown())
    .describe("Full set of model inputs as baseline parameters"),
  output_metric: z
    .string()
    .optional()
    .describe(
      "Output field to extract (e.g. enterprise_value, irr). Defaults to primary output."
    ),
});

export const ScenarioSchema = z.object({
  model: z
    .enum(["Dcf", "Lbo", "Bond", "CreditMetrics"])
    .describe("Financial model to run scenarios on"),
  scenarios: z
    .array(
      z.object({
        name: z.string().describe("Scenario name (e.g. Bear, Base, Bull)"),
        probability: z
          .number()
          .min(0)
          .max(1)
          .describe("Probability weight (all scenarios should sum to 1.0)"),
        overrides: z
          .record(z.unknown())
          .describe("Parameter overrides for this scenario"),
      })
    )
    .min(2)
    .describe("Scenario definitions with probability weights"),
  base_inputs: z
    .record(z.unknown())
    .describe("Full set of baseline model inputs"),
  output_metric: z
    .string()
    .optional()
    .describe("Output field to extract for comparison across scenarios"),
});
