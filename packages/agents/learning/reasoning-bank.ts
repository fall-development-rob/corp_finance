// ReasoningBank — stores analysis patterns via agentic-flow
// Uses agentic-flow/reasoningbank for trajectory recording, judging, and distillation
// Falls back to in-memory for environments without agentic-flow

import { randomUUID, createHash } from 'node:crypto';
import {
  initialize as initReasoningBank,
  runTask,
  retrieveMemories,
  judgeTrajectory,
  distillMemories,
  db,
  computeEmbedding,
} from 'agentic-flow/reasoningbank';
import type { Trajectory } from 'agentic-flow/reasoningbank';
import type {
  LearningPattern, ReasoningTrace, QualityFeedback, TaskType,
} from '../types/learning.js';

export interface ReasoningBank {
  recordTrace(trace: ReasoningTrace): Promise<void>;
  recordFeedback(feedback: QualityFeedback): Promise<void>;
  searchPatterns(taskType: TaskType, limit?: number): Promise<LearningPattern[]>;
  getPattern(patternId: string): Promise<LearningPattern | null>;
  getStats(): { totalPatterns: number; totalTraces: number; avgReward: number };
}

// agentic-flow ReasoningBank implementation
export class SonaReasoningBank implements ReasoningBank {
  private patternCount = 0;
  private traceCount = 0;
  private totalReward = 0;
  private initialized = false;

  private async ensureInit(): Promise<void> {
    if (this.initialized) return;
    await initReasoningBank();
    this.initialized = true;
  }

  async recordTrace(trace: ReasoningTrace): Promise<void> {
    await this.ensureInit();
    this.traceCount++;

    // Build trajectory in agentic-flow format
    const trajectory: Trajectory = {
      steps: trace.steps.map(s => ({
        action: s.phase,
        summary: s.content,
        toolCalls: s.toolCalls,
        timestamp: new Date().toISOString(),
      })),
      metadata: {
        agentType: trace.agentType,
        requestId: trace.requestId,
        outcome: trace.outcome,
      },
    };

    // Store trajectory in ReasoningBank DB
    db.storeTrajectory({
      task_id: trace.requestId,
      agent_id: trace.agentType,
      query: trace.steps[0]?.content ?? '',
      trajectory_json: JSON.stringify(trajectory),
      started_at: new Date().toISOString(),
      ended_at: new Date().toISOString(),
      judge_label: trace.outcome === 'success' ? 'Success' : 'Failure',
      judge_conf: trace.outcome === 'success' ? 0.8 : 0.2,
    });

    // Judge and distill if successful
    if (trace.outcome === 'success') {
      const query = trace.steps[0]?.content ?? '';

      try {
        const verdict = await judgeTrajectory(trajectory, query);
        if (verdict.label === 'Success') {
          // Distill memories from successful trajectory
          const memoryIds = await distillMemories(trajectory, verdict, query, {
            taskId: trace.requestId,
            agentId: trace.agentType,
            domain: this.inferDomain(trace),
          });

          this.patternCount += memoryIds.length;
          this.totalReward += verdict.confidence * memoryIds.length;
        }
      } catch {
        // Judge/distill are best-effort
      }

      // Also store as a learning pattern
      const toolCalls = trace.steps
        .filter(s => s.phase === 'act' && s.toolCalls)
        .flatMap(s => s.toolCalls ?? []);

      if (toolCalls.length > 0) {
        const fingerprint = createHash('sha256')
          .update(toolCalls.sort().join(','))
          .digest('hex')
          .slice(0, 16);

        const taskType = this.inferTaskType(trace);
        const patternId = randomUUID();

        db.upsertMemory({
          id: patternId,
          type: 'reasoning_memory',
          pattern_data: {
            title: `${taskType}-pattern`,
            description: `Tool sequence: ${toolCalls.join(' → ')}`,
            content: JSON.stringify({
              patternId,
              taskType,
              toolSequence: toolCalls,
              agentTypes: [trace.agentType],
              rewardScore: 0.5,
              fingerprint,
            }),
            source: {
              task_id: trace.requestId,
              agent_id: trace.agentType,
              outcome: 'Success',
              evidence: toolCalls,
            },
            tags: ['cfa-pattern', taskType],
            domain: this.inferDomain(trace),
            created_at: new Date().toISOString(),
            confidence: 0.5,
            n_uses: 1,
          },
          confidence: 0.5,
          usage_count: 1,
        });

        // Generate embedding for pattern search
        try {
          const embedding = await computeEmbedding(
            `${taskType} ${toolCalls.join(' ')} ${trace.agentType}`,
          );
          db.upsertEmbedding({
            id: patternId,
            model: 'all-MiniLM-L6-v2',
            dims: embedding.length,
            vector: embedding,
            created_at: new Date().toISOString(),
          });
        } catch { /* embedding is best-effort */ }

        this.patternCount++;
        this.totalReward += 0.5;
      }
    }
  }

