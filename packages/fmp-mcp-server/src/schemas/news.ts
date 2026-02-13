import { z } from 'zod';

// Shared pagination for news
export const PageLimitSchema = z.object({
  page: z.number().int().min(0).default(0).describe('Page number (0-indexed)'),
  limit: z.number().int().min(1).max(100).default(20).describe('Results per page'),
});

export const SymbolNewsSchema = z.object({
  symbols: z.string().min(1).describe('Comma-separated ticker symbols (e.g., AAPL,MSFT)'),
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(100).default(20).describe('Results per page'),
});
