import { z } from "zod";
import { SensitivityVariableSchema } from "./common.js";

export const SensitivitySchema = z.object({
  base_inputs: z
    .record(z.unknown())
    .describe("Base case input values (model-specific JSON)"),
  variable_1: SensitivityVariableSchema.describe(
    "First sensitivity axis (row variable)"
  ),
  variable_2: SensitivityVariableSchema.describe(
    "Second sensitivity axis (column variable)"
  ),
  output_metric: z
    .string()
    .describe(
      "Name of the output metric being measured (e.g. enterprise_value, irr)"
    ),
  compute_fn: z
    .string()
    .describe("Model function identifier (e.g. dcf, lbo)"),
});

export const ScenarioSchema = z.object({
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
    .describe("Base case input values (model-specific JSON)"),
  output_values: z
    .array(z.coerce.number())
    .describe("Pre-computed output value for each scenario"),
  base_case_value: z
    .number()
    .describe("Base case output value for deviation calculations"),
});
