import { z } from "zod"

export const DocBlockSchema = z.any().superRefine((x, ctx) => {
    const schemas = [z.object({ "kind": z.literal("heading"), "level": z.number().int().gte(0).lte(255), "text": z.string() }), z.object({ "kind": z.literal("paragraph"), "runs": z.array(z.any()) }), z.object({ "headers": z.array(z.string()), "kind": z.literal("table"), "rows": z.array(z.array(z.string())) }), z.object({ "items": z.array(z.string()), "kind": z.literal("bullet_list") }), z.object({ "items": z.array(z.string()), "kind": z.literal("numbered_list") }), z.object({ "kind": z.literal("page_break") })];
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
  }).describe("A single content block within a document section.")
