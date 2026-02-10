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
