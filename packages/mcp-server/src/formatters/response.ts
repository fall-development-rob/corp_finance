/**
 * Recursively coerce string values that look like numbers into actual numbers.
 * The MCP SDK sometimes passes numeric arguments as strings.
 */
export function coerceNumbers(obj: unknown): unknown {
  if (typeof obj === "string") {
    if (obj === "" || obj === "true" || obj === "false" || obj === "null") return obj;
    const n = Number(obj);
    if (!isNaN(n) && obj.trim() !== "") return n;
    return obj;
  }
  if (Array.isArray(obj)) return obj.map(coerceNumbers);
  if (obj !== null && typeof obj === "object") {
    const result: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
      result[k] = coerceNumbers(v);
    }
    return result;
  }
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
