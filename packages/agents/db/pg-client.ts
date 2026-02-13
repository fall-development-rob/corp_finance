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
    await pool.query(sql);
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
 * Convert a Float32Array to a ruvector literal string: `[0.1,0.2,...]`
 */
export function float32ToVectorLiteral(vec: Float32Array): string {
  const parts: string[] = [];
  for (let i = 0; i < vec.length; i++) {
    parts.push(String(vec[i]));
  }
  return `[${parts.join(',')}]`;
}
