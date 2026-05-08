import { z } from "zod"

export const RenderTemplateInputSchema = z.object({ "kind": z.any(), "result_json": z.string() })
