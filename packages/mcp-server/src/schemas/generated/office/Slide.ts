import { z } from "zod"

export const SlideSchema = z.any().superRefine((x, ctx) => {
    const schemas = [z.object({ "kind": z.literal("title"), "subtitle": z.union([z.string(), z.null()]).default(null), "title": z.string() }).describe("Full-bleed title slide: large title + optional subtitle."), z.object({ "heading": z.string(), "kind": z.literal("section") }).describe("Section divider slide with a single centred heading."), z.object({ "bullets": z.array(z.string()).default([]), "kind": z.literal("content"), "title": z.string() }).describe("Title at top + bulleted body text."), z.object({ "headers": z.array(z.string()), "kind": z.literal("table"), "rows": z.array(z.array(z.string())), "title": z.string() }).describe("Title at top + a simple table (header row + N data rows).")];
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
  }).describe("A single slide in a deck. Tagged union on `kind` (snake_case).")
