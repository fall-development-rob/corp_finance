import { z } from "zod"

export const SlideDeckSpecSchema = z.object({ "properties": z.any().default({"author":null,"company":null,"subject":null,"title":null}), "slides": z.array(z.any()) }).describe("Top-level description of a PowerPoint deck to be written.")