  async recordFeedback(feedback: QualityFeedback): Promise<void> {
    await this.ensureInit();

    // Store feedback as a reasoning memory for reward signal
    db.upsertMemory({
      id: feedback.feedbackId,
      type: 'reasoning_memory',
      pattern_data: {
        title: 'quality-feedback',
        description: `Score: ${feedback.score}`,
        content: JSON.stringify(feedback),
        source: {
          task_id: feedback.requestId,
          agent_id: 'feedback',
          outcome: feedback.score >= 0.5 ? 'Success' : 'Failure',
          evidence: [],
        },
        tags: ['cfa-feedback', `score-${Math.round(feedback.score * 100)}`],
        domain: 'cfa-learning',
        created_at: new Date().toISOString(),
        confidence: feedback.score,
        n_uses: 0,
      },
      confidence: feedback.score,
      usage_count: 0,
    });
  }

  async searchPatterns(taskType: TaskType, limit = 10): Promise<LearningPattern[]> {
    await this.ensureInit();

    // Use ReasoningBank MMR retrieval for semantic pattern search
    const memories = await retrieveMemories(`${taskType} analysis pattern`, {
      k: limit,
      domain: this.domainForTaskType(taskType),
    });

    return memories
      .map(m => {
        try {
          const data = JSON.parse(m.content);
          return {
            patternId: data.patternId ?? m.id,
            taskType: data.taskType ?? taskType,
            toolSequence: data.toolSequence ?? [],
            agentTypes: data.agentTypes ?? [],
            rewardScore: m.components.reliability,
            usageCount: 0,
            fingerprint: data.fingerprint ?? m.id.slice(0, 16),
            createdAt: new Date(),
            lastUsedAt: new Date(),
          } as LearningPattern;
        } catch {
          return null;
        }
      })
      .filter((p): p is LearningPattern => p !== null)
      .slice(0, limit);
  }

  async getPattern(patternId: string): Promise<LearningPattern | null> {
    await this.ensureInit();

    const allMemories = db.getAllActiveMemories();
    const found = allMemories.find(m => m.id === patternId);
    if (!found) return null;

    try {
      const data = JSON.parse(found.pattern_data.content);
      return {
        patternId: data.patternId ?? found.id,
        taskType: data.taskType ?? 'valuation',
        toolSequence: data.toolSequence ?? [],
        agentTypes: data.agentTypes ?? [],
        rewardScore: found.confidence,
        usageCount: found.usage_count,
        fingerprint: data.fingerprint ?? found.id.slice(0, 16),
        createdAt: new Date(found.created_at),
        lastUsedAt: new Date(found.last_used ?? found.created_at),
      };
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

  private inferDomain(trace: ReasoningTrace): string {
    return `cfa-${this.inferTaskType(trace).replace(/_/g, '-')}`;
  }

  private domainForTaskType(taskType: TaskType): string {
    return `cfa-${taskType.replace(/_/g, '-')}`;
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
