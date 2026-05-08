import { z } from "zod"

export const WordDocSpecSchema = z.object({ "properties": z.any().default({"author":null,"company":null,"subject":null,"title":null}), "sections": z.array(z.any()) }).describe("Top-level description of a Word document to be written.")
