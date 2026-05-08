import { z } from "zod"

export const WorkbookSpecSchema = z.object({ "defined_names": z.array(z.any()).default([]), "properties": z.any().default({"author":null,"company":null,"subject":null,"title":null}), "sheets": z.array(z.any()) }).describe("Top-level description of an Excel workbook to be written.")
