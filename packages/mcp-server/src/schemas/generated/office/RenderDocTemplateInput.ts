import { z } from "zod"

export const RenderDocTemplateInputSchema = z.object({ "input_json": z.string(), "kind": z.enum(["ic_memo","research_init"]) })
