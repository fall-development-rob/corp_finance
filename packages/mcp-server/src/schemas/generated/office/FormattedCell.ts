import { z } from "zod"

export const FormattedCellSchema = z.object({ "col": z.number().int().gte(0).describe("0-indexed worksheet column."), "format": z.object({ "bold": z.boolean().default(false), "italic": z.boolean().default(false), "num_format": z.union([z.string(), z.null()]).default(null) }).describe("Format to apply at this coordinate."), "row": z.number().int().gte(0).describe("0-indexed worksheet row.") }).describe("A format overlay applied to a specific (row, col) after all data writes.")
