import { z } from "zod"

export const CellValueSchema = z.any().superRefine((x, ctx) => {
    const schemas = [z.object({ "kind": z.literal("text"), "value": z.string() }), z.object({ "kind": z.literal("number"), "value": z.number() }), z.object({ "kind": z.literal("decimal"), "value": z.string() }).describe("Decimal preserved on the wire as a string; converted to f64 at the\nxlsx cell boundary. Excel is f64-native; this conversion is the\ndocumented exception to the no-f64 invariant."), z.object({ "kind": z.literal("bool"), "value": z.boolean() }), z.object({ "excel_format": z.union([z.string(), z.null()]).optional(), "kind": z.literal("date_time"), "value": z.string() }).describe("RFC3339 timestamp string. Parsed at write time and rendered as an\nExcel date number with the format defined by `excel_format`."), z.object({ "kind": z.literal("empty") })];
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
  }).describe("A single cell value in a data row.")
