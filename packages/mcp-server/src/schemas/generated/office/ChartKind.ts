import { z } from "zod"

export const ChartKindSchema = z.enum(["line","bar","column","pie"]).describe("Chart variety supported by the xlsx writer.")
