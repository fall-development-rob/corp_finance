import { z } from "zod"

export const WriteDocResultSchema = z.object({ "bytes_written": z.number().int().gte(0), "output_path": z.string(), "section_count": z.number().int().gte(0), "sha256": z.string().describe("SHA-256 of the written file, 64 lowercase hex chars.") }).describe("Returned by [`crate::office::docx::write_word_doc`] on success.")
