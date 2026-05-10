/**
 * Tool router — dispatches ToolCall[] to MCPClient in parallel and
 * returns ToolResult[].
 *
 * CFA plugins are independent processes; parallel execution is safe and
 * reduces latency when the model emits multiple tool_use blocks in one turn.
 */
import type { MCPClient, ToolCall, ToolResult } from "../types.js";

export type OnEventFn = (event: { type: "tool_call" | "tool_result"; payload: unknown }) => void;

async function executeOne(
  call: ToolCall,
  mcp: MCPClient,
): Promise<ToolResult> {
  try {
    const content = await mcp.callTool(call.name, call.input);
    return { call_id: call.id, content, is_error: false };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { call_id: call.id, content: message, is_error: true };
  }
}

/**
 * Routes all tool calls in parallel via `Promise.all`.
 * `onEvent` is called (fire-and-forget) before and after each call.
 */
export async function routeToolCalls(
  calls: ToolCall[],
  mcp: MCPClient,
  onEvent?: OnEventFn,
): Promise<ToolResult[]> {
  return Promise.all(
    calls.map(async (call) => {
      onEvent?.({ type: "tool_call", payload: call });
      const result = await executeOne(call, mcp);
      onEvent?.({ type: "tool_result", payload: result });
      return result;
    }),
  );
}
