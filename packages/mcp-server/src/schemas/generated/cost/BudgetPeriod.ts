import { z } from "zod"

export const BudgetPeriodSchema = z.union([z.enum(["daily","weekly","monthly"]), z.literal("total").describe("All-time cumulative.")]).describe("The period a budget covers.")
