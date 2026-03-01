import { z } from 'zod';
import { FundIdentifierSchema } from './common.js';

export const FairValueSchema = z.object({
  ticker: z.string().min(1).describe('Stock ticker symbol'),
});

export const MoatRatingSchema = z.object({
  ticker: z.string().min(1).describe('Stock ticker symbol'),
});

export const EsgRiskSchema = FundIdentifierSchema;

export const AnalystReportSchema = z.object({
  ticker: z.string().min(1).describe('Stock ticker symbol'),
});

export const CompanyProfileSchema = z.object({
  ticker: z.string().min(1).describe('Stock ticker symbol'),
});
