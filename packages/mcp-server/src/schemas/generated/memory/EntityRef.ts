import { z } from "zod"

export const EntityRefSchema = z.object({ "kind": z.enum(["ticker","issuer","sector","fund","cusip"]).describe("Canonical entity kinds extracted from `RunSummary` text.\n\nShared kernel with the multi-agent coordination context (Phase 27); the\n`EntityRef` value object also lives there once that context lands."), "value": z.string() }).describe("A single named entity associated with a `RunSummary`.")
