/**
 * Agent dispatch loop — the core runtime of Phase 31.
 *
 * Drives the model through turns until `end_turn` or `maxTurns` is reached.
 * Real tool calls are resolved via the MCP client and fed back as
 * tool_result blocks. Virtual `delegate_to_<id>` calls (Wave 2) are
 * intercepted and dispatched as nested specialist runs.
 */
import type {
  CanonicalTool,
  ContentBlock,
  DispatchEvent,
  DispatchOptions,
  DispatchResult,
  Message,
  ToolCall,
  ToolResult,
  ToolResultBlock,
  ToolUseBlock,
} from "../types.js";
import { createAnthropicProvider } from "./providers/anthropic.js";
import { filterToolsForAgent } from "./tool-schema.js";
import { routeToolCalls } from "./tool-router.js";
import {
  buildSubPrompt,
  createDelegationTools,
  formatDelegationResult,
  isDelegationToolName,
  resolveDelegationTarget,
} from "../agents/delegate.js";

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
 *
 * Wave 2 additions:
 * - If `options.delegates` is non-empty, virtual `delegate_to_<id>` tools
 *   are injected. When invoked, the loop runs a nested `dispatch()` on the
 *   specialist with `depth + 1`.
 * - Specialists at `depth >= 1` receive `delegates: undefined` so they
 *   cannot delegate further (Phase 31 cap).
 */
export async function dispatch(options: DispatchOptions): Promise<DispatchResult> {
  const { agent, prompt, mcp, onEvent } = options;
  const depth = options.depth ?? 0;
  const maxRecursion = agent.maxRecursionDepth ?? 0;

  // Note: `maxRecursionDepth` bounds DELEGATION, not execution. An agent
  // at depth=1 with maxRecursion=0 still runs (it just can't delegate
  // further). The delegate exposure below uses `depth < maxRecursion` to
  // cut off virtual delegation tools at the appropriate level.

  const provider = options.provider ?? createAnthropicProvider();
  const maxTurns = options.maxTurns ?? DEFAULT_MAX_TURNS;

  // Real MCP tools, filtered by the agent's allowlist.
  const allTools = await mcp.listTools();
  const realTools: CanonicalTool[] = filterToolsForAgent(allTools, agent.tools);

  // Virtual delegation tools — only at depth 0 (or wherever depth < maxRecursion).
  // Phase 31 caps at 1: chief at depth 0 sees delegates; specialists at depth 1 do not.
  const delegates = depth < maxRecursion ? options.delegates ?? [] : [];
  const delegationTools = delegates.length > 0 ? createDelegationTools(delegates) : [];

  const tools: CanonicalTool[] = [...realTools, ...delegationTools];

  const messages: Message[] = [
    { role: "user", content: [{ type: "text", text: prompt }] },
  ];

  let toolUses = 0;
  let turn = 0;
  const childDispatches: DispatchResult[] = [];

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

    if (
      response.stopReason === "end_turn" ||
      response.stopReason === "max_tokens" ||
      response.stopReason === "stop_sequence"
    ) {
      break;
    }

    if (response.stopReason === "tool_use") {
      const toolUseBlocks = extractToolUseBlocks(response.message.content);
      const calls: ToolCall[] = toolUseBlocks.map((b) => ({
        id: b.id,
        name: b.name,
        input: b.input,
      }));

      for (const call of calls) {
        emit(onEvent, { type: "tool_call", call, turn });
      }

      // Split calls into delegations vs real tool calls.
      const delegationCalls: ToolCall[] = [];
      const realCalls: ToolCall[] = [];
      for (const c of calls) {
        if (isDelegationToolName(c.name)) {
          delegationCalls.push(c);
        } else {
          realCalls.push(c);
        }
      }

      // Route real tool calls via MCP, delegations via nested dispatch.
      // Both run in parallel within the turn; results merged back in original
      // call order so the model sees consistent tool_result blocks.
      const realResultsP = realCalls.length > 0 ? routeToolCalls(realCalls, mcp) : Promise.resolve([] as ToolResult[]);
      const delegationResults: ToolResult[] = [];
      const delegationsP = Promise.all(
        delegationCalls.map(async (call) => {
          const target = resolveDelegationTarget(call.name, delegates);
          if (!target) {
            const r: ToolResult = {
              call_id: call.id,
              content: `error: unknown delegation target "${call.name}"`,
              is_error: true,
            };
            return r;
          }
          emit(onEvent, {
            type: "delegation",
            targetAgent: target.id,
            subPrompt: typeof call.input["sub_prompt"] === "string" ? call.input["sub_prompt"] : "",
          });

          try {
            const childResult = await dispatch({
              agent: target,
              prompt: buildSubPrompt(call.input),
              provider,
              mcp,
              maxTurns: options.maxTurns,
              depth: depth + 1,
              onEvent,
              // Specialists do not delegate further (Phase 31 cap).
              delegates: undefined,
            });
            childDispatches.push(childResult);
            const r: ToolResult = {
              call_id: call.id,
              content: formatDelegationResult(target.id, childResult),
              is_error: false,
            };
            return r;
          } catch (err) {
            const r: ToolResult = {
              call_id: call.id,
              content: `delegation to "${target.id}" failed: ${err instanceof Error ? err.message : String(err)}`,
              is_error: true,
            };
            return r;
          }
        }),
      );

      const [realResults, delResults] = await Promise.all([realResultsP, delegationsP]);
      delegationResults.push(...delResults);

      // Re-merge results in original call order so message indices remain stable.
      const byId = new Map<string, ToolResult>();
      for (const r of realResults) byId.set(r.call_id, r);
      for (const r of delegationResults) byId.set(r.call_id, r);
      const ordered: ToolResult[] = calls.map((c) => {
        const r = byId.get(c.id);
        if (!r) {
          return {
            call_id: c.id,
            content: "error: tool result missing",
            is_error: true,
          };
        }
        return r;
      });

      toolUses += ordered.length;

      for (const result of ordered) {
        emit(onEvent, { type: "tool_result", result, turn });
      }

      const toolResultBlocks: ToolResultBlock[] = ordered.map((r) => ({
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

  return { finalText, toolUses, messages, childDispatches };
}
