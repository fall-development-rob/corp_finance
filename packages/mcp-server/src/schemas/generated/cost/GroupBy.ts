import { z } from "zod"

export const GroupBySchema = z.union([z.literal("surface").describe("One row per `Surface` (cli / mcp / skill / plugin)."), z.literal("tier").describe("One row per `TierTag` value (cookbook_* / mcp_* / unknown)."), z.literal("tenant").describe("One row per `tenant_id` (NULL collapsed to \"default\")."), z.literal("surface_and_tier").describe("One row per `(surface, tier)` pair.")]).describe("What dimension to bucket the summary on.")
