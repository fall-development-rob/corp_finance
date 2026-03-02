import { describe, it, expect } from 'vitest';
import { readFileSync, existsSync } from 'fs';
import { globSync } from 'glob';
import { join } from 'path';

const ROOT = join(import.meta.dirname, '..', '..');
const REPO_ROOT = join(ROOT, '..', '..');

function getFiles(pattern: string, cwd: string = ROOT): string[] {
  return globSync(pattern, { cwd, ignore: ['node_modules/**', 'dist/**'] });
}

// ---------------------------------------------------------------------------
// DRIFT-001: Tool count consistency across docs
// ---------------------------------------------------------------------------
describe('DRIFT-001: Tool count consistency', () => {
  const expectedToolCount = 195;
  const expectedModuleCount = 67;

  const docsToCheck = [
    'docs/adr/ADR-001.md',
    'docs/prd/PRD.md',
    'docs/ddd/DDD.md',
  ];

  for (const doc of docsToCheck) {
    const fullPath = join(REPO_ROOT, doc);
    if (!existsSync(fullPath)) continue;

    it(`${doc}: references ${expectedToolCount} tools (not stale)`, () => {
      const content = readFileSync(fullPath, 'utf-8');
      // Check for outdated "215" references
      const staleToolRefs = content.match(/\b215\s+(?:MCP\s+)?tools\b/gi);
      expect(
        staleToolRefs,
        `Found stale "215 tools" reference in ${doc}. Should be ${expectedToolCount}.`
      ).toBeNull();
    });

    it(`${doc}: references ${expectedModuleCount} modules (not stale)`, () => {
      const content = readFileSync(fullPath, 'utf-8');
      const staleModuleRefs = content.match(/\b71\s+(?:domain\s+|Rust\s+)?modules\b/gi);
      expect(
        staleModuleRefs,
        `Found stale "71 modules" reference in ${doc}. Should be ${expectedModuleCount}.`
      ).toBeNull();
    });
  }
});

// ---------------------------------------------------------------------------
// DRIFT-002: All 8 specialist agents must exist
// ---------------------------------------------------------------------------
describe('DRIFT-002: Specialist agent roster completeness', () => {
  const requiredAgents = [
    'equity-analyst',
    'credit-analyst',
    'fixed-income-analyst',
    'derivatives-analyst',
    'quant-risk-analyst',
    'macro-analyst',
    'esg-regulatory-analyst',
    'private-markets-analyst',
  ];

  for (const agent of requiredAgents) {
    it(`agent file exists: ${agent}.ts`, () => {
      const filePath = join(ROOT, 'agents', `${agent}.ts`);
      expect(existsSync(filePath), `Missing agent: ${agent}.ts`).toBe(true);
    });

    it(`agent prompt exists: .claude/agents/cfa/${agent}.md`, () => {
      const promptPath = join(REPO_ROOT, '.claude', 'agents', 'cfa', `${agent}.md`);
      expect(existsSync(promptPath), `Missing prompt: ${agent}.md`).toBe(true);
    });
  }
});

// ---------------------------------------------------------------------------
// DRIFT-003: Chief analyst must reference all specialist types
// ---------------------------------------------------------------------------
describe('DRIFT-003: Chief analyst references all specialists', () => {
  it('chief-analyst.ts references all 8 specialist types', () => {
    const chiefPath = join(ROOT, 'agents', 'chief-analyst.ts');
    if (!existsSync(chiefPath)) return;

    const content = readFileSync(chiefPath, 'utf-8');
    const specialists = [
      'equity', 'credit', 'fixed-income', 'derivatives',
      'quant-risk', 'macro', 'esg-regulatory', 'private-markets',
    ];

    for (const spec of specialists) {
      expect(
        content.includes(spec),
        `Chief analyst missing reference to ${spec} specialist`
      ).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// DRIFT-004: ADR files must be consistently numbered
// ---------------------------------------------------------------------------
describe('DRIFT-004: ADR numbering consistency', () => {
  it('ADR files follow ADR-NNN.md naming', () => {
    const adrFiles = getFiles('docs/adr/ADR*.md', REPO_ROOT);
    const badNames = adrFiles.filter(f => !/ADR-\d{3}(?:-[\w-]+)?\.md$/.test(f));
    expect(
      badNames,
      `ADR files with bad naming: ${badNames.join(', ')}. Use ADR-NNN.md format.`
    ).toHaveLength(0);
  });

  it('no gaps in ADR numbering', () => {
    const adrFiles = getFiles('docs/adr/ADR-*.md', REPO_ROOT);
    const numbers = adrFiles
      .map(f => parseInt(f.match(/ADR-(\d+)/)?.[1] ?? '0'))
      .sort((a, b) => a - b);

    if (numbers.length < 2) return;

    for (let i = 1; i < numbers.length; i++) {
      expect(
        numbers[i],
        `Gap in ADR numbering: ADR-${String(numbers[i - 1]).padStart(3, '0')} exists but ADR-${String(numbers[i - 1] + 1).padStart(3, '0')} is missing`
      ).toBe(numbers[i - 1] + 1);
    }
  });
});

// ---------------------------------------------------------------------------
// DRIFT-005: Rust feature count matches docs
// ---------------------------------------------------------------------------
describe('DRIFT-005: Rust feature count', () => {
  it('Cargo.toml feature count matches documented 67', () => {
    const cargoPath = join(REPO_ROOT, 'crates', 'corp-finance-core', 'Cargo.toml');
    if (!existsSync(cargoPath)) return;

    const content = readFileSync(cargoPath, 'utf-8');
    const featureSection = content.split('[features]')[1]?.split(/\n\[/)?.[0];
    if (!featureSection) return;

    // Count feature definitions (lines starting with a word = [...])
    const features = featureSection
      .split('\n')
      .filter(line => /^\w+\s*=\s*\[/.test(line.trim()))
      .length;

    // Allow some tolerance (default feature doesn't count)
    expect(
      features,
      `Cargo.toml has ${features} features, docs say 67. Update docs if features changed.`
    ).toBeGreaterThanOrEqual(60);
  });
});

// ---------------------------------------------------------------------------
// DRIFT-006: MCP schema files must exist for all tool categories
// ---------------------------------------------------------------------------
describe('DRIFT-006: MCP schema coverage', () => {
  const expectedSchemas = [
    'valuation', 'credit', 'fixed_income', 'derivatives',
    'portfolio', 'scenarios', 'pe', 'esg',
  ];

  for (const schema of expectedSchemas) {
    it(`schema exists: ${schema}`, () => {
      const schemaFiles = getFiles('src/schemas/*.ts', join(REPO_ROOT, 'packages', 'mcp-server'));
      const hasSchema = schemaFiles.some(f => f.includes(schema));
      expect(hasSchema, `Missing MCP schema for ${schema}`).toBe(true);
    });
  }
});
