import { z } from "zod"

export const DocSectionSchema = z.object({ "blocks": z.array(z.any()) }).describe("A contiguous section of content blocks.")
