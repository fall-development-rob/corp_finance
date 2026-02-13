import { z } from 'zod';

export const InsiderLatestSchema = z.object({
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(500).default(100).describe('Results per page'),
});

export const InsiderSearchSchema = z.object({
  symbol: z.string().optional().describe('Filter by ticker symbol'),
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(500).default(100).describe('Results per page'),
});

export const InsiderByNameSchema = z.object({
  name: z.string().min(1).describe('Insider name to search (e.g., Zuckerberg)'),
});

export const InsiderStatsSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
});
