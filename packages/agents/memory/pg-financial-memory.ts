// PgFinancialMemory — PostgreSQL-backed FinancialMemory via ruvector-postgres
// Uses pg pool + ruvector extension for vector similarity search (384-dim)

import { randomUUID } from 'node:crypto';
import { computeEmbedding } from 'agentic-flow/reasoningbank';
import { getPool, float32ToVectorLiteral } from '../db/pg-client.js';
import type { FinancialMemory } from './financial-memory.js';
import type { MemoryEntry, MemoryMetadata, RetrievalContext } from '../types/memory.js';

export class PgFinancialMemory implements FinancialMemory {
  private domain: string;

  constructor(domain = 'cfa-analysis') {
    this.domain = domain;
  }

  async store(content: string, metadata: MemoryMetadata): Promise<MemoryEntry> {
    const pool = await getPool();
    const entryId = randomUUID();

    const tags = [
      metadata.sourceType,
      ...(metadata.tickers ?? []),
      ...(metadata.tags ?? []),
      metadata.sector,
      metadata.analysisType,
    ].filter((t): t is string => Boolean(t));

    // Generate embedding for vector search
    const embedding = await computeEmbedding(content);
    const vecLiteral = float32ToVectorLiteral(embedding);

    await pool.query(
      `INSERT INTO reasoning_memories
        (id, type, title, description, content, domain, tags, source_json, confidence, usage_count, embedding)
       VALUES ($1, 'reasoning_memory', $2, $3, $4, $5, $6, $7, 0.8, 0, $8::ruvector)`,
      [
        entryId,
        metadata.analysisType ?? 'financial-analysis',
        tags.join(', '),
        content,
        this.domain,
        tags,
        JSON.stringify({
          task_id: entryId,
          agent_id: metadata.sourceType ?? 'cfa-agent',
          outcome: 'Success',
          evidence: tags,
        }),
        vecLiteral,
      ],
    );

    return {
      entryId,
      archiveId: 'default',
      content,
      metadata,
      embeddingModel: 'all-MiniLM-L6-v2',
      retentionTier: 'hot',
      createdAt: new Date(),
      lastAccessedAt: new Date(),
      accessCount: 0,
    };
  }

  async search(query: string, limit = 10): Promise<RetrievalContext> {
    const pool = await getPool();

    const embedding = await computeEmbedding(query);
    const vecLiteral = float32ToVectorLiteral(embedding);

    const { rows } = await pool.query<{
      id: string;
      title: string;
      content: string;
      domain: string;
      tags: string[];
      confidence: number;
      usage_count: number;
      similarity: number;
    }>(
      `SELECT * FROM search_reasoning_memories($1::ruvector, $2, $3)`,
      [vecLiteral, this.domain, limit],
    );

    const entries = rows.map(r => ({
      entry: {
        entryId: r.id,
        archiveId: 'default',
        content: r.content,
        metadata: {
          sourceType: 'analysis' as const,
          tags: r.tags ?? [],
        } as MemoryMetadata,
        embeddingModel: 'all-MiniLM-L6-v2',
        retentionTier: 'hot' as const,
        createdAt: new Date(),
        lastAccessedAt: new Date(),
        accessCount: r.usage_count,
      },
      similarityScore: r.similarity,
    }));

    return { entries, query, totalSearched: entries.length };
  }

  async retrieve(entryId: string): Promise<MemoryEntry | null> {
    const pool = await getPool();

    const { rows } = await pool.query<{
      id: string;
      content: string;
      domain: string;
      tags: string[];
      confidence: number;
      usage_count: number;
      created_at: Date;
    }>(
      `UPDATE reasoning_memories
         SET usage_count = usage_count + 1, last_used_at = now(), updated_at = now()
       WHERE id = $1
       RETURNING id, content, domain, tags, confidence, usage_count, created_at`,
      [entryId],
    );

    if (rows.length === 0) return null;

    const r = rows[0];
    return {
      entryId: r.id,
      archiveId: 'default',
      content: r.content,
      metadata: {
        sourceType: 'analysis',
        tags: r.tags ?? [],
        sector: r.domain,
      } as MemoryMetadata,
      embeddingModel: 'all-MiniLM-L6-v2',
      retentionTier: 'hot',
      createdAt: new Date(r.created_at),
      lastAccessedAt: new Date(),
      accessCount: r.usage_count,
    };
  }

  async getByTicker(ticker: string, limit = 10): Promise<MemoryEntry[]> {
    const pool = await getPool();

    // Use embedding similarity to find ticker-related entries
    const embedding = await computeEmbedding(ticker);
    const vecLiteral = float32ToVectorLiteral(embedding);

    const { rows } = await pool.query<{
      id: string;
      content: string;
      domain: string;
      tags: string[];
      confidence: number;
      usage_count: number;
      similarity: number;
    }>(
      `SELECT * FROM search_reasoning_memories($1::ruvector, $2, $3)`,
      [vecLiteral, this.domain, limit],
    );

    return rows.map(r => ({
      entryId: r.id,
      archiveId: 'default',
      content: r.content,
      metadata: {
        sourceType: 'analysis' as const,
        tags: r.tags ?? [],
        tickers: [ticker],
      } as MemoryMetadata,
      embeddingModel: 'all-MiniLM-L6-v2',
      retentionTier: 'hot' as const,
      createdAt: new Date(),
      lastAccessedAt: new Date(),
      accessCount: r.usage_count,
    }));
  }
}
