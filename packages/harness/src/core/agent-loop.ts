/**
 * Agent dispatch loop — the core runtime of Phase 31 Wave 1.
 *
 * Drives the model through turns until `end_turn` or `maxTurns` is reached.
 * Tool calls are resolved via the MCP client and fed back as tool_result blocks.
 */
import type {
  ContentBlock,
  DispatchEvent,
  DispatchOptions,
  DispatchResult,
  Message,
  ToolResultBlock,
  ToolUseBlock,
} from "../types.js";
import { createAnthropicProvider } from "./providers/anthropic.js";
import { filterToolsForAgent } from "./tool-schema.js";
import { routeToolCalls } from "./tool-router.js";

const DEFAULT_MAX_TURNS = 25;

function extractToolUseBlocks(content: ContentBlock[]): ToolUseBlock[] {
  return content.filter((b): b is ToolUseBlock => b.type === "tool_use");
}

function extractFinalText(content: ContentBlock[]): string {
  return content
    .filter((b): b is { type: "text"; text: string } => b.type === "text")
    .map((b) => b.text)
    .join("");
}

function emit(onEvent: ((e: DispatchEvent) => void) | undefined, event: DispatchEvent): void {
  onEvent?.(event);
}

/**
 * Dispatches an agent through the Anthropic Messages API in a loop.
 * Returns when the model signals `end_turn` or `maxTurns` is exhausted.
 */
export async function dispatch(options: DispatchOptions): Promise<DispatchResult> {
  const { agent, prompt, mcp, onEvent } = options;
  const depth = options.depth ?? 0;
  const maxRecursion = agent.maxRecursionDepth ?? 0;

  if (depth > maxRecursion) {
    throw new Error(
      `Recursion depth ${depth} exceeds agent "${agent.id}" maxRecursionDepth ${maxRecursion}`,
    );
  }

  const provider = options.provider ?? createAnthropicProvider();
  const maxTurns = options.maxTurns ?? DEFAULT_MAX_TURNS;

  const allTools = await mcp.listTools();
  const tools = filterToolsForAgent(allTools, agent.tools);

  const messages: Message[] = [
    { role: "user", content: [{ type: "text", text: prompt }] },
  ];

  let toolUses = 0;
  let turn = 0;

  while (turn < maxTurns) {
    emit(onEvent, { type: "turn_started", turn, agentId: agent.id });

    const response = await provider.turn({
      systemPrompt: agent.systemPrompt,
      messages,
      tools,
      model: agent.model,
      maxTokens: agent.maxTokens,
    });

    messages.push(response.message);

    emit(onEvent, { type: "turn_completed", turn, stopReason: response.stopReason });

    if (response.stopReason === "end_turn" || response.stopReason === "max_tokens" || response.stopReason === "stop_sequence") {
      break;
    }

    if (response.stopReason === "tool_use") {
      const toolUseBlocks = extractToolUseBlocks(response.message.content);
      const calls = toolUseBlocks.map((b) => ({
        id: b.id,
        name: b.name,
        input: b.input,
      }));

      for (const call of calls) {
        emit(onEvent, { type: "tool_call", call, turn });
      }

      const results = await routeToolCalls(calls, mcp);
      toolUses += results.length;

      for (const result of results) {
        emit(onEvent, { type: "tool_result", result, turn });
      }

      const toolResultBlocks: ToolResultBlock[] = results.map((r) => ({
        type: "tool_result" as const,
        tool_use_id: r.call_id,
        content: typeof r.content === "string" ? r.content : JSON.stringify(r.content),
        is_error: r.is_error,
      }));

      messages.push({ role: "user", content: toolResultBlocks });
    }

    turn++;
  }

  const lastAssistant = [...messages].reverse().find((m) => m.role === "assistant");
  const finalText = lastAssistant ? extractFinalText(lastAssistant.content) : "";

  emit(onEvent, { type: "dispatch_completed", toolUses });

  return { finalText, toolUses, messages, childDispatches: [] };
}
