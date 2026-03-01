import { z } from 'zod';

export const CategorySchema = z.object({
  category_id: z.number().int().min(0).default(0).describe('FRED category ID (0 = root)'),
});

export const CategoryChildrenSchema = z.object({
  category_id: z.number().int().min(0).default(0).describe('FRED category ID'),
});

export const CategorySeriesSchema = z.object({
  category_id: z.number().int().min(0).describe('FRED category ID'),
  limit: z.number().int().min(1).max(1000).default(100).describe('Max series'),
});
