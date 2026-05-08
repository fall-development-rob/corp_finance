import { z } from "zod"

export const ChartSchema = z.object({ "anchor_col": z.number().int().gte(0).describe("0-indexed column of the chart's top-left anchor cell."), "anchor_row": z.number().int().gte(0).describe("0-indexed row of the chart's top-left anchor cell."), "kind": z.any(), "series": z.array(z.any()).describe("At least one series is required for a valid chart."), "title": z.union([z.string(), z.null()]).default(null) }).describe("A chart object embedded in a worksheet at `(anchor_row, anchor_col)`.")
