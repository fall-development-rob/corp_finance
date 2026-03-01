import { describe, it, expect } from 'vitest';
import { readFileSync, statSync } from 'fs';
import { globSync } from 'glob';
import { join } from 'path';

const ROOT = join(import.meta.dirname, '..', '..');

function readFile(path: string): string {
  return readFileSync(join(ROOT, path), 'utf-8');
}

function getFiles(pattern: string): string[] {
  return globSync(pattern, { cwd: ROOT, ignore: ['node_modules/**', 'dist/**'] });
}

// ---------------------------------------------------------------------------
// ARCH-001: Agents must never perform financial calculations directly
// ---------------------------------------------------------------------------
describe('ARCH-001: No direct financial calculations in agents', () => {
  const agentFiles = [
    ...getFiles('agents/*.ts'),
    ...getFiles('orchestrator/*.ts'),
  ];

  it('should find agent files to scan', () => {
    expect(agentFiles.length).toBeGreaterThan(0);
  });

  for (const file of agentFiles) {
    it(`${file}: no Math.pow/sqrt/log/exp`, () => {
      const content = readFile(file);
      const mathCalls = content.match(/Math\.(pow|sqrt|log|exp)\(/g);
      expect(mathCalls, `Found ${mathCalls?.join(', ')} in ${file}`).toBeNull();
    });
  }
});

// ---------------------------------------------------------------------------
// ARCH-002: No hardcoded API keys or secrets
// ---------------------------------------------------------------------------
describe('ARCH-002: No hardcoded secrets', () => {
  const sourceFiles = getFiles('**/*.ts');

  for (const file of sourceFiles) {
    it(`${file}: no hardcoded API keys`, () => {
      const content = readFile(file);
      // Skip test files and type definitions
      if (file.includes('.test.') || file.includes('.d.ts')) return;

      const apiKeyPattern = /['"]sk[-_][a-zA-Z0-9]{20,}['"]/;
      expect(
        apiKeyPattern.test(content),
        `Hardcoded API key found in ${file}`
      ).toBe(false);
    });

    it(`${file}: no hardcoded long hex secrets`, () => {
      const content = readFile(file);
      if (file.includes('.test.') || file.includes('.d.ts')) return;

      // Match quoted hex strings 32+ chars that aren't obviously UUIDs or hashes in comments
      const lines = content.split('\n');
      const violations: string[] = [];
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        // Skip comments and imports
        if (line.trim().startsWith('//') || line.trim().startsWith('*') || line.includes('import')) continue;
        if (/apiKey\s*[:=]\s*['"][a-f0-9]{32,}['"]/.test(line)) {
          violations.push(`Line ${i + 1}: ${line.trim()}`);
        }
      }
      expect(violations, `Hardcoded secrets:\n${violations.join('\n')}`).toHaveLength(0);
    });
  }
});

// ---------------------------------------------------------------------------
// ARCH-003: Agent files must not exceed 500 lines
// ---------------------------------------------------------------------------
describe('ARCH-003: File size limits (500 lines)', () => {
  const tsFiles = getFiles('**/*.ts');

  for (const file of tsFiles) {
    it(`${file}: under 500 lines`, () => {
      const content = readFile(file);
      const lineCount = content.split('\n').length;
      expect(
        lineCount,
        `${file} has ${lineCount} lines (max 500)`
      ).toBeLessThanOrEqual(500);
    });
  }
});

// ---------------------------------------------------------------------------
// ARCH-004: No direct database writes from agent code
// ---------------------------------------------------------------------------
describe('ARCH-004: No direct DB writes from agents', () => {
  const agentFiles = getFiles('agents/*.ts');

  for (const file of agentFiles) {
    it(`${file}: no INSERT/UPDATE/DELETE queries`, () => {
      const content = readFile(file);
      const dbWrites = content.match(/\.(query|execute)\s*\(\s*['"`](?:INSERT|UPDATE|DELETE)/gi);
      expect(
        dbWrites,
        `Direct DB writes found in ${file}: ${dbWrites?.join(', ')}`
      ).toBeNull();
    });
  }
});

// ---------------------------------------------------------------------------
// ARCH-006: No .env files in repo
// ---------------------------------------------------------------------------
describe('ARCH-006: No .env files committed', () => {
  it('should not have .env files in packages/', () => {
    const envFiles = getFiles('**/.env').filter(
      f => !f.includes('node_modules') && !f.includes('.gitignore')
    );
    // docker/.env is gitignored, so filter only tracked files
    const tracked = envFiles.filter(f => !f.includes('docker/'));
    expect(tracked, `Found .env files: ${tracked.join(', ')}`).toHaveLength(0);
  });
});
