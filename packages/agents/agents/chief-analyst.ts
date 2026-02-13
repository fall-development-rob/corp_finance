// CFA Chief Analyst — Coordinator Agent
// Receives queries, creates research plans, delegates to specialists, aggregates results

import { randomUUID } from 'node:crypto';
import type {
  AnalysisRequest, QueryIntent, ResearchPlan, PlanStep,
  AnalystAssignment, ConfidenceScore, Priority, AggregationStrategy,
} from '../types/analysis.js';
import type { AnalysisResult } from '../types/agents.js';
import type { EventBus } from '../types/events.js';
import { TOOL_MAPPINGS, AGENT_DESCRIPTIONS, suggestAgents } from '../config/tool-mappings.js';

export interface ChiefAnalystConfig {
  confidenceThreshold: number;     // below this, escalate for human review (default 0.6)
  maxSpecialists: number;          // max concurrent specialists (default 6)
  eventBus: EventBus;
}

export class ChiefAnalyst {
  private config: ChiefAnalystConfig;
  private eventBus: EventBus;

  constructor(config: ChiefAnalystConfig) {
    this.config = config;
    this.eventBus = config.eventBus;
  }

  // Step 1: Create an AnalysisRequest from a user query
  createRequest(query: string, priority: Priority = 'STANDARD'): AnalysisRequest {
    const intent = this.classifyIntent(query);

    const request: AnalysisRequest = {
      requestId: randomUUID(),
      query,
      intent,
      priority,
      status: 'pending',
      assignments: [],
      createdAt: new Date(),
    };

    this.eventBus.emit({
      eventId: randomUUID(),
      type: 'AnalysisRequested',
      timestamp: new Date(),
      sourceContext: 'AnalysisOrchestration',
      payload: { requestId: request.requestId, query, priority },
    });

    return request;
  }

  // Step 2: Decompose query into a research plan
  createPlan(request: AnalysisRequest): ResearchPlan {
    const agentTypes = suggestAgents(request.intent.domains);
    const steps: PlanStep[] = agentTypes.map((agentType, idx) => ({
      id: `step-${idx + 1}`,
      description: `${AGENT_DESCRIPTIONS[agentType] ?? agentType}: Analyze relevant aspects of "${request.query}"`,
      requiredDomains: TOOL_MAPPINGS[agentType] ?? [],
      dependencies: [],  // parallel by default; coordinator adds deps for sequential needs
    }));

    // Add a final synthesis step
    steps.push({
      id: `step-synthesis`,
      description: 'Synthesize all specialist findings into a unified analysis',
      requiredDomains: [],
      dependencies: steps.filter(s => s.id !== 'step-synthesis').map(s => s.id),
    });

    const plan: ResearchPlan = {
      planId: randomUUID(),
      steps,
      estimatedDuration: steps.length * 10000,  // rough estimate
      aggregationStrategy: this.selectStrategy(request.intent),
    };

    request.plan = plan;
    request.status = 'planning';

    this.eventBus.emit({
      eventId: randomUUID(),
      type: 'PlanCreated',
      timestamp: new Date(),
      sourceContext: 'AnalysisOrchestration',
      payload: { requestId: request.requestId, planId: plan.planId, steps: plan.steps },
    });

    return plan;
  }

  // Step 3: Create assignments for specialist agents
  createAssignments(request: AnalysisRequest): AnalystAssignment[] {
    if (!request.plan) throw new Error('Plan must exist before creating assignments');

    const assignments: AnalystAssignment[] = [];

    for (const step of request.plan.steps) {
      if (step.id === 'step-synthesis') continue;  // synthesis is done by chief

      const agentType = this.inferAgentType(step);
      const assignment: AnalystAssignment = {
        assignmentId: randomUUID(),
        stepRef: step.id,
        agentType,
        status: 'pending',
      };

      assignments.push(assignment);

      this.eventBus.emit({
        eventId: randomUUID(),
        type: 'AnalystAssigned',
        timestamp: new Date(),
        sourceContext: 'AnalysisOrchestration',
        payload: { requestId: request.requestId, assignmentId: assignment.assignmentId, agentType, stepRef: step.id },
      });
    }

    request.assignments = assignments;
    request.status = 'assigned';
    return assignments;
  }

