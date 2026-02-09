export function wrapResponse(resultJson: string) {
  return {
    content: [{ type: "text" as const, text: resultJson }],
  };
}
