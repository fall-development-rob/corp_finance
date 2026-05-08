import { z } from "zod"

export const SurfaceSchema = z.enum(["cli", "mcp", "skill", "plugin"]).describe("One of the four CFA execution surfaces.\n\nEvery CFA runtime event lands on exactly one of these four boundaries.\nThe wire form is `snake_case` — for the current variant set this is\nbyte-identical to lowercase, but `snake_case` is the canonical choice\nto match the rest of the Phase-26 enum vocabulary (see `cost::TierTag`,\n`cost::BudgetPeriod`).")
