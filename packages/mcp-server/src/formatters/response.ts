/**
 * Identity pass-through. String-to-number coercion is now handled by
 * z.coerce.number() in every Zod schema, so the MCP SDK coerces before
 * the handler runs. This function is retained for API compatibility but
 * no longer mutates data (the old recursive coercion broke string fields
 * whose values happened to look like numbers, e.g. period_name "2020").
 */
export function coerceNumbers(obj: unknown): unknown {
  return obj;
}

export function wrapResponse(resultJson: unknown) {
  if (resultJson instanceof Error) {
    return {
      content: [{ type: "text" as const, text: JSON.stringify({ error: resultJson.message }) }],
      isError: true,
    };
  }
  return {
    content: [{ type: "text" as const, text: String(resultJson) }],
  };
}
