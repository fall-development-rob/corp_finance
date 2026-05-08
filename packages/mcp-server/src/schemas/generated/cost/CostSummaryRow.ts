import { z } from "zod"

export const CostSummaryRowSchema = z.object({ "cents": z.number().int().describe("Total cents in the bucket."), "count": z.number().int().gte(0).describe("Number of events in the bucket."), "key": z.string().describe("String key of the bucket (e.g. `\"cli\"`, `\"mcp_paid_vendor\"`,\n`\"cli|mcp_freemium\"`, `\"default\"`)."), "tokens_in": z.number().int().gte(0).describe("Total input tokens in the bucket."), "tokens_out": z.number().int().gte(0).describe("Total output tokens in the bucket.") }).describe("Per-bucket aggregate.")
