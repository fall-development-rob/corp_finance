import { z } from 'zod';
import { FundIdentifierSchema } from './common.js';

export const EtfAnalyticsSchema = FundIdentifierSchema.extend({
  include_tracking_error: z.boolean().optional().describe('Include tracking error analysis'),
});