  // Step 4: Aggregate specialist results into final output
  aggregate(request: AnalysisRequest, results: AnalysisResult[]): string {
    request.status = 'aggregating';

    // Build the final report from specialist findings
    const sections: string[] = [];
    sections.push(`# Analysis: ${request.query}\n`);
    sections.push(`**Request ID**: ${request.requestId}`);
    sections.push(`**Priority**: ${request.priority}`);
    sections.push(`**Strategy**: ${request.plan?.aggregationStrategy ?? 'synthesis'}\n`);
    sections.push('---\n');

    for (const result of results) {
      sections.push(`## ${result.agentType.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}\n`);
      sections.push(`**Confidence**: ${(result.confidence * 100).toFixed(1)}%\n`);

      for (const finding of result.findings) {
        sections.push(`### ${finding.methodology}\n`);
        sections.push(finding.statement);

        if (finding.citations.length > 0) {
          sections.push('\n**Sources**:');
          for (const cite of finding.citations) {
            sections.push(`- \`${cite.toolName}\`: ${cite.relevantOutput}`);
          }
        }
        sections.push('');
      }
      sections.push('---\n');
    }

    // Confidence assessment
    const avgConfidence = results.length > 0
      ? results.reduce((sum, r) => sum + r.confidence, 0) / results.length
      : 0;

    const confidence: ConfidenceScore = {
      value: avgConfidence,
      reasoning: `Based on ${results.length} specialist analyses with average tool success rate`,
    };

    sections.push(`## Overall Confidence: ${(confidence.value * 100).toFixed(1)}%\n`);
    sections.push(`*${confidence.reasoning}*`);

    const aggregated = sections.join('\n');
    request.aggregatedResult = aggregated;
    request.confidence = confidence;
    request.status = confidence.value >= this.config.confidenceThreshold ? 'completed' : 'escalated';
    request.completedAt = new Date();

    this.eventBus.emit({
      eventId: randomUUID(),
      type: confidence.value >= this.config.confidenceThreshold ? 'ResultAggregated' : 'AnalysisEscalated',
      timestamp: new Date(),
      sourceContext: 'AnalysisOrchestration',
      payload: { requestId: request.requestId, confidence: confidence.value },
    });

    return aggregated;
  }

  // Classify the query intent to determine which domains/specialists are needed
  private classifyIntent(query: string): QueryIntent {
    const q = query.toLowerCase();
    const domains: string[] = [];
    let type: QueryIntent['type'] = 'comprehensive';

    // Pattern matching for domain detection
    const domainPatterns: Record<string, string[]> = {
      valuation: ['dcf', 'valuation', 'fair value', 'intrinsic value', 'comps', 'multiples', 'sum of parts'],
      equity_research: ['equity', 'stock', 'earnings', 'eps', 'revenue growth', 'margin'],
      credit: ['credit', 'default', 'spread', 'covenant', 'rating', 'leverage'],
      fixed_income: ['bond', 'yield', 'duration', 'convexity', 'coupon', 'fixed income'],
      derivatives: ['option', 'derivative', 'swap', 'futures', 'greeks', 'volatility'],
      quant_risk: ['var', 'risk', 'sharpe', 'drawdown', 'factor', 'beta'],
      portfolio_optimization: ['portfolio', 'allocation', 'rebalance', 'efficient frontier'],
      macro_economics: ['macro', 'gdp', 'inflation', 'rates', 'central bank'],
      esg: ['esg', 'sustainability', 'carbon', 'governance', 'social'],
      regulatory: ['regulatory', 'compliance', 'aml', 'fatca', 'basel'],
      pe: ['lbo', 'buyout', 'private equity', 'leverage'],
      ma: ['m&a', 'merger', 'acquisition', 'accretion', 'dilution'],
      restructuring: ['restructuring', 'distressed', 'bankruptcy', 'workout'],
    };

    for (const [domain, patterns] of Object.entries(domainPatterns)) {
      if (patterns.some(p => q.includes(p))) {
        domains.push(domain);
      }
    }

    // Set primary type based on strongest signal
    if (domains.includes('valuation') || domains.includes('equity_research')) type = 'valuation';
    else if (domains.includes('credit')) type = 'credit_assessment';
    else if (domains.includes('portfolio_optimization')) type = 'portfolio_construction';
    else if (domains.includes('quant_risk')) type = 'risk_analysis';
    else if (domains.includes('pe') || domains.includes('ma')) type = 'deal_analysis';
    else if (domains.includes('macro_economics')) type = 'macro_research';
    else if (domains.includes('esg')) type = 'esg_review';
    else if (domains.includes('regulatory')) type = 'regulatory_check';

    // If no specific domains detected, default to comprehensive
    if (domains.length === 0) {
      domains.push('valuation', 'credit', 'quant_risk');
      type = 'comprehensive';
    }

    return {
      type,
      domains,
      complexity: Math.min(1, domains.length / 5),
    };
  }

  private selectStrategy(intent: QueryIntent): AggregationStrategy {
    if (intent.type === 'comprehensive') return 'synthesis';
    if (intent.domains.length === 1) return 'synthesis';
    if (intent.type === 'risk_analysis') return 'weighted-consensus';
    return 'synthesis';
  }

  private inferAgentType(step: PlanStep): string {
    // Find the agent type whose tool domains best match this step's required domains
    let bestMatch = 'equity-analyst';
    let bestScore = 0;

    for (const [agentType, tools] of Object.entries(TOOL_MAPPINGS)) {
      const overlap = step.requiredDomains.filter(d => tools.includes(d)).length;
      if (overlap > bestScore) {
        bestScore = overlap;
        bestMatch = agentType;
      }
    }
    return bestMatch;
  }
}
