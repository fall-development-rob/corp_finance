// BC4: Learning & Adaptation - LearningPattern aggregate
// SONA reinforcement learning + ReasoningBank

export type TaskType = 'valuation' | 'credit_assessment' | 'risk_analysis' | 'deal_analysis' | 'portfolio_construction' | 'macro_research' | 'esg_review' | 'regulatory_check';

export interface ReasoningStep {
  readonly phase: 'observe' | 'think' | 'act' | 'reflect';
  readonly content: string;
  readonly toolCalls?: string[];
  readonly timestamp: Date;
}

export interface ReasoningTrace {
  traceId: string;
  agentType: string;
  requestId: string;
  steps: ReasoningStep[];
  outcome: 'success' | 'partial' | 'failure';
  createdAt: Date;
}

export interface QualityFeedback {
  feedbackId: string;
  requestId: string;
  score: number;        // 0-1, SONA reward signal
  comments?: string;
  automated: boolean;   // true = system-generated, false = human
  createdAt: Date;
}

export interface LearningPattern {
  patternId: string;
  taskType: TaskType;
  toolSequence: string[];     // ordered tool names that worked well
  agentTypes: string[];       // which specialists were involved
  rewardScore: number;        // cumulative SONA reward
  usageCount: number;
  fingerprint: string;        // content hash for dedup
  createdAt: Date;
  lastUsedAt: Date;
}

export interface AdaptationDelta {
  readonly patternId: string;
  readonly oldToolSequence: string[];
  readonly newToolSequence: string[];
  readonly rewardDelta: number;
  readonly reason: string;
}
