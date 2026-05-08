import { z } from "zod"

export const RenderDocTemplateInputSchema = z.object({ "input_json": z.string(), "kind": z.enum(["ic_memo","research_init","cim","sector_overview","earnings_update"]) })
