import { z } from 'zod';
import { SingleIdSchema, IdentifierSchema, DateRangeSchema, PaginationSchema } from './common.js';

export const SupplyChainSchema = SingleIdSchema.extend({
  direction: z.enum(['suppliers', 'customers']).optional().describe('Supply chain direction: suppliers or customers'),
});

export const GeoRevenueSchema = SingleIdSchema;

export const EventsSchema = IdentifierSchema.merge(DateRangeSchema).extend({
  type: z.string().optional().describe('Event type filter (e.g., earnings, conference, filing)'),
});

export const PeopleSchema = SingleIdSchema;

export const MaDealsSchema = z.object({
  target: z.string().optional().describe('Target company identifier'),
  acquirer: z.string().optional().describe('Acquirer company identifier'),
}).merge(DateRangeSchema).merge(PaginationSchema);
