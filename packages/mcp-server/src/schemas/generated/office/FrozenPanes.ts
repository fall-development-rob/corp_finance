import { z } from "zod"

export const FrozenPanesSchema = z.object({ "col": z.number().int().gte(0), "row": z.number().int().gte(0) }).describe("Freeze row/col coordinates (0-indexed).")
