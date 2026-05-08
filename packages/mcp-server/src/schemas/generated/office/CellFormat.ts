import { z } from "zod"

export const CellFormatSchema = z.object({ "bold": z.boolean().default(false), "italic": z.boolean().default(false), "num_format": z.union([z.string(), z.null()]).default(null) }).describe("Minimal cell-level format overlay applied after value writes.\n`num_format` follows Excel number format syntax (e.g. `\"$#,##0.00\"`, `\"0.00%\"`).")
