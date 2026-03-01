import { z } from 'zod';

export const TagsSchema = z.object({
  limit: z.number().int().min(1).max(1000).default(100).describe('Max tags'),
  offset: z.number().int().min(0).default(0).describe('Offset for pagination'),
  search_text: z.string().optional().describe('Filter tags by search text'),
});

export const RelatedTagsSchema = z.object({
  tag_names: z.string().min(1).describe('Semicolon-delimited tag names (e.g., "monetary aggregates;weekly")'),
  limit: z.number().int().min(1).max(1000).default(100).describe('Max tags'),
});

export const TagsSeriesSchema = z.object({
  tag_names: z.string().min(1).describe('Semicolon-delimited tag names (e.g., "slovenia;food;oecd")'),
  limit: z.number().int().min(1).max(1000).default(100).describe('Max series'),
});
