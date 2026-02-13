// BC1: Analysis Orchestration - AnalysisRequest aggregate
// The Chief Analyst receives queries, decomposes into plans, assigns to specialists

export type Priority = 'CRITICAL' | 'HIGH' | 'STANDARD' | 'LOW';
export type AggregationStrategy = 'synthesis' | 'comparison' | 'weighted-consensus' | 'majority-vote';
export type AnalysisStatus = 'pending' | 'planning' | 'assigned' | 'in_progress' | 'aggregating' | 'completed' | 'escalated';
export type AssignmentStatus = 'pending' | 'in_progress' | 'completed' | 'failed' | 'skipped';

export interface QueryIntent {
  readonly type: 'valuation' | 'credit_assessment' | 'portfolio_construction' | 'risk_analysis' | 'deal_analysis' | 'macro_research' | 'esg_review' | 'regulatory_check' | 'comprehensive';
  readonly domains: string[];   // which specialist domains are needed
  readonly complexity: number;  // 0-1 estimate
}

export interface PlanStep {
  readonly id: string;
  readonly description: string;
  readonly requiredDomains: string[];  // maps to specialist agent types
  readonly dependencies: string[];     // IDs of prior steps this depends on
}

export interface ResearchPlan {
  readonly planId: string;
  readonly steps: PlanStep[];
  readonly estimatedDuration: number;  // ms
  readonly aggregationStrategy: AggregationStrategy;
}

export interface AnalystAssignment {
  assignmentId: string;
  stepRef: string;           // PlanStep.id
  agentType: string;         // e.g. 'equity-analyst', 'credit-analyst'
  status: AssignmentStatus;
  resultRef?: string;        // AnalysisResult.resultId when complete
  startedAt?: Date;
  completedAt?: Date;
}

export interface ConfidenceScore {
  readonly value: number;    // 0.0 - 1.0
  readonly reasoning: string;
}

export interface AnalysisRequest {
  requestId: string;
  query: string;
  intent: QueryIntent;
  priority: Priority;
  status: AnalysisStatus;
  plan?: ResearchPlan;
  assignments: AnalystAssignment[];
  aggregatedResult?: string;  // final markdown output
  confidence?: ConfidenceScore;
  createdAt: Date;
  completedAt?: Date;
}
