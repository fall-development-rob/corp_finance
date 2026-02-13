import { z } from 'zod';
import { SymbolSchema, SymbolLimitSchema, DateRangeSchema } from './common.js';

export const EarningsSchema = SymbolLimitSchema;

export const EarningsCalendarSchema = DateRangeSchema;

export const EarningsTranscriptSchema = SymbolSchema.extend({
  year: z.number().int().min(2000).describe('Fiscal year'),
  quarter: z.number().int().min(1).max(4).describe('Fiscal quarter (1-4)'),
});

export const AnalystEstimatesSchema = SymbolSchema.extend({
  period: z.enum(['annual', 'quarter']).default('annual').describe('Estimate period'),
  limit: z.number().int().min(1).max(30).default(4).describe('Number of periods'),
});

export const PriceTargetSchema = SymbolSchema;
export const GradesSchema = SymbolLimitSchema;
