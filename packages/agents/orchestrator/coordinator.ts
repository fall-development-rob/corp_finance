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
import { createEntityExtractor } from '../utils/llm-extractor.js';
import { InsightBus } from '../collaboration/insight-bus.js';

export interface OrchestratorConfig {
  confidenceThreshold?: number;
  maxSpecialists?: number;
  callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  callFmpTool?: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  onEvent?: (event: { type: string; payload: unknown }) => void;
}

export class Orchestrator {
  private chief: ChiefAnalyst;
  private eventBus: EventBus;
  private callTool: OrchestratorConfig['callTool'];
  private callFmpTool?: OrchestratorConfig['callFmpTool'];
  private extractEntities: ReturnType<typeof createEntityExtractor>;
  private initialized = false;

  constructor(config: OrchestratorConfig) {
    this.eventBus = new SimpleEventBus();
    this.callTool = config.callTool;
    this.callFmpTool = config.callFmpTool;
    this.extractEntities = createEntityExtractor();

    this.chief = new ChiefAnalyst({
      confidenceThreshold: config.confidenceThreshold ?? 0.6,
      maxSpecialists: config.maxSpecialists ?? 6,
      eventBus: this.eventBus,
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
    try { await initReasoningBank(); } catch { /* best-effort */ }
    this.initialized = true;
  }

  async analyze(query: string, priority: Priority = 'STANDARD', options?: { company?: string }): Promise<{
    request: AnalysisRequest;
    report: string;
    results: AnalysisResult[];
  }> {
    await this.ensureInit();

    // 1. Create request
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
    } catch { /* best-effort */ }

    // 3. Plan
    this.chief.createPlan(request);

    // 4. Create assignments
    const assignments = this.chief.createAssignments(request);

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
          company: options?.company,
          eventBus: this.eventBus,
          callTool: this.callTool,
          callFmpTool: this.callFmpTool,
          extractEntities: this.extractEntities ?? undefined,
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
        } catch { /* best-effort */ }

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
          } catch { /* best-effort */ }

          return result;
        } catch {
          assignment.status = 'failed';
          assignment.completedAt = new Date();
          return null;
        }
      }),
    );

    const validResults = results.filter((r): r is AnalysisResult => r !== null);

    // 6. Aggregate
    const report = this.chief.aggregate(request, validResults);

    // 7. Store final report in ReasoningBank for learning
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
            completedAt: new Date().toISOString(),
          }),
          source: {
            task_id: request.requestId,
            agent_id: 'orchestrator',
            outcome: 'Success',
            evidence: validResults.map(r => r.summary),
          },
          tags: ['cfa-report', 'completed'],
          domain: 'cfa-orchestration',
          created_at: new Date().toISOString(),
          confidence: request.confidence?.value ?? 0.5,
          n_uses: 0,
        },
        confidence: request.confidence?.value ?? 0.5,
        usage_count: 0,
      });
    } catch { /* best-effort */ }

    return { request, report, results: validResults };
  }
}
