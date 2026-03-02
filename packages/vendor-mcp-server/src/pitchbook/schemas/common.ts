import { z } from 'zod';

export const EntityIdentifierSchema = z.object({
  entity_id: z.string().optional().describe('PitchBook entity ID'),
  name: z.string().optional().describe('Entity name'),
});

export const PaginationSchema = z.object({
  page: z.number().int().min(1).default(1).describe('Page number'),
  page_size: z.number().int().min(1).max(100).default(25).describe('Results per page'),
});

export const DateRangeSchema = z.object({
  start_date: z.string().optional().describe('Start date YYYY-MM-DD'),
  end_date: z.string().optional().describe('End date YYYY-MM-DD'),
});
