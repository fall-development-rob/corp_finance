// Database factory — selects memory/learning backend based on CFA_MEMORY_BACKEND env var
// Supported values: 'sqlite' (default), 'postgres', 'local'

import type { FinancialMemory } from '../memory/financial-memory.js';
import type { ReasoningBank } from '../learning/reasoning-bank.js';

export type MemoryBackend = 'sqlite' | 'postgres' | 'local';

function getBackend(): MemoryBackend {
  const env = process.env.CFA_MEMORY_BACKEND?.toLowerCase();
  if (env === 'postgres' || env === 'pg') return 'postgres';
  if (env === 'local') return 'local';
  return 'sqlite';
}

/**
 * Create a FinancialMemory instance based on the configured backend.
 * - `sqlite`: AgentDbFinancialMemory (agentic-flow SQLite)
 * - `postgres`: PgFinancialMemory (ruvector-postgres)
 * - `local`: LocalFinancialMemory (in-memory)
 */
export async function createFinancialMemory(
  domain = 'cfa-analysis',
): Promise<FinancialMemory> {
  const backend = getBackend();

  switch (backend) {
    case 'postgres': {
      const { PgFinancialMemory } = await import('../memory/pg-financial-memory.js');
      return new PgFinancialMemory(domain);
    }
    case 'local': {
      const { LocalFinancialMemory } = await import('../memory/financial-memory.js');
      return new LocalFinancialMemory();
    }
    case 'sqlite':
    default: {
      const { AgentDbFinancialMemory } = await import('../memory/financial-memory.js');
      return new AgentDbFinancialMemory(domain);
    }
  }
}

/**
 * Create a ReasoningBank instance based on the configured backend.
 * - `sqlite`: SonaReasoningBank (agentic-flow SQLite)
 * - `postgres`: PgReasoningBank (ruvector-postgres)
 * - `local`: LocalReasoningBank (in-memory)
 */
export async function createReasoningBank(): Promise<ReasoningBank> {
  const backend = getBackend();

  switch (backend) {
    case 'postgres': {
      const { PgReasoningBank } = await import('../learning/pg-reasoning-bank.js');
      return new PgReasoningBank();
    }
    case 'local': {
      const { LocalReasoningBank } = await import('../learning/reasoning-bank.js');
      return new LocalReasoningBank();
    }
    case 'sqlite':
    default: {
      const { SonaReasoningBank } = await import('../learning/reasoning-bank.js');
      return new SonaReasoningBank();
    }
  }
}
