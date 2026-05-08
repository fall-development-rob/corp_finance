import { z } from "zod"

export const DeckTemplateKindSchema = z.enum(["pitch_deck","ic_presentation"])
