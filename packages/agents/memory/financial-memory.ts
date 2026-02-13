// Financial Memory — persistent analysis storage via agentic-flow agentdb
// Uses claude-flow memory CLI for HNSW-indexed vector search
// Falls back to in-memory for environments without claude-flow

import { randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import type { MemoryEntry, MemoryMetadata, RetrievalContext } from '../types/memory.js';

const execFileAsync = promisify(execFile);

export interface FinancialMemory {
  store(content: string, metadata: MemoryMetadata): Promise<MemoryEntry>;
  search(query: string, limit?: number): Promise<RetrievalContext>;
  retrieve(entryId: string): Promise<MemoryEntry | null>;
  getByTicker(ticker: string, limit?: number): Promise<MemoryEntry[]>;
}

// Execute a claude-flow memory command
async function cfMemoryCmd(
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

// AgentDB-backed implementation using claude-flow memory
export class AgentDbFinancialMemory implements FinancialMemory {
  private namespace: string;

  constructor(namespace = 'cfa-memory') {
    this.namespace = namespace;
  }

  async store(content: string, metadata: MemoryMetadata): Promise<MemoryEntry> {
    const entry: MemoryEntry = {
      entryId: randomUUID(),
      archiveId: 'default',
      content,
      metadata,
      embeddingModel: 'all-MiniLM-L6-v2',
      retentionTier: 'hot',
      createdAt: new Date(),
      lastAccessedAt: new Date(),
      accessCount: 0,
    };

    // Store in agentdb with HNSW indexing
    const tags = [
      metadata.sourceType,
      ...(metadata.tickers ?? []),
      ...(metadata.tags ?? []),
      metadata.sector,
      metadata.analysisType,
    ].filter(Boolean).join(',');

    await cfMemoryCmd('store', {
      key: `analysis/${entry.entryId}`,
      value: JSON.stringify({ content, metadata, entryId: entry.entryId }),
      namespace: this.namespace,
      ...(tags ? { tags } : {}),
    });

    return entry;
  }

  async search(query: string, limit = 10): Promise<RetrievalContext> {
    // Use agentdb HNSW vector search
    const result = await cfMemoryCmd('search', {
      query,
      namespace: this.namespace,
      limit: String(limit),
    });

    if (!result) {
      return { entries: [], query, totalSearched: 0 };
    }

    // Parse search results
    try {
      const parsed = JSON.parse(result);
      const entries = Array.isArray(parsed)
        ? parsed.map((r: any) => ({
            entry: this.parseEntry(r),
            similarityScore: r.score ?? r.similarity ?? 0.5,
          }))
        : [];
      return { entries, query, totalSearched: entries.length };
    } catch {
      return { entries: [], query, totalSearched: 0 };
    }
  }

  async retrieve(entryId: string): Promise<MemoryEntry | null> {
    const result = await cfMemoryCmd('retrieve', {
      key: `analysis/${entryId}`,
      namespace: this.namespace,
    });

    if (!result) return null;

    try {
      const parsed = JSON.parse(result);
      return this.parseEntry(parsed);
    } catch {
      return null;
    }
  }

  async getByTicker(ticker: string, limit = 10): Promise<MemoryEntry[]> {
    const result = await cfMemoryCmd('search', {
      query: ticker,
      namespace: this.namespace,
      limit: String(limit),
    });

    if (!result) return [];

    try {
      const parsed = JSON.parse(result);
      return Array.isArray(parsed) ? parsed.map((r: any) => this.parseEntry(r)) : [];
    } catch {
      return [];
    }
  }

  private parseEntry(raw: any): MemoryEntry {
    return {
      entryId: raw.entryId ?? raw.key ?? randomUUID(),
      archiveId: 'default',
      content: raw.content ?? raw.value ?? '',
      metadata: raw.metadata ?? { sourceType: 'analysis', tags: [] },
      embeddingModel: 'all-MiniLM-L6-v2',
      retentionTier: 'hot',
      createdAt: new Date(raw.createdAt ?? Date.now()),
      lastAccessedAt: new Date(),
      accessCount: raw.accessCount ?? 0,
    };
  }
}

// In-memory fallback (no agentdb dependency)
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
