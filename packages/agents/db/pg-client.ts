// PG client — connection pool singleton for ruvector-postgres
// Provides pool management, health check, migration runner, and vector helpers

import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// pg is dynamically imported so it's only loaded when postgres backend is selected
let _pool: import('pg').Pool | null = null;
let _pg: typeof import('pg') | null = null;

async function loadPg(): Promise<typeof import('pg')> {
  if (!_pg) {
    _pg = await import('pg');
  }
  return _pg;
}

export interface PgConfig {
  host: string;
  port: number;
  user: string;
  password: string;
  database: string;
  poolMin?: number;
  poolMax?: number;
  idleTimeoutMs?: number;
  connectionTimeoutMs?: number;
}

function configFromEnv(): PgConfig {
  return {
    host: process.env.PG_HOST ?? 'localhost',
    port: Number(process.env.PG_PORT ?? 5433),
    user: process.env.PG_USER ?? 'cfa',
    password: process.env.PG_PASSWORD ?? 'cfa_dev_pass',
    database: process.env.PG_DATABASE ?? 'cfa_agents',
    poolMin: Number(process.env.PG_POOL_MIN ?? 2),
    poolMax: Number(process.env.PG_POOL_MAX ?? 10),
    idleTimeoutMs: Number(process.env.PG_IDLE_TIMEOUT_MS ?? 30_000),
    connectionTimeoutMs: Number(process.env.PG_CONNECTION_TIMEOUT_MS ?? 5_000),
  };
}

/**
 * Returns the shared pg.Pool singleton, creating it on first call.
 */
export async function getPool(config?: PgConfig): Promise<import('pg').Pool> {
  if (_pool) return _pool;

  const pg = await loadPg();
  const c = config ?? configFromEnv();

  _pool = new pg.default.Pool({
    host: c.host,
    port: c.port,
    user: c.user,
    password: c.password,
    database: c.database,
    min: c.poolMin,
    max: c.poolMax,
    idleTimeoutMillis: c.idleTimeoutMs,
    connectionTimeoutMillis: c.connectionTimeoutMs,
    statement_timeout: 30_000,
    application_name: 'cfa-agents',
  });

  // Resilience: never crash the process on idle-client errors
  _pool.on('error', (err) => {
    console.warn('[pg-client] pool background error, resetting pool:', err.message);
    resetPool();
  });

  // Set ruvector ef_search on every new connection for improved recall
  _pool.on('connect', (client) => {
    client.query('SET ruvector.ef_search = 100').catch((err: Error) => {
      console.warn('[pg-client] failed to SET ruvector.ef_search:', err.message);
    });
  });

  return _pool;
}

/**
 * Verify database connectivity. Returns true if the pool can reach the database.
 */
export async function healthCheck(): Promise<boolean> {
  try {
    const pool = await getPool();
    const result = await pool.query('SELECT 1 AS ok');
    return result.rows[0]?.ok === 1;
  } catch {
    return false;
  }
}

/**
 * Run all pending SQL migrations from db/migrations/ in version order.
 * Each migration is wrapped in a transaction with its version recording.
 */
export async function runMigrations(): Promise<string[]> {
  const pool = await getPool();

  // Ensure schema_migrations table exists
  await pool.query(`
    CREATE TABLE IF NOT EXISTS schema_migrations (
      version TEXT PRIMARY KEY,
      applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
    )
  `);

  // Check which migrations have already been applied
  const { rows: applied } = await pool.query<{ version: string }>(
    'SELECT version FROM schema_migrations ORDER BY version',
  );
  const appliedSet = new Set(applied.map(r => r.version));

  // Read migration files
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const migrationsDir = join(__dirname, 'migrations');

  let migrationFiles: string[];
  try {
    const { readdirSync } = await import('node:fs');
    migrationFiles = readdirSync(migrationsDir)
      .filter(f => f.endsWith('.sql'))
      .sort();
  } catch {
    return []; // No migrations directory
  }

  const ran: string[] = [];
  for (const file of migrationFiles) {
    const version = file.replace('.sql', '');
    if (appliedSet.has(version)) continue;

    const sql = readFileSync(join(migrationsDir, file), 'utf-8');

    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      await client.query(sql);
      await client.query(
        'INSERT INTO schema_migrations (version) VALUES ($1)',
        [version],
      );
      await client.query('COMMIT');
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }

    ran.push(version);
  }

  return ran;
}

/**
 * Close the pool and release all connections.
 */
export async function closePool(): Promise<void> {
  if (_pool) {
    await _pool.end();
    _pool = null;
  }
}

/**
 * Reset the pool — used after ruvector segfault recovery.
 * Silently ends the existing pool and nulls it so getPool() creates a fresh one.
 */
export async function resetPool(): Promise<void> {
  if (_pool) {
    try { await _pool.end(); } catch { /* pool already broken */ }
    _pool = null;
  }
}

/**
 * Execute a query with automatic retry on connection/recovery errors.
 * Handles ruvector HNSW segfault → Postgres recovery → retry pattern.
 * Uses exponential backoff: base delay * 3^attempt (1s → 3s → 9s by default).
 */
export async function queryWithRetry<T extends import('pg').QueryResultRow>(
  queryText: string,
  params: unknown[],
  maxRetries = Number(process.env.PG_RETRY_MAX ?? 2),
  retryDelayMs = Number(process.env.PG_RETRY_DELAY_MS ?? 1000),
): Promise<import('pg').QueryResult<T>> {
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const pool = await getPool();
      return await pool.query<T>(queryText, params);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      const isRecoverable =
        msg.includes('Connection terminated') ||
        msg.includes('recovery mode') ||
        msg.includes('the database system is starting up') ||
        msg.includes('connection refused') ||
        msg.includes('terminating connection');

      if (attempt < maxRetries && isRecoverable) {
        const delay = retryDelayMs * Math.pow(3, attempt);
        console.warn(
          `[pg-client] queryWithRetry attempt ${attempt + 1}/${maxRetries} failed: ${msg}. ` +
          `Retrying in ${delay}ms...`,
        );
        await resetPool();
        await new Promise(r => setTimeout(r, delay));
        continue;
      }

      console.error(
        `[pg-client] queryWithRetry failed after ${attempt + 1} attempt(s): ${msg}`,
      );
      throw err;
    }
  }
  throw new Error('queryWithRetry: exhausted retries');
}

/**
 * Convert a Float32Array to a ruvector literal string: `[0.1,0.2,...]`
 * Uses toFixed(6) to reduce literal size while preserving sufficient precision.
 */
export function float32ToVectorLiteral(vec: Float32Array): string {
  const parts: string[] = [];
  for (let i = 0; i < vec.length; i++) {
    parts.push(vec[i].toFixed(6));
  }
  return `[${parts.join(',')}]`;
}
