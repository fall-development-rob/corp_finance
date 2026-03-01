import { z } from 'zod';

export const IdTypeEnum = z.enum([
  'ID_ISIN',
  'ID_CUSIP',
  'ID_SEDOL',
  'ID_BB_GLOBAL',
  'TICKER',
  'ID_WERTPAPIER',
  'ID_COMMON',
  'COMPOSITE_FIGI',
]).describe('Type of identifier');

export const MappingJobSchema = z.object({
  idType: IdTypeEnum,
  idValue: z.string().min(1).describe('Identifier value'),
  exchCode: z.string().optional().describe('Exchange code (e.g., US, LN)'),
  micCode: z.string().optional().describe('Market Identifier Code'),
  currency: z.string().optional().describe('Currency code'),
});

export const SingleMappingSchema = z.object({
  idType: IdTypeEnum,
  idValue: z.string().min(1).describe('Identifier value'),
  exchCode: z.string().optional().describe('Exchange code (e.g., US, LN)'),
  micCode: z.string().optional().describe('Market Identifier Code'),
  currency: z.string().optional().describe('Currency code'),
});

export const BulkMappingSchema = z.object({
  jobs: z.array(MappingJobSchema).min(1).max(100).describe('Array of mapping jobs (max 100)'),
});

export const IsinMappingSchema = z.object({
  isin: z.string().min(1).describe('ISIN code (e.g., US0378331005 for Apple)'),
  exchCode: z.string().optional().describe('Exchange code to narrow results'),
  micCode: z.string().optional().describe('Market Identifier Code to narrow results'),
});

export const CusipMappingSchema = z.object({
  cusip: z.string().min(1).describe('CUSIP code (e.g., 037833100 for Apple)'),
  exchCode: z.string().optional().describe('Exchange code to narrow results'),
  micCode: z.string().optional().describe('Market Identifier Code to narrow results'),
});

export const SearchSchema = z.object({
  query: z.string().min(1).describe('Search keyword (e.g., company name or partial ticker)'),
  exchCode: z.string().optional().describe('Exchange code filter'),
  micCode: z.string().optional().describe('Market Identifier Code filter'),
});

export const FilterSchema = z.object({
  exchCode: z.string().optional().describe('Exchange code (e.g., US, LN)'),
  micCode: z.string().optional().describe('Market Identifier Code'),
  currency: z.string().optional().describe('Currency code (e.g., USD, GBP)'),
  securityType: z.string().optional().describe('Security type (e.g., Common Stock, ETP)'),
  securityType2: z.string().optional().describe('Security sub-type'),
  marketSector: z.string().optional().describe('Market sector (e.g., Equity, Govt)'),
  ticker: z.string().optional().describe('Ticker to filter by'),
});

export const EnumerationsSchema = z.object({
  key: z.string().min(1).describe('Enumeration key (e.g., exchCode, micCode, securityType, securityType2, marketSector, currency)'),
});

export const FigiLookupSchema = z.object({
  figi: z.string().min(1).describe('FIGI identifier (e.g., BBG000B9XRY4)'),
});
