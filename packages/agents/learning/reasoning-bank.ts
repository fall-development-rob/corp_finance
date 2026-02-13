// ReasoningBank — stores analysis patterns via agentic-flow SONA
// Uses claude-flow memory CLI with pattern-specific namespace for SONA learning
// Falls back to in-memory for environments without claude-flow

import { randomUUID, createHash } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import type {
  LearningPattern, ReasoningTrace, QualityFeedback, TaskType,
} from '../types/learning.js';

const execFileAsync = promisify(execFile);

export interface ReasoningBank {
  recordTrace(trace: ReasoningTrace): Promise<void>;
  recordFeedback(feedback: QualityFeedback): Promise<void>;
  searchPatterns(taskType: TaskType, limit?: number): Promise<LearningPattern[]>;
  getPattern(patternId: string): Promise<LearningPattern | null>;
  getStats(): { totalPatterns: number; totalTraces: number; avgReward: number };
}

// Execute claude-flow memory command
async function cfMemory(
  action: string,
  args: Record<string, string>,
): Promise<string | null> {
  const cmdArgs = ['@claude-flow/cli@latest', 'memory', action];
  for (const [k, v] of Object.entries(args)) {
    cmdArgs.push(`--${k}`, v);
  }
  try {
    const { stdout } = await execFileAsync('npx', cmdArgs, { timeout: 15000 });
    return stdout.trim();
  } catch {
    return null;
  }
}

// SONA-backed implementation using claude-flow
export class SonaReasoningBank implements ReasoningBank {
  private namespace = 'cfa-reasoning';
  private patternCount = 0;
  private traceCount = 0;
  private totalReward = 0;

  async recordTrace(trace: ReasoningTrace): Promise<void> {
    this.traceCount++;

    // Store trace in agentdb
    await cfMemory('store', {
      key: `trace/${trace.traceId}`,
      value: JSON.stringify(trace),
      namespace: this.namespace,
      tags: `trace,${trace.agentType},${trace.outcome}`,
    });

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

        const taskType = this.inferTaskType(trace);
        const pattern: LearningPattern = {
          patternId: randomUUID(),
          taskType,
          toolSequence: toolCalls,
          agentTypes: [trace.agentType],
          rewardScore: 0.5,
          usageCount: 1,
          fingerprint,
          createdAt: new Date(),
          lastUsedAt: new Date(),
        };

        // Store pattern for SONA learning
        await cfMemory('store', {
          key: `pattern/${pattern.patternId}`,
          value: JSON.stringify(pattern),
          namespace: this.namespace,
          tags: `pattern,${taskType},reward-${Math.round(pattern.rewardScore * 100)}`,
        });

        this.patternCount++;
        this.totalReward += pattern.rewardScore;
      }
    }
  }

  async recordFeedback(feedback: QualityFeedback): Promise<void> {
    // Store feedback for SONA reward signal
    await cfMemory('store', {
      key: `feedback/${feedback.feedbackId}`,
      value: JSON.stringify(feedback),
      namespace: this.namespace,
      tags: `feedback,score-${Math.round(feedback.score * 100)}`,
    });
  }

  async searchPatterns(taskType: TaskType, limit = 10): Promise<LearningPattern[]> {
    // Search patterns via agentdb HNSW
    const result = await cfMemory('search', {
      query: `${taskType} analysis pattern`,
      namespace: this.namespace,
      limit: String(limit),
    });

    if (!result) return [];

    try {
      const parsed = JSON.parse(result);
      if (Array.isArray(parsed)) {
        return parsed
          .filter((p: any) => p.taskType === taskType || p.tags?.includes(taskType))
          .map((p: any) => p as LearningPattern)
          .slice(0, limit);
      }
    } catch {
      // Fall through
    }
    return [];
  }

  async getPattern(patternId: string): Promise<LearningPattern | null> {
    const result = await cfMemory('retrieve', {
      key: `pattern/${patternId}`,
      namespace: this.namespace,
    });

    if (!result) return null;
    try {
      return JSON.parse(result) as LearningPattern;
    } catch {
      return null;
    }
  }

  getStats() {
    return {
      totalPatterns: this.patternCount,
      totalTraces: this.traceCount,
      avgReward: this.patternCount > 0 ? this.totalReward / this.patternCount : 0,
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

// In-memory fallback (same as original LocalReasoningBank)
export class LocalReasoningBank implements ReasoningBank {
  private patterns = new Map<string, LearningPattern>();
  private traces = new Map<string, ReasoningTrace>();
  private feedback = new Map<string, QualityFeedback>();

  async recordTrace(trace: ReasoningTrace): Promise<void> {
    this.traces.set(trace.traceId, trace);
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
    return { totalPatterns: patterns.length, totalTraces: this.traces.size, avgReward };
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
