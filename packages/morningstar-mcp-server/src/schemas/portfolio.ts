import { z } from 'zod';
import { FundIdentifierSchema } from './common.js';

export const PortfolioXraySchema = z.object({
  holdings: z.array(z.object({
    ticker: z.string().min(1).describe('Holding ticker symbol'),
    weight: z.number().min(0).max(1).describe('Portfolio weight (0-1)'),
  })).min(1).describe('Portfolio holdings with weights'),
});

export const AssetAllocationSchema = FundIdentifierSchema;

export const PeerComparisonSchema = FundIdentifierSchema.extend({
  category: z.string().optional().describe('Morningstar category for comparison'),
});
