import { z } from "zod"

export const FormattedCellSchema = z.object({ "col": z.number().int().gte(0).describe("0-indexed worksheet column."), "format": z.any().describe("Format to apply at this coordinate."), "row": z.number().int().gte(0).describe("0-indexed worksheet row.") }).describe("A format overlay applied to a specific (row, col) after all data writes.")
