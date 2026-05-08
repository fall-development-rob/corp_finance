import { z } from "zod"

export const GroupBySchema = z.enum(["surface", "tier", "tenant", "surface_and_tier"]).describe("What dimension to bucket the summary on.")
