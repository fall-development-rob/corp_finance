import { z } from "zod"

export const RenderDeckTemplateInputSchema = z.object({ "input_json": z.string(), "kind": z.enum(["pitch_deck","ic_presentation"]) })
