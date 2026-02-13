// Main Orchestrator — coordinates the full analysis pipeline
// Creates chief analyst, spawns specialists, manages lifecycle

import { randomUUID } from 'node:crypto';
import type { AnalysisRequest, Priority } from '../types/analysis.js';
import type { AnalysisResult, AgentType } from '../types/agents.js';
import type { EventBus } from '../types/events.js';
import { SimpleEventBus } from '../types/events.js';
import { ChiefAnalyst } from '../agents/chief-analyst.js';
import { createSpecialist } from './specialist-factory.js';
import type { BaseAnalyst, AnalystContext } from '../agents/base-analyst.js';

export interface OrchestratorConfig {
  confidenceThreshold?: number;
  maxSpecialists?: number;
  callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  onEvent?: (event: { type: string; payload: unknown }) => void;
}

export class Orchestrator {
  private chief: ChiefAnalyst;
  private eventBus: EventBus;
  private callTool: OrchestratorConfig['callTool'];

  constructor(config: OrchestratorConfig) {
    this.eventBus = new SimpleEventBus();
    this.callTool = config.callTool;

    this.chief = new ChiefAnalyst({
      confidenceThreshold: config.confidenceThreshold ?? 0.6,
      maxSpecialists: config.maxSpecialists ?? 6,
      eventBus: this.eventBus,
    });

    // Forward events if callback provided
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

  // Run a full analysis pipeline
  async analyze(query: string, priority: Priority = 'STANDARD'): Promise<{
    request: AnalysisRequest;
    report: string;
    results: AnalysisResult[];
  }> {
    // 1. Create request
    const request = this.chief.createRequest(query, priority);

    // 2. Plan
    this.chief.createPlan(request);

    // 3. Create assignments
    const assignments = this.chief.createAssignments(request);

    // 4. Execute specialist agents in parallel
    const results = await Promise.all(
      assignments.map(async (assignment) => {
        const specialist = createSpecialist(assignment.agentType as AgentType);
        if (!specialist) {
          assignment.status = 'skipped';
          return null;
        }

        assignment.status = 'in_progress';
        assignment.startedAt = new Date();

        const ctx: AnalystContext = {
          assignmentId: assignment.assignmentId,
          requestId: request.requestId,
          task: request.plan!.steps.find(s => s.id === assignment.stepRef)?.description ?? request.query,
          eventBus: this.eventBus,
          callTool: this.callTool,
        };

        try {
          const result = await specialist.execute(ctx);
          assignment.status = 'completed';
          assignment.completedAt = new Date();
          assignment.resultRef = result.resultId;
          return result;
        } catch (err) {
          assignment.status = 'failed';
          assignment.completedAt = new Date();
          return null;
        }
      }),
    );

    const validResults = results.filter((r): r is AnalysisResult => r !== null);

    // 5. Aggregate
    const report = this.chief.aggregate(request, validResults);

    return { request, report, results: validResults };
  }
}
