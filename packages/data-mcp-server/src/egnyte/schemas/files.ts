import { z } from 'zod';

export const EgnyteListFilesSchema = z.object({
  folder_path: z.string().min(1).describe('Path under root, e.g. "/Shared/DealRoom/AAPL"'),
  recursive: z.boolean().default(false),
  limit: z.number().int().min(1).max(500).default(50),
});

export const EgnyteGetFileMetadataSchema = z.object({
  file_path: z.string().min(1).describe('Full path to file, e.g. "/Shared/DealRoom/AAPL/cim.pdf"'),
});

export const EgnyteSearchSchema = z.object({
  query: z.string().min(1).describe('Full-text query'),
  folder_scope: z.string().optional().describe('Restrict to a subtree, e.g. "/Shared/DealRoom"'),
  file_type: z.enum(['all', 'document', 'spreadsheet', 'pdf', 'image']).default('all'),
  limit: z.number().int().min(1).max(100).default(20),
});

export const EgnyteDownloadFileSchema = z.object({
  file_path: z.string().min(1),
  return_content: z.boolean().default(false).describe(
    'If true, return file bytes (base64); if false, return a download URL.',
  ),
});
