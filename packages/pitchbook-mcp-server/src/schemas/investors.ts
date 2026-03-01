import { z } from 'zod';
import { EntityIdentifierSchema, PaginationSchema } from './common.js';

export const InvestorProfileSchema = EntityIdentifierSchema;

export const FundSearchSchema = z.object({
  manager: z.string().optional().describe('Fund manager name'),
  strategy: z.string().optional().describe('Investment strategy (buyout, growth, venture, etc.)'),
  vintage: z.number().optional().describe('Fund vintage year'),
}).merge(PaginationSchema);

export const FundPerformanceSchema = z.object({
  fund_id: z.string().min(1).describe('PitchBook fund ID'),
});

export const LpCommitmentsSchema = EntityIdentifierSchema.merge(PaginationSchema);
