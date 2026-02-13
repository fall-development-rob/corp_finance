// ReasoningBank — stores successful analysis patterns for SONA learning
// In-memory implementation with interface for production backend

import { randomUUID } from 'node:crypto';
import { createHash } from 'node:crypto';
import type {
  LearningPattern, ReasoningTrace, QualityFeedback,
  TaskType, AdaptationDelta,
} from '../types/learning.js';

export interface ReasoningBank {
  recordTrace(trace: ReasoningTrace): Promise<void>;
  recordFeedback(feedback: QualityFeedback): Promise<void>;
  searchPatterns(taskType: TaskType, limit?: number): Promise<LearningPattern[]>;
  getPattern(patternId: string): Promise<LearningPattern | null>;
  getStats(): { totalPatterns: number; totalTraces: number; avgReward: number };
}

export class LocalReasoningBank implements ReasoningBank {
  private patterns = new Map<string, LearningPattern>();
  private traces = new Map<string, ReasoningTrace>();
  private feedback = new Map<string, QualityFeedback>();

  async recordTrace(trace: ReasoningTrace): Promise<void> {
    this.traces.set(trace.traceId, trace);

    // Extract pattern from successful traces
    if (trace.outcome === 'success') {
      const toolCalls = trace.steps
        .filter(s => s.phase === 'act' && s.toolCalls)
        .flatMap(s => s.toolCalls ?? []);

      if (toolCalls.length > 0) {
        const fingerprint = createHash('sha256')
          .update(toolCalls.sort().join(','))
          .digest('hex')
          .slice(0, 16);

        const existing = [...this.patterns.values()].find(p => p.fingerprint === fingerprint);
        if (existing) {
          existing.usageCount++;
          existing.lastUsedAt = new Date();
        } else {
          const pattern: LearningPattern = {
            patternId: randomUUID(),
            taskType: this.inferTaskType(trace),
            toolSequence: toolCalls,
            agentTypes: [trace.agentType],
            rewardScore: 0.5,
            usageCount: 1,
            fingerprint,
            createdAt: new Date(),
            lastUsedAt: new Date(),
          };
          this.patterns.set(pattern.patternId, pattern);
        }
      }
    }
  }

  async recordFeedback(feedback: QualityFeedback): Promise<void> {
    this.feedback.set(feedback.feedbackId, feedback);

    // Update pattern rewards based on feedback
    // Find traces for this request and boost/decay their patterns
    for (const trace of this.traces.values()) {
      if (trace.requestId === feedback.requestId) {
        const toolCalls = trace.steps
          .filter(s => s.phase === 'act' && s.toolCalls)
          .flatMap(s => s.toolCalls ?? []);

        const fingerprint = createHash('sha256')
          .update(toolCalls.sort().join(','))
          .digest('hex')
          .slice(0, 16);

        for (const pattern of this.patterns.values()) {
          if (pattern.fingerprint === fingerprint) {
            // Exponential moving average for reward
            pattern.rewardScore = pattern.rewardScore * 0.7 + feedback.score * 0.3;
          }
        }
      }
    }
  }

  async searchPatterns(taskType: TaskType, limit = 10): Promise<LearningPattern[]> {
    return [...this.patterns.values()]
      .filter(p => p.taskType === taskType)
      .sort((a, b) => b.rewardScore - a.rewardScore)
      .slice(0, limit);
  }

  async getPattern(patternId: string): Promise<LearningPattern | null> {
    return this.patterns.get(patternId) ?? null;
  }

  getStats() {
    const patterns = [...this.patterns.values()];
    const avgReward = patterns.length > 0
      ? patterns.reduce((s, p) => s + p.rewardScore, 0) / patterns.length
      : 0;
    return {
      totalPatterns: patterns.length,
      totalTraces: this.traces.size,
      avgReward,
    };
  }

  private inferTaskType(trace: ReasoningTrace): TaskType {
    const type = trace.agentType;
    if (type.includes('equity')) return 'valuation';
    if (type.includes('credit')) return 'credit_assessment';
    if (type.includes('risk') || type.includes('quant')) return 'risk_analysis';
    if (type.includes('macro')) return 'macro_research';
    if (type.includes('esg')) return 'esg_review';
    if (type.includes('private') || type.includes('pe')) return 'deal_analysis';
    if (type.includes('portfolio')) return 'portfolio_construction';
    if (type.includes('regulatory')) return 'regulatory_check';
    return 'valuation';
  }
}
