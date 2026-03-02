// Contract-level smoke tests — validates MCP tool registration invariants
// Enforces: MCP-SMOKE-001 through MCP-SMOKE-006
// Static analysis only: reads source files and validates via regex, no runtime imports

import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, existsSync } from 'fs';
import { join } from 'path';

const repoRoot = join(__dirname, '..', '..', '..', '..');
const mcpServerToolsDir = join(repoRoot, 'packages', 'mcp-server', 'src', 'tools');
const mcpServerSchemasDir = join(repoRoot, 'packages', 'mcp-server', 'src', 'schemas');
const dataMcpServerSrc = join(repoRoot, 'packages', 'data-mcp-server', 'src');
const vendorMcpServerSrc = join(repoRoot, 'packages', 'vendor-mcp-server', 'src');

/** Recursively collect all .ts files under a directory */
function collectTsFiles(dir: string): string[] {
  const results: string[] = [];
  if (!existsSync(dir)) return results;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...collectTsFiles(full));
    } else if (entry.name.endsWith('.ts') && !entry.name.endsWith('.d.ts')) {
      results.push(full);
    }
  }
  return results;
}

/** Extract all server.tool("name", ...) first-argument strings from source files */
function extractToolNames(files: string[]): string[] {
  const names: string[] = [];
  const toolCallRegex = /server\.tool\(\s*['"]([^'"]+)['"]/g;
  for (const file of files) {
    const content = readFileSync(file, 'utf-8');
    let match: RegExpExecArray | null;
    while ((match = toolCallRegex.exec(content)) !== null) {
      names.push(match[1]);
    }
  }
  return names;
}

// ── Pre-compute tool lists for all three servers ──

const mcpToolFiles = readdirSync(mcpServerToolsDir)
  .filter(f => f.endsWith('.ts'))
  .map(f => join(mcpServerToolsDir, f));
const mcpToolNames = extractToolNames(mcpToolFiles);

const dataToolFiles = collectTsFiles(dataMcpServerSrc).filter(f => !f.includes('/schemas/') && !f.includes('/client'));
const dataToolNames = extractToolNames(dataToolFiles);

const vendorToolFiles = collectTsFiles(vendorMcpServerSrc).filter(f => !f.includes('/schemas/') && !f.includes('/client'));
const vendorToolNames = extractToolNames(vendorToolFiles);

// ── MCP-SMOKE-001: Tool registration count (mcp-server >= 195) ──

describe('MCP-SMOKE-001: Corp-finance MCP server tool count', () => {
  it('registers at least 195 tools across all tool files', () => {
    expect(
      mcpToolNames.length,
      `Expected >=195 tools, found ${mcpToolNames.length}: ${mcpToolNames.join(', ')}`,
    ).toBeGreaterThanOrEqual(195);
  });
});

// ── MCP-SMOKE-002: Tool naming convention (snake_case only) ──

describe('MCP-SMOKE-002: Tool naming convention', () => {
  it('all mcp-server tool names follow snake_case', () => {
    const violators = mcpToolNames.filter(name => {
      // snake_case: lowercase letters, digits, underscores only
      return !/^[a-z][a-z0-9_]*$/.test(name);
    });
    expect(violators, `Non-snake_case tool names: ${violators.join(', ')}`).toEqual([]);
  });

  it('all data-mcp-server tool names follow snake_case', () => {
    const violators = dataToolNames.filter(name => !/^[a-z][a-z0-9_]*$/.test(name));
    expect(violators, `Non-snake_case tool names: ${violators.join(', ')}`).toEqual([]);
  });

  it('all vendor-mcp-server tool names follow snake_case', () => {
    const violators = vendorToolNames.filter(name => !/^[a-z][a-z0-9_]*$/.test(name));
    expect(violators, `Non-snake_case tool names: ${violators.join(', ')}`).toEqual([]);
  });
});

// ── MCP-SMOKE-003: Data server tool count (>= 75) ──

describe('MCP-SMOKE-003: Data MCP server tool count', () => {
  it('registers at least 75 tools', () => {
    expect(
      dataToolNames.length,
      `Expected >=75 tools, found ${dataToolNames.length}`,
    ).toBeGreaterThanOrEqual(75);
  });
});

// ── MCP-SMOKE-004: Vendor server tool count (>= 87) ──

describe('MCP-SMOKE-004: Vendor MCP server tool count', () => {
  it('registers at least 87 tools', () => {
    expect(
      vendorToolNames.length,
      `Expected >=87 tools, found ${vendorToolNames.length}`,
    ).toBeGreaterThanOrEqual(87);
  });
});

// ── MCP-SMOKE-005: No duplicate tool names within each server ──

describe('MCP-SMOKE-005: No duplicate tool names', () => {
  function findDuplicates(names: string[]): string[] {
    const seen = new Set<string>();
    const dupes: string[] = [];
    for (const name of names) {
      if (seen.has(name)) dupes.push(name);
      seen.add(name);
    }
    return dupes;
  }

  it('mcp-server has no duplicate tool names', () => {
    const dupes = findDuplicates(mcpToolNames);
    expect(dupes, `Duplicate tool names: ${dupes.join(', ')}`).toEqual([]);
  });

  it('data-mcp-server has no duplicate tool names', () => {
    const dupes = findDuplicates(dataToolNames);
    expect(dupes, `Duplicate tool names: ${dupes.join(', ')}`).toEqual([]);
  });

  it('vendor-mcp-server has no duplicate tool names', () => {
    const dupes = findDuplicates(vendorToolNames);
    expect(dupes, `Duplicate tool names: ${dupes.join(', ')}`).toEqual([]);
  });
});

// ── MCP-SMOKE-006: Zod schema validation in mcp-server ──

describe('MCP-SMOKE-006: Zod schema validation', () => {
  const toolFiles = readdirSync(mcpServerToolsDir)
    .filter(f => f.endsWith('.ts'))
    .map(f => f.replace('.ts', ''));

  it('every tool file imports from a corresponding schema module', () => {
    const missing: string[] = [];
    for (const file of toolFiles) {
      const content = readFileSync(join(mcpServerToolsDir, `${file}.ts`), 'utf-8');
      // Each tool file should import from ../schemas/
      if (!content.includes('../schemas/')) {
        missing.push(file);
      }
    }
    expect(missing, `Tool files missing schema imports: ${missing.join(', ')}`).toEqual([]);
  });

  it('every tool registration uses a Schema.shape parameter', () => {
    const filesWithout: string[] = [];
    for (const file of toolFiles) {
      const filePath = join(mcpServerToolsDir, `${file}.ts`);
      const content = readFileSync(filePath, 'utf-8');
      // Count server.tool( calls
      const toolCalls = (content.match(/server\.tool\(/g) || []).length;
      // Count .shape references (each tool passes Schema.shape as its zod schema)
      const shapeRefs = (content.match(/\.shape/g) || []).length;
      if (toolCalls > 0 && shapeRefs < toolCalls) {
        filesWithout.push(`${file} (${toolCalls} tools, ${shapeRefs} .shape refs)`);
      }
    }
    expect(filesWithout, `Files missing .shape for some tools: ${filesWithout.join('; ')}`).toEqual([]);
  });

  it('mcp-server schemas directory has z.object definitions', () => {
    const schemaFiles = readdirSync(mcpServerSchemasDir).filter(f => f.endsWith('.ts'));
    expect(schemaFiles.length).toBeGreaterThan(0);

    let schemasWithZodObject = 0;
    for (const file of schemaFiles) {
      const content = readFileSync(join(mcpServerSchemasDir, file), 'utf-8');
      if (content.includes('z.object(')) {
        schemasWithZodObject++;
      }
    }
    expect(
      schemasWithZodObject,
      'Expected most schema files to contain z.object definitions',
    ).toBeGreaterThanOrEqual(schemaFiles.length - 1); // allow 1 common.ts without z.object
  });
});
