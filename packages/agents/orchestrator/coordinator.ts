// Orchestrator — coordinates CFA agents via agentic-flow
// Uses agentic-flow/reasoningbank for memory persistence and retrieval

import { randomUUID } from 'node:crypto';
import {
  initialize as initReasoningBank,
  retrieveMemories,
  db,
} from 'agentic-flow/reasoningbank';
import type { AnalysisRequest, Priority } from '../types/analysis.js';
import type { AnalysisResult } from '../types/agents.js';
import type { EventBus } from '../types/events.js';
import { SimpleEventBus } from '../types/events.js';
import { ChiefAnalyst } from '../agents/chief-analyst.js';
import { createSpecialist } from './specialist-factory.js';
import type { AnalystContext } from '../agents/base-analyst.js';
import { parseFinancialData } from '../utils/financial-parser.js';
import { resolveCompany } from '../utils/company-resolver.js';
import { InsightBus } from '../collaboration/insight-bus.js';
import { ExpertRouter } from '../config/expert-router.js';

/** Simple structured logger for orchestrator diagnostics */
function log(level: 'info' | 'warn' | 'error', message: string, data?: Record<string, unknown>): void {
  const prefix = `[Orchestrator:${level.toUpperCase()}]`;
  if (data) {
    console.error(`${prefix} ${message}`, JSON.stringify(data));
  } else {
    console.error(`${prefix} ${message}`);
  }
}

export interface OrchestratorConfig {
  confidenceThreshold?: number;
  maxSpecialists?: number;
  callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  callFmpTool?: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  onEvent?: (event: { type: string; payload: unknown }) => void;
  /** Disable semantic MoE routing (forces static keyword fallback). Default: false */
  disableSemanticRouting?: boolean;
}

export class Orchestrator {
  private chief: ChiefAnalyst;
  private eventBus: EventBus;
  private callTool: OrchestratorConfig['callTool'];
  private callFmpTool?: OrchestratorConfig['callFmpTool'];
  private initialized = false;

  constructor(config: OrchestratorConfig) {
    this.eventBus = new SimpleEventBus();
    this.callTool = config.callTool;
    this.callFmpTool = config.callFmpTool;

    const expertRouter = config.disableSemanticRouting
      ? undefined
      : new ExpertRouter({ maxResults: config.maxSpecialists ?? 6 });
    this.chief = new ChiefAnalyst({
      confidenceThreshold: config.confidenceThreshold ?? 0.6,
      maxSpecialists: config.maxSpecialists ?? 6,
      eventBus: this.eventBus,
      expertRouter,
    });

    if (config.onEvent) {
      const handler = config.onEvent;
      const eventTypes = [
        'AnalysisRequested', 'PlanCreated', 'AnalystAssigned',
        'ToolCalled', 'ToolSucceeded', 'ToolFailed',
        'AnalysisCompleted', 'ResultAggregated', 'AnalysisEscalated',
      ] as const;
      for (const type of eventTypes) {
        this.eventBus.on(type, (e) => handler({ type: e.type, payload: e.payload }));
      }
    }
  }

  private async ensureInit(): Promise<void> {
    if (this.initialized) return;
    try {
      await initReasoningBank();
    } catch (err) {
      log('warn', 'ReasoningBank initialization failed — continuing without memory', {
        error: err instanceof Error ? err.message : String(err),
      });
    }
    this.initialized = true;
  }

