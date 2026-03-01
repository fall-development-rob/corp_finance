import { z } from 'zod';
import { IssuerIdentifierSchema } from './common.js';

export const EsgScoreSchema = IssuerIdentifierSchema;

export const ClimateRiskSchema = IssuerIdentifierSchema.extend({
  scenario: z.string().optional().describe('Climate scenario (e.g., RCP2.6, RCP4.5, RCP8.5)'),
});
