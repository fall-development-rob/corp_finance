import { z } from 'zod';
import { SeriesIdSchema, DateRangeSchema, LimitSchema, FrequencySchema, SearchSchema } from './common.js';

export const SeriesObservationsSchema = SeriesIdSchema
  .merge(DateRangeSchema)
  .merge(FrequencySchema)
  .merge(LimitSchema);

export const SeriesInfoSchema = SeriesIdSchema;

export const SeriesSearchSchema = SearchSchema;

export const SeriesCategoriesSchema = SeriesIdSchema;

export const SeriesTagsSchema = SeriesIdSchema;

export const SeriesVintageSchema = SeriesIdSchema.merge(z.object({
  limit: z.number().int().min(1).max(10000).default(1000).describe('Max vintage dates'),
}));
