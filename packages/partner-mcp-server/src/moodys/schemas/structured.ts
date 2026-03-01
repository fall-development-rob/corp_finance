import { z } from 'zod';
import { IssuerIdentifierSchema, PaginationSchema } from './common.js';

export const StructuredFinanceSchema = z.object({
  deal_id: z.string().optional().describe('Structured finance deal identifier'),
  asset_class: z.enum(['CMBS', 'RMBS', 'ABS', 'CLO']).optional().describe('Asset class filter'),
}).merge(PaginationSchema);

export const MunicipalScoreSchema = z.object({
  issuer: z.string().describe('Municipal issuer name'),
  state: z.string().optional().describe('US state abbreviation (e.g., CA, NY, TX)'),
});

export const CompanyFinancialsSchema = IssuerIdentifierSchema.extend({
  period: z.enum(['annual', 'quarterly']).optional().describe('Reporting period (annual or quarterly)'),
});
