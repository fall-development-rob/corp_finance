import { z } from 'zod';
import { DateRangeSchema, PaginationSchema } from './common.js';

export const VcExitsSchema = z.object({
  exit_type: z.string().optional().describe('Exit type filter (IPO, M&A, secondary)'),
}).merge(DateRangeSchema).merge(PaginationSchema);

export const FundraisingSchema = z.object({
  strategy: z.string().optional().describe('Fund strategy (buyout, growth, venture, etc.)'),
}).merge(DateRangeSchema).merge(PaginationSchema);

export const MarketStatsSchema = z.object({
  sector: z.string().optional().describe('Sector filter'),
  geography: z.string().optional().describe('Geography filter (US, Europe, Asia, etc.)'),
}).merge(DateRangeSchema);

export const PeopleSearchSchema = z.object({
  name: z.string().min(1).describe('Person name to search'),
  role: z.string().optional().describe('Role filter (Partner, MD, VP, etc.)'),
}).merge(PaginationSchema);

export const ServiceProvidersSchema = z.object({
  type: z.string().optional().describe('Provider type (law firm, investment bank, consultant)'),
  geography: z.string().optional().describe('Geography filter'),
}).merge(PaginationSchema);
