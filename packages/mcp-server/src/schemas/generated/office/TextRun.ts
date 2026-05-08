import { z } from "zod"

export const TextRunSchema = z.object({ "bold": z.boolean().default(false), "italic": z.boolean().default(false), "text": z.string() }).describe("A styled text run within a paragraph.")
