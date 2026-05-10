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
import { assertAllowlistsValid, filterToolsForAgent } from "./tool-schema.js";
import { routeToolCalls } from "./tool-router.js";
import {
  buildSubPrompt,
  createDelegationTools,
  formatDelegationResult,
  isDelegationToolName,
  resolveDelegationTarget,
} from "../agents/delegate.js";
import { indexAuditRecord } from "../reasoning/indexer.js";
import {
  createRecallTool,
  executeRecallCall,
  isRecallToolName,
} from "../reasoning/recall-tool.js";
import {
  createRecallGraphTool,
  executeRecallGraphCall,
  isRecallGraphToolName,
} from "../reasoning/recall-graph-tool.js";
import {
  createWorkflowTools,
  executeListWorkflowsCall,
  executeRunWorkflowCall,
  isWorkflowToolName,
} from "../workflow/tools.js";
import type { WorkflowMatch } from "../workflow/types.js";

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

  // Phase 32 Wave 1: ToolCatalogValidator ACL — convert silent
  // tool_uses=0 failures into loud startup errors. Validates this agent
  // and any delegates' allowlists against the live catalog.
  const catalog = new Set(allTools.map((t) => t.name));
  const agentDefsToValidate = [agent, ...(options.delegates ?? [])];
  assertAllowlistsValid(agentDefsToValidate, catalog);

  const realTools: CanonicalTool[] = filterToolsForAgent(allTools, agent.tools);

  // Phase 35: Hybrid dispatch — deterministic workflow path.
  // Inserted after assertAllowlistsValid so ACL validation always runs.
  // If the workflow path fires, we overwrite the session to "completed"
  // in a single write and return early, bypassing the LLM entirely.
  if (
    options.workflow &&
    (options.workflowMode ?? "auto") !== "disabled"
  ) {
    const threshold = options.workflowAutoRouteThreshold ?? 0.82;
    const mode = options.workflowMode ?? "auto";

    let matchResult: WorkflowMatch | null = null;
    try {
      matchResult = await options.workflow.match(prompt);
    } catch (err) {
      emit(onEvent, {
        type: "workflow_failed",
        slug: "(match)",
        reason: err instanceof Error ? err.message : String(err),
      });
      // fall through to LLM path
    }

    if (matchResult && matchResult.confidence >= threshold && mode === "auto") {
      emit(onEvent, {
        type: "workflow_matched",
        slug: matchResult.slug,
        confidence: matchResult.confidence,
        extracted_params: matchResult.extracted_params,
      });
      const wfStart = Date.now();
      try {
        const wfResult = await options.workflow.run(
          matchResult.slug,
          matchResult.extracted_params,
        );
        emit(onEvent, {
          type: "workflow_executed",
          slug: matchResult.slug,
          duration_ms: wfResult.duration_ms,
          audit_hash: wfResult.audit_hash,
        });

        // Audit record for workflow path
        let auditRecord: AuditRecord | undefined;
        if (audit && auditId) {
          auditRecord = {
            audit_id: auditId,
            ...(parentAuditId ? { parent_audit_id: parentAuditId } : {}),
            agent_id: agent.id,
            prompt_hash: sha256(prompt),
            tool_calls: wfResult.tool_calls.map((tc) => ({
              id: `wf-${tc.name}`,
              name: tc.name,
              input_hash: tc.input_hash,
              result_hash: tc.result_hash,
              is_error: false,
              duration_ms: tc.duration_ms,
            })),
            result_hash: sha256(wfResult.deliverable),
            duration_ms: Date.now() - wfStart,
            total_tool_uses: wfResult.tool_calls.length,
            child_audit_ids: [],
            timestamp: nowIso(),
            path: "workflow",
            workflow_slug: matchResult.slug,
            workflow_audit_hash: wfResult.audit_hash,
          };
          await audit.write(auditRecord);
        }

        // Session save: overwrite in_progress with completed (single durable write)
        if (session && sessionId) {
          await session.save({
            session_id: sessionId,
            agent_id: agent.id,
            prompt,
            messages: [],
            tool_uses: wfResult.tool_calls.length,
            child_session_ids: [],
            final_text: wfResult.deliverable,
            status: "completed",
            audit_id: auditId,
            created_at: nowIso(),
            updated_at: nowIso(),
          });
        }

        return {
          finalText: wfResult.deliverable,
          toolUses: wfResult.tool_calls.length,
          messages: [],
          childDispatches: [],
          ...(auditId ? { auditId } : {}),
          ...(sessionId ? { sessionId } : {}),
          notes: `executed via workflow path: ${matchResult.slug}`,
        };
      } catch (err) {
        emit(onEvent, {
          type: "workflow_failed",
          slug: matchResult.slug,
          reason: err instanceof Error ? err.message : String(err),
        });
        // fall through to LLM path — no re-throw
      }
    }
  }
  // LLM path continues below (existing code unchanged)

  // Virtual delegation tools — only at depth 0 (or wherever depth < maxRecursion).
  const delegates = depth < maxRecursion ? options.delegates ?? [] : [];
  const delegationTools = delegates.length > 0 ? createDelegationTools(delegates) : [];

  // Phase 34 Wave 3-4: virtual `recall_similar` + `recall_by_graph` tools —
  // only when a reasoning bank is configured. Appended after
  // `filterToolsForAgent`, so they bypass the per-agent allowlist;
  // specialists at depth 1 don't receive `reasoning` in their nested
  // dispatch options, so they never see the tools.
  const recallTools: CanonicalTool[] = options.reasoning
    ? [createRecallTool(), createRecallGraphTool()]
    : [];

  // Phase 35 Wave 3: virtual `list_workflows` + `run_workflow` tools —
  // injected at depth=0 only when a WorkflowRouter is configured and mode
  // is not "disabled". Specialists at depth=1 don't receive `workflow` in
  // their nested dispatch options, so they never see these tools.
  const workflowTools: CanonicalTool[] =
    options.workflow && (options.workflowMode ?? "auto") !== "disabled" && depth === 0
      ? createWorkflowTools()
      : [];

  const tools: CanonicalTool[] = [...realTools, ...delegationTools, ...recallTools, ...workflowTools];

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
        const recallCalls: ToolCall[] = [];
        const graphCalls: ToolCall[] = [];
        const workflowCalls: ToolCall[] = [];
        const realCalls: ToolCall[] = [];
        for (const c of calls) {
          if (isDelegationToolName(c.name)) {
            delegationCalls.push(c);
          } else if (options.reasoning && isRecallToolName(c.name)) {
            recallCalls.push(c);
          } else if (options.reasoning && isRecallGraphToolName(c.name)) {
            graphCalls.push(c);
          } else if (options.workflow && isWorkflowToolName(c.name)) {
            workflowCalls.push(c);
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

        const recallsP = Promise.all(
          recallCalls.map(async (call): Promise<ToolResult> => {
            // Defensive: this branch is only entered when options.reasoning is
            // truthy (see split above), but the closure capture isn't narrowed.
            if (!options.reasoning) {
              return {
                call_id: call.id,
                content: "recall_similar failed: no reasoning bank",
                is_error: true,
              };
            }
            const startsAt = Date.now();
            const result = await executeRecallCall(call, options.reasoning);
            if (audit) {
              auditToolCalls.push({
                id: call.id,
                name: call.name,
                input_hash: hashJSON(call.input),
                result_hash: hashJSON(result.content),
                is_error: result.is_error ?? false,
                duration_ms: Date.now() - startsAt,
              });
            }
            return result;
          }),
        );

        // Phase 34 Wave 4: parallel `recall_by_graph` calls. Same audit /
        // error-capture pattern as recallsP; structured-metadata recall.
        const graphsP = Promise.all(
          graphCalls.map(async (call): Promise<ToolResult> => {
            if (!options.reasoning) {
              return {
                call_id: call.id,
                content: "recall_by_graph failed: no reasoning bank",
                is_error: true,
              };
            }
            const startsAt = Date.now();
            const result = await executeRecallGraphCall(
              call,
              options.reasoning,
            );
            if (audit) {
              auditToolCalls.push({
                id: call.id,
                name: call.name,
                input_hash: hashJSON(call.input),
                result_hash: hashJSON(result.content),
                is_error: result.is_error ?? false,
                duration_ms: Date.now() - startsAt,
              });
            }
            return result;
          }),
        );

        // Phase 35 Wave 3: parallel workflow virtual tool calls. Same audit /
        // error-capture pattern as recallsP; errors return is_error results.
        const workflowsP = Promise.all(
          workflowCalls.map(async (call): Promise<ToolResult> => {
            if (!options.workflow) {
              return {
                call_id: call.id,
                content: `${call.name} failed: no workflow router configured`,
                is_error: true,
              };
            }
            const startsAt = Date.now();
            const result = call.name === "list_workflows"
              ? await executeListWorkflowsCall(call, options.workflow)
              : await executeRunWorkflowCall(call, options.workflow);
            if (audit) {
              auditToolCalls.push({
                id: call.id,
                name: call.name,
                input_hash: hashJSON(call.input),
                result_hash: hashJSON(result.content),
                is_error: result.is_error ?? false,
                duration_ms: Date.now() - startsAt,
              });
            }
            return result;
          }),
        );

        const [realResults, delResults, recallResults, graphResults, workflowResults] =
          await Promise.all([realResultsP, delegationsP, recallsP, graphsP, workflowsP]);

        const byId = new Map<string, ToolResult>();
        for (const r of realResults) byId.set(r.call_id, r);
        for (const r of delResults) byId.set(r.call_id, r);
        for (const r of recallResults) byId.set(r.call_id, r);
        for (const r of graphResults) byId.set(r.call_id, r);
        for (const r of workflowResults) byId.set(r.call_id, r);
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
    let auditRecord: AuditRecord | undefined;
    if (audit && auditId) {
      auditRecord = {
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
      await audit.write(auditRecord);
    }

    // Phase 34 Wave 2: fire-and-forget reasoning index. The semantic recall
    // surface is best-effort — index failures emit a DispatchEvent but never
    // fail the dispatch. The audit record is the durable source of truth;
    // the bank is a derived, regenerable cache.
    if (options.reasoning && auditRecord && auditId) {
      void indexAuditRecord({
        bank: options.reasoning,
        record: auditRecord,
        prompt,
        finalText,
      }).catch((err) => {
        emit(onEvent, {
          type: "reasoning_index_failed",
          reason: err instanceof Error ? err.message : String(err),
          audit_id: auditId,
        });
      });
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
