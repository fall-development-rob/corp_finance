// Orchestrator — coordinates CFA agents via agentic-flow
// Uses claude-flow CLI for swarm coordination and agentdb for memory

import { randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import type { AnalysisRequest, Priority } from '../types/analysis.js';
import type { AnalysisResult } from '../types/agents.js';
import type { EventBus } from '../types/events.js';
import { SimpleEventBus } from '../types/events.js';
import { ChiefAnalyst } from '../agents/chief-analyst.js';
import { createSpecialist } from './specialist-factory.js';
import type { BaseAnalyst, AnalystContext } from '../agents/base-analyst.js';

const execFileAsync = promisify(execFile);

export interface OrchestratorConfig {
  confidenceThreshold?: number;
  maxSpecialists?: number;
  callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  onEvent?: (event: { type: string; payload: unknown }) => void;
}

// Wrapper for claude-flow memory operations
async function cfMemory(action: 'store' | 'retrieve' | 'search', opts: Record<string, string>): Promise<unknown> {
  const args = ['@claude-flow/cli@latest', 'memory', action];
  for (const [k, v] of Object.entries(opts)) {
    args.push(`--${k}`, v);
  }
  try {
    const { stdout } = await execFileAsync('npx', args, { timeout: 10000 });
    try { return JSON.parse(stdout); } catch { return stdout.trim(); }
  } catch {
    return null; // Memory operations are best-effort
  }
}

export class Orchestrator {
  private chief: ChiefAnalyst;
  private eventBus: EventBus;
  private callTool: OrchestratorConfig['callTool'];
  private config: OrchestratorConfig;

  constructor(config: OrchestratorConfig) {
    this.config = config;
    this.eventBus = new SimpleEventBus();
    this.callTool = config.callTool;

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

  async analyze(query: string, priority: Priority = 'STANDARD'): Promise<{
    request: AnalysisRequest;
    report: string;
    results: AnalysisResult[];
  }> {
    // 1. Create request
    const request = this.chief.createRequest(query, priority);

    // 2. Store request in agentdb for swarm visibility
    await cfMemory('store', {
      key: `cfa/request/${request.requestId}`,
      value: JSON.stringify({ query, priority, status: 'planning' }),
      namespace: 'analysis',
    });

    // 3. Plan
    this.chief.createPlan(request);

    // 4. Create assignments
    const assignments = this.chief.createAssignments(request);

    // 5. Store assignments in agentdb
    await cfMemory('store', {
      key: 'cfa/assignments',
      value: JSON.stringify(assignments.map(a => ({
        assignmentId: a.assignmentId,
        agentType: a.agentType,
        stepRef: a.stepRef,
        task: request.plan!.steps.find(s => s.id === a.stepRef)?.description ?? query,
      }))),
      namespace: 'analysis',
    });

    // 6. Execute specialist agents in parallel
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
          eventBus: this.eventBus,
          callTool: this.callTool,
        };

        // Search agentdb for relevant prior analyses
        const priorContext = await cfMemory('search', {
          query: ctx.task,
          namespace: 'analysis',
          limit: '3',
        });
        if (priorContext && typeof priorContext === 'string') {
          ctx.priorContext = priorContext;
        }

        try {
          const result = await specialist.execute(ctx);
          assignment.status = 'completed';
          assignment.completedAt = new Date();
          assignment.resultRef = result.resultId;

          // Store result in agentdb for other agents and learning
          await cfMemory('store', {
            key: `cfa/results/${assignment.agentType}`,
            value: JSON.stringify({
              resultId: result.resultId,
              agentType: result.agentType,
              confidence: result.confidence,
              summary: result.summary,
              findingCount: result.findings.length,
            }),
            namespace: 'analysis',
          });

          return result;
        } catch {
          assignment.status = 'failed';
          assignment.completedAt = new Date();
          return null;
        }
      }),
    );

    const validResults = results.filter((r): r is AnalysisResult => r !== null);

    // 7. Aggregate
    const report = this.chief.aggregate(request, validResults);

    // 8. Store final report in agentdb for learning
    await cfMemory('store', {
      key: `cfa/reports/${request.requestId}`,
      value: JSON.stringify({
        requestId: request.requestId,
        query,
        confidence: request.confidence?.value,
        specialistsUsed: validResults.map(r => r.agentType),
        completedAt: new Date().toISOString(),
      }),
      namespace: 'analysis',
      tags: 'cfa-report,completed',
    });

    return { request, report, results: validResults };
  }
}
