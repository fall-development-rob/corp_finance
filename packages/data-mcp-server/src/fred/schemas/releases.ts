import { z } from 'zod';

export const ReleasesSchema = z.object({
  limit: z.number().int().min(1).max(1000).default(100).describe('Max releases'),
  offset: z.number().int().min(0).default(0).describe('Offset for pagination'),
});

export const ReleaseSchema = z.object({
  release_id: z.number().int().min(1).describe('FRED release ID'),
});

export const ReleaseDatesSchema = z.object({
  release_id: z.number().int().min(1).describe('FRED release ID'),
  limit: z.number().int().min(1).max(1000).default(100).describe('Max dates'),
});

export const ReleaseSeriesSchema = z.object({
  release_id: z.number().int().min(1).describe('FRED release ID'),
  limit: z.number().int().min(1).max(1000).default(100).describe('Max series'),
});
