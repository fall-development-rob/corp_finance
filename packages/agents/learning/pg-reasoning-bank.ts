// PgReasoningBank — PostgreSQL-backed ReasoningBank via ruvector-postgres
// Uses pg pool + ruvector extension for vector-based pattern retrieval (384-dim)

import { randomUUID, createHash } from 'node:crypto';
import { computeEmbedding } from 'agentic-flow/reasoningbank';
import { getPool, float32ToVectorLiteral } from '../db/pg-client.js';
import type { ReasoningBank } from './reasoning-bank.js';
import type {
  LearningPattern, ReasoningTrace, QualityFeedback, TaskType,
} from '../types/learning.js';

export class PgReasoningBank implements ReasoningBank {
  private patternCount = 0;
  private traceCount = 0;
  private totalReward = 0;

  async recordTrace(trace: ReasoningTrace): Promise<void> {
    const pool = await getPool();
    this.traceCount++;

    const trajectory = {
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

    // Store trajectory
    await pool.query(
      `INSERT INTO task_trajectories
        (task_id, agent_id, query, trajectory_json, judge_label, judge_conf, started_at, ended_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
      [
        trace.requestId,
        trace.agentType,
        trace.steps[0]?.content ?? '',
        JSON.stringify(trajectory),
        trace.outcome === 'success' ? 'Success' : 'Failure',
        trace.outcome === 'success' ? 0.8 : 0.2,
        trace.createdAt.toISOString(),
        new Date().toISOString(),
      ],
    );

    // Create learning pattern from successful traces with tool calls
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
        const domain = this.inferDomain(trace);
        const patternId = randomUUID();

        const patternContent = JSON.stringify({
          patternId,
          taskType,
          toolSequence: toolCalls,
          agentTypes: [trace.agentType],
          rewardScore: 0.5,
          fingerprint,
        });

        // Generate embedding for the pattern
        const embedding = await computeEmbedding(
          `${taskType} ${toolCalls.join(' ')} ${trace.agentType}`,
        );
        const vecLiteral = float32ToVectorLiteral(embedding);

        await pool.query(
          `INSERT INTO reasoning_memories
            (id, type, title, description, content, domain, tags, source_json, confidence, usage_count, embedding)
           VALUES ($1, 'reasoning_memory', $2, $3, $4, $5, $6, $7, 0.5, 1, $8::ruvector)`,
          [
            patternId,
            `${taskType}-pattern`,
            `Tool sequence: ${toolCalls.join(' → ')}`,
            patternContent,
            domain,
            ['cfa-pattern', taskType],
            JSON.stringify({
              task_id: trace.requestId,
              agent_id: trace.agentType,
              outcome: 'Success',
              evidence: toolCalls,
            }),
            vecLiteral,
          ],
        );

        this.patternCount++;
        this.totalReward += 0.5;
      }
    }
  }

  async recordFeedback(feedback: QualityFeedback): Promise<void> {
    const pool = await getPool();

    // Store feedback as a reasoning memory for reward signal
    // Use gen_random_uuid() since feedbackId may not be a valid UUID
    await pool.query(
      `INSERT INTO reasoning_memories
        (id, type, title, description, content, domain, tags, source_json, confidence, usage_count)
       VALUES (gen_random_uuid(), 'reasoning_memory', 'quality-feedback', $1, $2, 'cfa-learning', $3, $4, $5, 0)`,
      [
        `Score: ${feedback.score}`,
        JSON.stringify(feedback),
        ['cfa-feedback', `score-${Math.round(feedback.score * 100)}`],
        JSON.stringify({
          feedback_id: feedback.feedbackId,
          task_id: feedback.requestId,
          agent_id: 'feedback',
          outcome: feedback.score >= 0.5 ? 'Success' : 'Failure',
          evidence: [],
        }),
        feedback.score,
      ],
    );
  }

  async searchPatterns(taskType: TaskType, limit = 10): Promise<LearningPattern[]> {
    const pool = await getPool();
    const domain = this.domainForTaskType(taskType);

    // Generate query embedding
    const embedding = await computeEmbedding(`${taskType} analysis pattern`);
    const vecLiteral = float32ToVectorLiteral(embedding);

    const { rows } = await pool.query<{
      id: string;
      content: string;
      confidence: number;
      usage_count: number;
      similarity: number;
    }>(
      `SELECT * FROM search_reasoning_memories($1::ruvector, $2, $3)`,
      [vecLiteral, domain, limit],
    );

    return rows
      .map(r => {
        try {
          const data = JSON.parse(r.content);
          return {
            patternId: data.patternId ?? r.id,
            taskType: data.taskType ?? taskType,
            toolSequence: data.toolSequence ?? [],
            agentTypes: data.agentTypes ?? [],
            rewardScore: r.confidence,
            usageCount: r.usage_count,
            fingerprint: data.fingerprint ?? r.id.slice(0, 16),
            createdAt: new Date(),
            lastUsedAt: new Date(),
          } as LearningPattern;
        } catch {
          return null;
        }
      })
      .filter((p): p is LearningPattern => p !== null);
  }

  async getPattern(patternId: string): Promise<LearningPattern | null> {
    const pool = await getPool();

    const { rows } = await pool.query<{
      id: string;
      content: string;
      confidence: number;
      usage_count: number;
      created_at: Date;
      last_used_at: Date | null;
    }>(
      `SELECT id, content, confidence, usage_count, created_at, last_used_at
       FROM reasoning_memories WHERE id = $1`,
      [patternId],
    );

    if (rows.length === 0) return null;

    const r = rows[0];
    try {
      const data = JSON.parse(r.content);
      return {
        patternId: data.patternId ?? r.id,
        taskType: data.taskType ?? 'valuation',
        toolSequence: data.toolSequence ?? [],
        agentTypes: data.agentTypes ?? [],
        rewardScore: r.confidence,
        usageCount: r.usage_count,
        fingerprint: data.fingerprint ?? r.id.slice(0, 16),
        createdAt: new Date(r.created_at),
        lastUsedAt: new Date(r.last_used_at ?? r.created_at),
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
