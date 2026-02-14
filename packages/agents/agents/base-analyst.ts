// Base analyst agent with Dexter-style iterative reasoning loop
// All specialist agents extend this class

import { randomUUID } from 'node:crypto';
import type {
  AgentType, AgentCapability, ToolInvocation, Finding,
  Citation, AnalysisResult,
} from '../types/agents.js';
import type { DomainEvent, EventBus } from '../types/events.js';
import { TOOL_MAPPINGS, AGENT_DESCRIPTIONS } from '../config/tool-mappings.js';
import { parseFinancialData, type ExtractedMetrics } from '../utils/financial-parser.js';
import { enrichMetrics } from '../utils/fmp-data-fetcher.js';

export interface AnalystContext {
  assignmentId: string;
  requestId: string;
  task: string;
  priorContext?: string;       // relevant memory from prior analyses
  eventBus: EventBus;
  callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  callFmpTool?: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
}

export interface ReasoningState {
  observations: string[];
  thoughts: string[];
  toolResults: ToolInvocation[];
  reflections: string[];
  iteration: number;
  maxIterations: number;
  shouldContinue: boolean;
  /** Financial metrics extracted from the task text */
  metrics: ExtractedMetrics;
}

export abstract class BaseAnalyst {
  readonly agentId: string;
  readonly agentType: AgentType;
  readonly capability: AgentCapability;

  constructor(agentType: AgentType) {
    this.agentId = randomUUID();
    this.agentType = agentType;
    this.capability = {
      agentType,
      toolDomains: TOOL_MAPPINGS[agentType] ?? [],
      analysisTypes: [],
      description: AGENT_DESCRIPTIONS[agentType] ?? '',
    };
  }

  // The Dexter-style iterative reasoning loop
  async execute(ctx: AnalystContext): Promise<AnalysisResult> {
    const textMetrics = parseFinancialData(ctx.task);

    const state: ReasoningState = {
      observations: [],
      thoughts: [],
      toolResults: [],
      reflections: [],
      iteration: 0,
      maxIterations: 5,
      shouldContinue: true,
      metrics: textMetrics,
    };

    // Enrich with live FMP data if available
    if (ctx.callFmpTool && textMetrics._company) {
      try {
        state.metrics = await enrichMetrics(textMetrics, ctx.callFmpTool);
      } catch {
        // Graceful degradation — continue with text-only metrics
      }
    }

    // Phase 1: Observe
    state.observations.push(
      `Assignment: ${ctx.task}`,
      ...(ctx.priorContext ? [`Prior context: ${ctx.priorContext}`] : []),
    );

    while (state.shouldContinue && state.iteration < state.maxIterations) {
      state.iteration++;

      // Phase 2: Think — plan which tools to call
      const toolPlan = await this.think(ctx, state);

      // Phase 3: Act — execute tool calls
      for (const { toolName, params } of toolPlan) {
        if (!this.canUseTool(toolName)) continue;

        const invocation: ToolInvocation = {
          invocationId: randomUUID(),
          agentId: this.agentId,
          toolName,
          params,
          timestamp: new Date(),
        };

        ctx.eventBus.emit({
          eventId: randomUUID(),
          type: 'ToolCalled',
          timestamp: new Date(),
          sourceContext: 'SpecialistAnalysts',
          payload: { agentId: this.agentId, toolName, params, invocationId: invocation.invocationId },
        });

        try {
          const start = Date.now();
          invocation.result = await ctx.callTool(toolName, params);
          invocation.duration = Date.now() - start;

          ctx.eventBus.emit({
            eventId: randomUUID(),
            type: 'ToolSucceeded',
            timestamp: new Date(),
            sourceContext: 'SpecialistAnalysts',
            payload: { invocationId: invocation.invocationId, duration: invocation.duration },
          });
        } catch (err) {
          invocation.error = err instanceof Error ? err.message : String(err);
          ctx.eventBus.emit({
            eventId: randomUUID(),
            type: 'ToolFailed',
            timestamp: new Date(),
            sourceContext: 'SpecialistAnalysts',
            payload: { invocationId: invocation.invocationId, errorType: invocation.error },
          });
        }

        state.toolResults.push(invocation);
      }

      // Phase 4: Reflect — evaluate results, decide to iterate or finalize
      const reflection = await this.reflect(ctx, state);
      state.reflections.push(reflection.summary);
      state.shouldContinue = reflection.shouldIterate;
    }

    // Phase 5: Report — produce structured findings
    const findings = await this.synthesize(ctx, state);
    const confidence = this.calculateConfidence(state);

    const result: AnalysisResult = {
      resultId: randomUUID(),
      agentId: this.agentId,
      agentType: this.agentType,
      assignmentId: ctx.assignmentId,
      findings,
      summary: findings.map(f => f.statement).join('\n'),
      confidence,
      toolInvocations: state.toolResults,
      completedAt: new Date(),
    };

    ctx.eventBus.emit({
      eventId: randomUUID(),
      type: 'AnalysisCompleted',
      timestamp: new Date(),
      sourceContext: 'SpecialistAnalysts',
      payload: { agentId: this.agentId, assignmentId: ctx.assignmentId, resultId: result.resultId, confidence },
    });

    return result;
  }

  // Subclasses implement domain-specific reasoning
  protected abstract think(
    ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<Array<{ toolName: string; params: Record<string, unknown> }>>;

  protected abstract reflect(
    ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<{ summary: string; shouldIterate: boolean }>;

  protected abstract synthesize(
    ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<Finding[]>;

  protected canUseTool(toolName: string): boolean {
    // Check if the tool's module is in this agent's allowed domains
    return this.capability.toolDomains.some(domain =>
      toolName.startsWith(domain) || toolName.includes(domain),
    );
  }

  private calculateConfidence(state: ReasoningState): number {
    const successfulTools = state.toolResults.filter(t => !t.error).length;
    const totalTools = state.toolResults.length;
    if (totalTools === 0) return 0;
    return Math.min(1, successfulTools / totalTools * (state.iteration > 1 ? 1.1 : 1.0));
  }
}
