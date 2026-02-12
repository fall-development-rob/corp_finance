import { z } from "zod";

export const EconomicCapitalSchema = z.object({
  portfolio_losses: z.array(z.coerce.number()).describe("Simulated portfolio loss distribution"),
  confidence_level: z.coerce.number().describe("Confidence level for VaR/ES (e.g. 0.999)"),
  pd: z.coerce.number().describe("Probability of default"),
  lgd: z.coerce.number().describe("Loss given default"),
  ead: z.coerce.number().describe("Exposure at default"),
  maturity: z.coerce.number().describe("Effective maturity in years"),
  total_capital: z.coerce.number().describe("Total available capital"),
});

export const RarocSchema = z.object({
  revenue: z.coerce.number().describe("Revenue from the exposure"),
  operating_costs: z.coerce.number().describe("Operating costs allocated"),
  expected_loss: z.coerce.number().describe("Expected loss provision"),
  economic_capital: z.coerce.number().describe("Economic capital allocated"),
  hurdle_rate: z.coerce.number().describe("Hurdle rate as decimal"),
  cost_of_equity: z.coerce.number().describe("Cost of equity as decimal"),
  exposure: z.coerce.number().describe("Total exposure amount"),
});

export const EulerAllocationSchema = z.object({
  units: z.array(z.object({
    name: z.string().describe("Business unit name"),
    weight: z.coerce.number().describe("Portfolio weight"),
    standalone_var: z.coerce.number().describe("Standalone VaR"),
    returns: z.array(z.coerce.number()).describe("Historical return series"),
  })).describe("Business units with risk data"),
  portfolio_var: z.coerce.number().describe("Total portfolio VaR"),
  epsilon: z.coerce.number().describe("Perturbation size for marginal computation"),
});

export const ShapleyAllocationSchema = z.object({
  units: z.array(z.object({
    name: z.string().describe("Business unit name"),
    returns: z.array(z.coerce.number()).describe("Historical return series"),
  })).describe("Business units with return series"),
  confidence_level: z.coerce.number().describe("Confidence level for VaR (e.g. 0.95)"),
  num_samples: z.coerce.number().int().optional().describe("Number of samples for approximation (default: exact for N<=8)"),
});

export const LimitManagementSchema = z.object({
  limits: z.array(z.object({
    name: z.string().describe("Limit name or identifier"),
    limit_type: z.enum(["Notional", "VaR", "Concentration", "Sector", "Country"]).describe("Type of risk limit"),
    limit_value: z.coerce.number().describe("Maximum allowed value"),
    current_value: z.coerce.number().describe("Current utilization value"),
    warning_threshold: z.coerce.number().describe("Warning threshold as fraction of limit (e.g. 0.8)"),
  })).describe("Risk limits to evaluate"),
});
