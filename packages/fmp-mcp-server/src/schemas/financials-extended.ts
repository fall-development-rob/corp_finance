import { z } from 'zod';

export const SymbolOnlySchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol (e.g., AAPL)'),
});

export const SymbolPeriodLimitSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
  period: z.enum(['annual', 'quarter']).default('annual').describe('Reporting period'),
  limit: z.number().int().min(1).max(120).default(4).describe('Number of periods'),
});

export const LatestStatementsSchema = z.object({
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(250).default(100).describe('Results per page'),
});

export const FinancialReportsSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
  year: z.number().int().min(2000).describe('Fiscal year'),
  period: z.enum(['FY', 'Q1', 'Q2', 'Q3', 'Q4']).default('FY').describe('Filing period'),
});

export const AsReportedSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
  period: z.enum(['annual', 'quarter']).default('annual').describe('Reporting period'),
  limit: z.number().int().min(1).max(120).default(4).describe('Number of periods'),
});
