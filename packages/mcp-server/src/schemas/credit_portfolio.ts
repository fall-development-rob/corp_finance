import { z } from "zod";

const CreditExposureSchema = z.object({
  name: z.string().describe("Name or identifier of the credit exposure"),
  exposure: z.coerce.number().describe("Exposure amount (notional or market value)"),
  probability_of_default: z.coerce.number().describe("Probability of default (decimal, e.g. 0.02 for 2%)"),
  loss_given_default: z.coerce.number().describe("Loss given default (decimal, e.g. 0.45 for 45%)"),
  rating: z.string().describe("Credit rating (e.g. 'AA', 'BBB', 'B')"),
  sector: z.string().describe("Industry sector (e.g. 'Technology', 'Energy')"),
  maturity_years: z.coerce.number().describe("Remaining maturity in years"),
});

export const PortfolioRiskSchema = z.object({
  portfolio_name: z.string().describe("Name of the credit portfolio"),
  exposures: z.array(CreditExposureSchema).describe("Individual credit exposures in the portfolio"),
  default_correlation: z.coerce.number().describe("Default correlation between exposures (decimal, e.g. 0.3)"),
  confidence_level: z.coerce.number().describe("Confidence level for VaR/ES calculation (decimal, e.g. 0.99 for 99%)"),
  time_horizon_years: z.coerce.number().describe("Risk measurement time horizon in years"),
});

const RatedExposureSchema = z.object({
  name: z.string().describe("Name or identifier of the rated exposure"),
  rating: z.string().describe("Current credit rating (e.g. 'AA', 'BBB')"),
  exposure: z.coerce.number().describe("Exposure amount (notional or market value)"),
  maturity_years: z.coerce.number().describe("Remaining maturity in years"),
  coupon_rate: z.coerce.number().describe("Annual coupon rate (decimal, e.g. 0.05 for 5%)"),
});

const TransitionMatrixSchema = z.object({
  ratings: z.array(z.string()).describe("Ordered list of rating categories (e.g. ['AAA','AA','A','BBB','BB','B','CCC','D'])"),
  probabilities: z.array(z.array(z.coerce.number())).describe("NxN matrix of transition probabilities (rows sum to 1)"),
});

const RatingSpreadSchema = z.object({
  rating: z.string().describe("Credit rating category"),
  spread_bps: z.coerce.number().describe("Credit spread in basis points for this rating"),
});

export const MigrationSchema = z.object({
  initial_ratings: z.array(RatedExposureSchema).describe("Exposures with their current ratings"),
  transition_matrix: TransitionMatrixSchema.describe("Rating transition probability matrix"),
  time_horizon_years: z.coerce.number().int().min(1).describe("Migration analysis time horizon in years (integer)"),
  spread_curve: z.array(RatingSpreadSchema).describe("Credit spreads by rating for revaluation"),
});
