import { z } from "zod"

export const AttestationRevocationSchema = z.object({ "attestation_id": z.string().uuid(), "reason": z.string(), "revoked_at": z.string().datetime({ offset: true }), "revoked_by": z.string() })
