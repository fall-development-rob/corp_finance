import { z } from "zod"

export const WriteWorkbookResultSchema = z.object({ "bytes_written": z.number().int().gte(0), "output_path": z.string(), "sha256": z.string().describe("SHA-256 of the written file, 64 lowercase hex chars."), "sheet_count": z.number().int().gte(0) }).describe("Returned by [`crate::office::xlsx::write_workbook`] on success.")
