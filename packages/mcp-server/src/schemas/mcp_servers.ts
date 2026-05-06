import { z } from "zod";

export const McpServerListSchema = z.object({
  tier: z
    .enum([
      "free_native",
      "free_public_with_api_key",
      "freemium",
      "paid_vendor",
    ])
    .optional()
    .describe(
      "Filter by cost tier. free_native: bundled with cfa-core, fully offline. free_public_with_api_key: free public-data sources, some need a free API key. freemium: vendor offers a free tier (FMP). paid_vendor: requires a paid vendor subscription."
    ),
});
