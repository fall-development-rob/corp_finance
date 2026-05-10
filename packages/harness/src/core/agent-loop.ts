/**
 * Agent dispatch loop — the core runtime of Phase 31.
 *
 * Drives the model through turns until `end_turn` or `maxTurns` is reached.
 * Real tool calls are resolved via the MCP client and fed back as
 * tool_result blocks. Virtual `delegate_to_<id>` calls (Wave 2) are
 * intercepted and dispatched as nested specialist runs.
 *
 * Wave 4: optional audit + session hooks. If `options.audit` is provided,
 * every dispatch produces a sha256-hashed AuditRecord (with per-tool-call
 * input/result hashes and parent/child wiring). If `options.session` is
 * provided, dispatch state is persisted on completion for replay.
 */
import { createHash, randomUUID } from "node:crypto";
import type {
  AuditRecord,
  AuditToolCall,
  CanonicalTool,
  ContentBlock,
  DispatchEvent,
  DispatchOptions,
  DispatchResult,
  Message,
  SessionState,
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

function sha256(input: string): string {
  return createHash("sha256").update(input).digest("hex");
}

function hashJSON(value: unknown): string {
  return sha256(JSON.stringify(value));
}

function nowIso(): string {
  return new Date().toISOString();
}

export async function dispatch(options: DispatchOptions): Promise<DispatchResult> {
  const { agent, prompt, mcp, onEvent, audit, session, parentAuditId } = options;
  const depth = options.depth ?? 0;
  const maxRecursion = agent.maxRecursionDepth ?? 0;

  // Note: `maxRecursionDepth` bounds DELEGATION, not execution. An agent
  // at depth=1 with maxRecursion=0 still runs (it just can't delegate
  // further). The delegate exposure below uses `depth < maxRecursion` to
  // cut off virtual delegation tools at the appropriate level.

  const provider = options.provider ?? createAnthropicProvider();
  const maxTurns = options.maxTurns ?? DEFAULT_MAX_TURNS;

  // Wave 4 audit/session bookkeeping
  const auditId = audit ? randomUUID() : undefined;
  const sessionId = session ? randomUUID() : undefined;
  const startTime = Date.now();
  const auditToolCalls: AuditToolCall[] = [];
  const childAuditIds: string[] = [];
  const childSessionIds: string[] = [];

  // Initial session save (in_progress) so partial work is recoverable
  let sessionState: SessionState | undefined;
  if (session && sessionId) {
    sessionState = {
      session_id: sessionId,
      agent_id: agent.id,
      prompt,
      messages: [],
      tool_uses: 0,
      child_session_ids: [],
      status: "in_progress",
      audit_id: auditId,
      created_at: nowIso(),
      updated_at: nowIso(),
    };
    await session.save(sessionState);
  }

  // Real MCP tools, filtered by the agent's allowlist.
  const allTools = await mcp.listTools();
  const realTools: CanonicalTool[] = filterToolsForAgent(allTools, agent.tools);

  // Virtual delegation tools — only at depth 0 (or wherever depth < maxRecursion).
  const delegates = depth < maxRecursion ? options.delegates ?? [] : [];
  const delegationTools = delegates.length > 0 ? createDelegationTools(delegates) : [];

  const tools: CanonicalTool[] = [...realTools, ...delegationTools];

  const messages: Message[] = [
    { role: "user", content: [{ type: "text", text: prompt }] },
  ];

  let toolUses = 0;
  let turn = 0;
  const childDispatches: DispatchResult[] = [];
  let totalUsage: { inputTokens: number; outputTokens: number } | undefined;

  try {
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

      if (response.usage) {
        totalUsage = {
          inputTokens: (totalUsage?.inputTokens ?? 0) + response.usage.inputTokens,
          outputTokens: (totalUsage?.outputTokens ?? 0) + response.usage.outputTokens,
        };
      }

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

        const delegationCalls: ToolCall[] = [];
        const realCalls: ToolCall[] = [];
        for (const c of calls) {
          if (isDelegationToolName(c.name)) {
            delegationCalls.push(c);
          } else {
            realCalls.push(c);
          }
        }

        // Real tool call routing — track per-call timing for audit
        const realResultsP = (async (): Promise<ToolResult[]> => {
          if (realCalls.length === 0) return [];
          const startsAt = new Map(realCalls.map((c) => [c.id, Date.now()]));
          const results = await routeToolCalls(realCalls, mcp);
          if (audit) {
            for (const c of realCalls) {
              const r = results.find((x) => x.call_id === c.id);
              const durMs = Date.now() - (startsAt.get(c.id) ?? Date.now());
              auditToolCalls.push({
                id: c.id,
                name: c.name,
                input_hash: hashJSON(c.input),
                result_hash: r ? hashJSON(r.content) : sha256(""),
                is_error: r?.is_error ?? false,
                duration_ms: durMs,
              });
            }
          }
          return results;
        })();

        const delegationsP = Promise.all(
          delegationCalls.map(async (call) => {
            const target = resolveDelegationTarget(call.name, delegates);
            if (!target) {
              const r: ToolResult = {
                call_id: call.id,
                content: `error: unknown delegation target "${call.name}"`,
                is_error: true,
              };
              if (audit) {
                auditToolCalls.push({
                  id: call.id,
                  name: call.name,
                  input_hash: hashJSON(call.input),
                  result_hash: hashJSON(r.content),
                  is_error: true,
                  duration_ms: 0,
                });
              }
              return r;
            }
            emit(onEvent, {
              type: "delegation",
              targetAgent: target.id,
              subPrompt: typeof call.input["sub_prompt"] === "string" ? call.input["sub_prompt"] : "",
            });

            const delStart = Date.now();
            try {
              const childResult = await dispatch({
                agent: target,
                prompt: buildSubPrompt(call.input),
                provider,
                mcp,
                maxTurns: options.maxTurns,
                depth: depth + 1,
                onEvent,
                delegates: undefined,
                audit,
                session,
                parentAuditId: auditId,
              });
              childDispatches.push(childResult);
              if (childResult.auditId) childAuditIds.push(childResult.auditId);
              if (childResult.sessionId) childSessionIds.push(childResult.sessionId);
              const r: ToolResult = {
                call_id: call.id,
                content: formatDelegationResult(target.id, childResult),
                is_error: false,
              };
              if (audit) {
                auditToolCalls.push({
                  id: call.id,
                  name: call.name,
                  input_hash: hashJSON(call.input),
                  result_hash: hashJSON(r.content),
                  is_error: false,
                  duration_ms: Date.now() - delStart,
                });
              }
              return r;
            } catch (err) {
              const r: ToolResult = {
                call_id: call.id,
                content: `delegation to "${target.id}" failed: ${err instanceof Error ? err.message : String(err)}`,
                is_error: true,
              };
              if (audit) {
                auditToolCalls.push({
                  id: call.id,
                  name: call.name,
                  input_hash: hashJSON(call.input),
                  result_hash: hashJSON(r.content),
                  is_error: true,
                  duration_ms: Date.now() - delStart,
                });
              }
              return r;
            }
          }),
        );

        const [realResults, delResults] = await Promise.all([realResultsP, delegationsP]);

        const byId = new Map<string, ToolResult>();
        for (const r of realResults) byId.set(r.call_id, r);
        for (const r of delResults) byId.set(r.call_id, r);
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

    // Wave 4: audit write
    if (audit && auditId) {
      const record: AuditRecord = {
        audit_id: auditId,
        ...(parentAuditId ? { parent_audit_id: parentAuditId } : {}),
        agent_id: agent.id,
        prompt_hash: sha256(prompt),
        tool_calls: auditToolCalls,
        result_hash: sha256(finalText),
        duration_ms: Date.now() - startTime,
        total_tool_uses: toolUses,
        child_audit_ids: childAuditIds,
        timestamp: nowIso(),
        ...(agent.model ? { model: agent.model } : {}),
        ...(totalUsage ? { usage: totalUsage } : {}),
      };
      await audit.write(record);
    }

    // Wave 4: session save (completed)
    if (session && sessionState) {
      sessionState = {
        ...sessionState,
        messages,
        tool_uses: toolUses,
        child_session_ids: childSessionIds,
        final_text: finalText,
        status: "completed",
        updated_at: nowIso(),
      };
      await session.save(sessionState);
    }

    return {
      finalText,
      toolUses,
      messages,
      childDispatches,
      ...(auditId ? { auditId } : {}),
      ...(sessionId ? { sessionId } : {}),
    };
  } catch (err) {
    // Wave 4: persist failed-state session for forensic replay
    if (session && sessionState) {
      try {
        await session.save({
          ...sessionState,
          messages,
          tool_uses: toolUses,
          child_session_ids: childSessionIds,
          status: "failed",
          updated_at: nowIso(),
        });
      } catch {
        // best-effort
      }
    }
    throw err;
  }
}
