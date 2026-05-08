import { z } from "zod"

export const AttestationStatusSchema = z.any().superRefine((x, ctx) => {
    const schemas = [z.literal("valid").describe("signature valid, current time is within [issued_at, expires_at], not revoked"), z.literal("expired").describe("signature valid, current time > expires_at"), z.literal("revoked").describe("listed in attestation_revocations"), z.literal("bad_signature").describe("signature does not verify against public_key on canonical payload"), z.literal("issuer_mismatch").describe("canonical-payload-internal mismatch (e.g. expected_issuer mismatches stored issuer)")];
    const { errors, failed } = schemas.reduce<{
      errors: z.ZodError[];
      failed: number;
    }>(
      ({ errors, failed }, schema) =>
        ((result) =>
          result.error
            ? {
                errors: [...errors, result.error],
                failed: failed + 1,
              }
            : { errors, failed })(
          schema.safeParse(x),
        ),
      { errors: [], failed: 0 },
    );
    const passed = schemas.length - failed;
    if (passed !== 1) {
      ctx.addIssue(errors.length ? {
        path: ctx.path,
        code: "invalid_union",
        unionErrors: errors,
        message: "Invalid input: Should pass single schema. Passed " + passed,
      } : {
        path: ctx.path,
        code: "custom",
        message: "Invalid input: Should pass single schema. Passed " + passed,
      });
    }
  })
