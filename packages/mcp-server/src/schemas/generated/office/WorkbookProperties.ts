import { z } from "zod"

export const WorkbookPropertiesSchema = z.object({ "author": z.union([z.string(), z.null()]).default(null), "company": z.union([z.string(), z.null()]).default(null), "subject": z.union([z.string(), z.null()]).default(null), "title": z.union([z.string(), z.null()]).default(null) }).describe("Workbook document properties written into the xlsx metadata.")
