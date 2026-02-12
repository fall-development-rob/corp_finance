import { z } from "zod";

export const CreditScorecardSchema = z.object({
  bins: z.array(z.object({
    lower: z.coerce.number().describe("Lower bound of bin"),
    upper: z.coerce.number().describe("Upper bound of bin"),
    good_count: z.coerce.number().int().describe("Number of good observations in bin"),
    bad_count: z.coerce.number().int().describe("Number of bad observations in bin"),
  })).describe("Score bins with good/bad counts for WoE calculation"),
  target_score: z.coerce.number().describe("Target scorecard score (e.g. 600)"),
  target_odds: z.coerce.number().describe("Target odds at target score (e.g. 50:1)"),
  pdo: z.coerce.number().describe("Points to double the odds (e.g. 20)"),
});

export const MertonPdSchema = z.object({
  equity_value: z.coerce.number().describe("Market value of equity"),
  equity_vol: z.coerce.number().describe("Equity volatility as decimal"),
  debt_face: z.coerce.number().describe("Face value of debt (default barrier)"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate as decimal"),
  maturity: z.coerce.number().describe("Time to maturity in years"),
  growth_rate: z.coerce.number().describe("Asset growth rate (drift) as decimal"),
});

export const IntensityModelSchema = z.object({
  cds_spreads: z.array(z.object({
    tenor: z.coerce.number().describe("CDS tenor in years"),
    spread: z.coerce.number().describe("CDS spread in basis points"),
  })).describe("CDS spread term structure"),
  recovery_rate: z.coerce.number().describe("Recovery rate as decimal (e.g. 0.4)"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate as decimal"),
});

export const PdCalibrationSchema = z.object({
  pd_input: z.coerce.number().describe("Input PD as decimal"),
  macro_index: z.coerce.number().describe("Macro-economic index value (z-score)"),
  direction: z.enum(["TtcToPit", "PitToTtc"]).describe("Calibration direction"),
  correlation_override: z.coerce.number().optional().describe("Override asset correlation (default: Basel IRB formula)"),
});

export const ScoringValidationSchema = z.object({
  observations: z.array(z.object({
    predicted: z.coerce.number().describe("Predicted probability of default"),
    actual: z.coerce.number().describe("Actual outcome (0 or 1)"),
  })).describe("Model predictions vs actuals"),
  num_bins: z.coerce.number().int().optional().describe("Number of bins for Hosmer-Lemeshow test (default 10)"),
});
