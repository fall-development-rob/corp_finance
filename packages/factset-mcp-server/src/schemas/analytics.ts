import { z } from 'zod';
import { IdentifierSchema, DateRangeSchema } from './common.js';

export const PortfolioAnalyticsSchema = z.object({
  portfolio_id: z.string().describe('Portfolio identifier'),
  benchmark_id: z.string().optional().describe('Benchmark identifier for relative analytics'),
}).merge(DateRangeSchema);

export const RiskModelSchema = z.object({
  portfolio_id: z.string().describe('Portfolio identifier'),
  risk_model: z.string().optional().describe('Risk model to use (e.g., Axioma, Barra)'),
});

export const FactorExposureSchema = IdentifierSchema.extend({
  factor_model: z.string().optional().describe('Factor model to use (e.g., Fama-French, Barra)'),
});
