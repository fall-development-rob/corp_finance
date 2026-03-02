import { z } from 'zod';

export const BatchRequestSchema = z.object({
  requests: z.array(z.object({
    endpoint: z.string().describe('API endpoint path'),
    params: z.record(z.unknown()).describe('Request parameters'),
  })).describe('Array of batch requests to execute'),
});