  async analyze(query: string, priority: Priority = 'STANDARD', options?: { company?: string }): Promise<{
    request: AnalysisRequest;
    report: string;
    results: AnalysisResult[];
  }> {
    await this.ensureInit();

    // 1. Route query (semantic MoE first, static keyword fallback)
    const { intent, agents: routedAgents } = await this.chief.routeQuery(query);

    // 2. Create request
    const request = this.chief.createRequest(query, priority);

    // 2. Store request in ReasoningBank for swarm visibility
    try {
      db.upsertMemory({
        id: request.requestId,
        type: 'reasoning_memory',
        pattern_data: {
          title: 'analysis-request',
          description: query,
          content: JSON.stringify({ query, priority, status: 'planning' }),
          source: {
            task_id: request.requestId,
            agent_id: 'orchestrator',
            outcome: 'Success',
            evidence: [],
          },
          tags: ['cfa-request'],
          domain: 'cfa-orchestration',
          created_at: new Date().toISOString(),
          confidence: 1.0,
          n_uses: 0,
        },
        confidence: 1.0,
        usage_count: 0,
      });
    } catch (err) {
      log('warn', 'Failed to store analysis request in ReasoningBank', {
        requestId: request.requestId,
        error: err instanceof Error ? err.message : String(err),
      });
    }

    // 3. Resolve company name once for all specialists
    //    Priority: explicit > regex > semantic (local embeddings, no API call)
    let company = options?.company;
    if (!company) {
      const parsed = parseFinancialData(query);
      company = parsed._company ?? undefined;
    }
    // If regex found a name, validate/enrich it via semantic matching
    // If regex missed, try semantic search on the full query
    if (company) {
      try {
        const match = await resolveCompany(company);
        if (match) company = match.name; // normalise to canonical name
      } catch (err) {
        log('warn', 'Company resolution failed', {
          company,
          error: err instanceof Error ? err.message : String(err),
        });
      }
    } else {
      try {
        const match = await resolveCompany(query);
        if (match) company = match.name;
      } catch (err) {
        log('warn', 'Company resolution failed', {
          company: query,
          error: err instanceof Error ? err.message : String(err),
        });
      }
    }

    // 4. Plan (using routed agents if available)
    this.chief.createPlan(request, routedAgents);

    // 5. Create assignments
    const assignments = this.chief.createAssignments(request);

    // Propagate routing scores to assignments for observability
    const scoreMap = new Map(routedAgents.map(a => [a.agentType, a.score]));
    for (const assignment of assignments) {
      assignment.routingScore = scoreMap.get(assignment.agentType) ?? 0;
    }

    // ADR-006: Create shared InsightBus for cross-specialist collaboration
    const insightBus = new InsightBus();

    // 5. Execute specialist agents in parallel
    const results = await Promise.all(
      assignments.map(async (assignment) => {
        const specialist = createSpecialist(assignment.agentType as any);
        if (!specialist) {
          assignment.status = 'skipped';
          return null;
        }

        assignment.status = 'in_progress';
        assignment.startedAt = new Date();

        const ctx: AnalystContext = {
          assignmentId: assignment.assignmentId,
          requestId: request.requestId,
          task: request.plan!.steps.find(s => s.id === assignment.stepRef)?.description ?? query,
          company,
          eventBus: this.eventBus,
          callTool: this.callTool,
          callFmpTool: this.callFmpTool,
          insightBus,
        };

        // Search ReasoningBank for relevant prior analyses
        try {
          const priorMemories = await retrieveMemories(ctx.task, {
            k: 3,
            domain: 'cfa-analysis',
          });
          if (priorMemories.length > 0) {
            ctx.priorContext = priorMemories.map(m => m.content).join('\n---\n');
          }
        } catch (err) {
          log('info', 'ReasoningBank memory retrieval failed', {
            task: ctx.task.slice(0, 100),
            error: err instanceof Error ? err.message : String(err),
          });
        }

        try {
          const result = await specialist.execute(ctx);
          assignment.status = 'completed';
          assignment.completedAt = new Date();
          assignment.resultRef = result.resultId;

          // Store result in ReasoningBank for cross-agent learning
          try {
            db.upsertMemory({
              id: result.resultId,
              type: 'reasoning_memory',
              pattern_data: {
                title: `${assignment.agentType}-result`,
                description: result.summary,
                content: JSON.stringify({
                  resultId: result.resultId,
                  agentType: result.agentType,
                  confidence: result.confidence,
                  summary: result.summary,
                  findingCount: result.findings.length,
                }),
                source: {
                  task_id: request.requestId,
                  agent_id: assignment.agentType,
                  outcome: 'Success',
                  evidence: result.findings.map(f => f.statement),
                },
                tags: ['cfa-result', assignment.agentType],
                domain: 'cfa-analysis',
                created_at: new Date().toISOString(),
                confidence: result.confidence,
                n_uses: 0,
              },
              confidence: result.confidence,
              usage_count: 0,
            });
          } catch (err) {
            log('warn', 'Failed to store specialist result in ReasoningBank', {
              agentType: assignment.agentType,
              error: err instanceof Error ? err.message : String(err),
            });
          }

          return result;
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          assignment.status = 'failed';
          assignment.completedAt = new Date();
          log('error', `Specialist ${assignment.agentType} failed`, {
            assignmentId: assignment.assignmentId,
            error: errorMsg,
          });
          return null;
        }
      }),
    );

    const validResults = results.filter((r): r is AnalysisResult => r !== null);

    // 6. Aggregate
    const report = this.chief.aggregate(request, validResults);

    // 7. Store final report in ReasoningBank for learning
    const failedCount = assignments.filter(a => a.status === 'failed').length;
    const outcome: 'Success' | 'Failure' = failedCount === 0 ? 'Success' : 'Failure';
    try {
      db.upsertMemory({
        id: `report-${request.requestId}`,
        type: 'reasoning_memory',
        pattern_data: {
          title: 'cfa-report',
          description: query,
          content: JSON.stringify({
            requestId: request.requestId,
            query,
            confidence: request.confidence?.value,
            specialistsUsed: validResults.map(r => r.agentType),
            failedSpecialists: assignments.filter(a => a.status === 'failed').map(a => a.agentType),
            completedAt: new Date().toISOString(),
          }),
          source: {
            task_id: request.requestId,
            agent_id: 'orchestrator',
            outcome,
            evidence: validResults.map(r => r.summary),
          },
          tags: ['cfa-report', outcome === 'Success' ? 'completed' : validResults.length > 0 ? 'partial' : 'failed'],
          domain: 'cfa-orchestration',
          created_at: new Date().toISOString(),
          confidence: request.confidence?.value ?? 0.5,
          n_uses: 0,
        },
        confidence: request.confidence?.value ?? 0.5,
        usage_count: 0,
      });
    } catch (err) {
      log('warn', 'Failed to store final report in ReasoningBank', {
        requestId: request.requestId,
        error: err instanceof Error ? err.message : String(err),
      });
    }

    return { request, report, results: validResults };
  }
}
