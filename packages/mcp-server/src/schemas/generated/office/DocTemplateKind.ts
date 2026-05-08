import { z } from "zod"

export const DocTemplateKindSchema = z.enum(["ic_memo","research_init","cim","sector_overview","earnings_update"])
