// ADR-006: Cross-Specialist Collaboration types

import type { AgentType } from './agents.js';

export type InsightType = 'finding' | 'risk' | 'opportunity' | 'metric';

export interface AgentInsight {
  readonly id: string;
  readonly sourceAgent: AgentType;
  readonly sourceAgentId: string;
  readonly insightType: InsightType;
  readonly content: string;
  readonly data: Record<string, unknown>;
  readonly confidence: number;
  readonly timestamp: Date;
}

export interface InsightFilter {
  sourceAgent?: AgentType;
  insightType?: InsightType;
  minConfidence?: number;
  since?: Date;
}
