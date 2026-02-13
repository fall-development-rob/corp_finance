import { z } from 'zod';

export const InstitutionalLatestSchema = z.object({
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(500).default(100).describe('Results per page'),
});

export const InstitutionalExtractSchema = z.object({
  cik: z.string().min(1).describe('Institutional holder CIK (e.g., 0001388838)'),
  year: z.number().int().min(2000).describe('Filing year'),
  quarter: z.number().int().min(1).max(4).describe('Filing quarter'),
});

export const InstitutionalDatesSchema = z.object({
  cik: z.string().min(1).describe('Institutional holder CIK'),
});

export const InstitutionalAnalyticsByHolderSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
  year: z.number().int().min(2000).describe('Year'),
  quarter: z.number().int().min(1).max(4).describe('Quarter'),
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(100).default(10).describe('Results per page'),
});

export const HolderPerformanceSchema = z.object({
  cik: z.string().min(1).describe('Institutional holder CIK'),
  page: z.number().int().min(0).default(0).describe('Page number'),
});

export const HolderIndustrySchema = z.object({
  cik: z.string().min(1).describe('Institutional holder CIK'),
  year: z.number().int().min(2000).describe('Year'),
  quarter: z.number().int().min(1).max(4).describe('Quarter'),
});

export const PositionsSummarySchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
  year: z.number().int().min(2000).describe('Year'),
  quarter: z.number().int().min(1).max(4).describe('Quarter'),
});

export const IndustrySummarySchema = z.object({
  year: z.number().int().min(2000).describe('Year'),
  quarter: z.number().int().min(1).max(4).describe('Quarter'),
});
