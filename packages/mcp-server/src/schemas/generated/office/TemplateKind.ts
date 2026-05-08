import { z } from "zod"

export const TemplateKindSchema = z.enum(["dcf","comps","lbo","three_statement"])
