import { z } from "zod"

export const RenderTemplateInputSchema = z.object({ "kind": z.enum(["dcf","comps","lbo","three_statement"]), "result_json": z.string() })
