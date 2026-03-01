import { z } from 'zod';

export const IdentifierSchema = z.object({
  ids: z.array(z.string()).describe('FactSet identifiers (fsym_id, ticker, SEDOL, ISIN)'),
});

export const SingleIdSchema = z.object({
  id: z.string().describe('Single FactSet identifier (fsym_id, ticker, SEDOL, ISIN)'),
});

export const DateRangeSchema = z.object({
  start_date: z.string().optional().describe('Start date YYYY-MM-DD'),
  end_date: z.string().optional().describe('End date YYYY-MM-DD'),
});

export const PaginationSchema = z.object({
  limit: z.number().int().min(1).max(10000).default(100).describe('Max results to return'),
  offset: z.number().int().min(0).default(0).describe('Offset for pagination'),
});
