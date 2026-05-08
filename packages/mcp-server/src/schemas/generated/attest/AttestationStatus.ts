import { z } from "zod"

export const AttestationStatusSchema = z.enum(["valid", "expired", "revoked", "bad_signature", "issuer_mismatch"])
