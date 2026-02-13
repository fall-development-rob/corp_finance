// Financial Memory — persistent analysis storage via agentic-flow ReasoningBank
// Uses agentic-flow/reasoningbank for MMR-ranked retrieval and embedding-based search
// Falls back to in-memory for environments without agentic-flow

import { randomUUID } from 'node:crypto';
import {
  initialize as initReasoningBank,
  retrieveMemories,
  db,
  computeEmbedding,
} from 'agentic-flow/reasoningbank';
import type { MemoryEntry, MemoryMetadata, RetrievalContext } from '../types/memory.js';

export interface FinancialMemory {
  store(content: string, metadata: MemoryMetadata): Promise<MemoryEntry>;
  search(query: string, limit?: number): Promise<RetrievalContext>;
  retrieve(entryId: string): Promise<MemoryEntry | null>;
  getByTicker(ticker: string, limit?: number): Promise<MemoryEntry[]>;
}

// AgentDB-backed implementation using agentic-flow ReasoningBank
export class AgentDbFinancialMemory implements FinancialMemory {
  private initialized = false;
  private domain: string;

  constructor(domain = 'cfa-analysis') {
    this.domain = domain;
  }

  private async ensureInit(): Promise<void> {
    if (this.initialized) return;
    await initReasoningBank();
    this.initialized = true;
  }

  async store(content: string, metadata: MemoryMetadata): Promise<MemoryEntry> {
    await this.ensureInit();

    const entryId = randomUUID();
    const tags = [
      metadata.sourceType,
      ...(metadata.tickers ?? []),
      ...(metadata.tags ?? []),
      metadata.sector,
      metadata.analysisType,
    ].filter((t): t is string => Boolean(t));

    // Store as a reasoning memory in agentdb
    db.upsertMemory({
      id: entryId,
      type: 'reasoning_memory',
      pattern_data: {
        title: metadata.analysisType ?? 'financial-analysis',
        description: tags.join(', '),
        content,
        source: {
          task_id: entryId,
          agent_id: metadata.sourceType ?? 'cfa-agent',
          outcome: 'Success',
          evidence: tags,
        },
        tags,
        domain: this.domain,
        created_at: new Date().toISOString(),
        confidence: 0.8,
        n_uses: 0,
      },
      confidence: 0.8,
      usage_count: 0,
    });

    // Generate and store embedding for vector search
    const embedding = await computeEmbedding(content);
    db.upsertEmbedding({
      id: entryId,
      model: 'all-MiniLM-L6-v2',
      dims: embedding.length,
      vector: embedding,
      created_at: new Date().toISOString(),
    });

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
    await this.ensureInit();

    // Use ReasoningBank's MMR-ranked retrieval (cosine + recency + reliability)
    const memories = await retrieveMemories(query, {
      k: limit,
      domain: this.domain,
    });

    const entries = memories.map((m) => ({
      entry: {
        entryId: m.id,
        archiveId: 'default',
        content: m.content,
        metadata: {
          sourceType: 'analysis' as const,
          tags: [],
        } as MemoryMetadata,
        embeddingModel: 'all-MiniLM-L6-v2',
        retentionTier: 'hot' as const,
        createdAt: new Date(),
        lastAccessedAt: new Date(),
        accessCount: 0,
      },
      similarityScore: m.score,
    }));

    return { entries, query, totalSearched: entries.length };
  }

  async retrieve(entryId: string): Promise<MemoryEntry | null> {
    await this.ensureInit();

    const allMemories = db.getAllActiveMemories();
    const found = allMemories.find(m => m.id === entryId);
    if (!found) return null;

    db.incrementUsage(entryId);

    return {
      entryId: found.id,
      archiveId: 'default',
      content: found.pattern_data.content,
      metadata: {
        sourceType: 'analysis',
        tags: found.pattern_data.tags,
        sector: found.pattern_data.domain,
      } as MemoryMetadata,
      embeddingModel: 'all-MiniLM-L6-v2',
      retentionTier: 'hot',
      createdAt: new Date(found.created_at),
      lastAccessedAt: new Date(),
      accessCount: found.usage_count,
    };
  }

  async getByTicker(ticker: string, limit = 10): Promise<MemoryEntry[]> {
    await this.ensureInit();

    // Search by ticker symbol using embedding similarity
    const memories = await retrieveMemories(ticker, {
      k: limit,
      domain: this.domain,
    });

    return memories.map((m) => ({
      entryId: m.id,
      archiveId: 'default',
      content: m.content,
      metadata: { sourceType: 'analysis' as const, tags: [], tickers: [ticker] } as MemoryMetadata,
      embeddingModel: 'all-MiniLM-L6-v2',
      retentionTier: 'hot' as const,
      createdAt: new Date(),
      lastAccessedAt: new Date(),
      accessCount: 0,
    }));
  }
}

// In-memory fallback (no agentic-flow dependency)
export class LocalFinancialMemory implements FinancialMemory {
  private entries = new Map<string, MemoryEntry>();

  async store(content: string, metadata: MemoryMetadata): Promise<MemoryEntry> {
    const entry: MemoryEntry = {
      entryId: randomUUID(),
      archiveId: 'default',
      content,
      metadata,
      embeddingModel: 'local-placeholder',
      retentionTier: 'hot',
      createdAt: new Date(),
      lastAccessedAt: new Date(),
      accessCount: 0,
    };
    this.entries.set(entry.entryId, entry);
    return entry;
  }

  async search(query: string, limit = 10): Promise<RetrievalContext> {
    const q = query.toLowerCase();
    const scored = [...this.entries.values()]
      .map(entry => {
        const content = entry.content.toLowerCase();
        const words = q.split(/\s+/);
        const matchCount = words.filter(w => content.includes(w)).length;
        return { entry, similarityScore: matchCount / Math.max(words.length, 1) };
      })
      .filter(r => r.similarityScore > 0)
      .sort((a, b) => b.similarityScore - a.similarityScore)
      .slice(0, limit);

    for (const { entry } of scored) {
      entry.lastAccessedAt = new Date();
      entry.accessCount++;
    }

    return { entries: scored, query, totalSearched: this.entries.size };
  }

  async retrieve(entryId: string): Promise<MemoryEntry | null> {
    const entry = this.entries.get(entryId);
    if (entry) {
      entry.lastAccessedAt = new Date();
      entry.accessCount++;
    }
    return entry ?? null;
  }

  async getByTicker(ticker: string, limit = 10): Promise<MemoryEntry[]> {
    return [...this.entries.values()]
      .filter(e => e.metadata.tickers?.includes(ticker))
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime())
      .slice(0, limit);
  }
}
