import { z } from 'zod';

export const SecFilingsDateSchema = z.object({
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(500).default(100).describe('Results per page'),
});

export const SecFilingsByFormSchema = z.object({
  formType: z.string().min(1).describe('SEC form type (e.g., 8-K, 10-K, 10-Q, S-1)'),
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(500).default(100).describe('Results per page'),
});

export const SecFilingsBySymbolSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(500).default(100).describe('Results per page'),
});

export const SecFilingsByCikSchema = z.object({
  cik: z.string().min(1).describe('CIK number (e.g., 0000320193)'),
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(500).default(100).describe('Results per page'),
});

export const SecCompanySearchNameSchema = z.object({
  company: z.string().min(1).describe('Company name to search'),
});

export const SecCompanySearchSymbolSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
});

export const SecCompanySearchCikSchema = z.object({
  cik: z.string().min(1).describe('CIK number'),
});

export const SecProfileSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
});
