// End-to-end test — runs the full Orchestrator pipeline with real MCP tools
// Requires: MCP server built, Rust bindings compiled
// Optional: ruvector-postgres on port 5433 for PG backend tests

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { Orchestrator } from '../orchestrator/coordinator.js';
import { McpBridge, createToolCaller } from '../bridge/mcp-client.js';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const mcpServerPath = join(__dirname, '..', '..', 'mcp-server', 'dist', 'index.js');

// Check if MCP server exists
let serverExists = false;
try {
  const { existsSync } = await import('node:fs');
  serverExists = existsSync(mcpServerPath);
} catch {
  serverExists = false;
}

describe.skipIf(!serverExists)('E2E Orchestrator — full pipeline', () => {
  let bridge: McpBridge;
  let callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  const events: Array<{ type: string; payload: unknown }> = [];

  beforeAll(async () => {
    const result = await createToolCaller({ serverPath: mcpServerPath });
    bridge = result.bridge;
    callTool = result.callTool;
  }, 30_000);

  afterAll(async () => {
    if (bridge?.isConnected) {
      await bridge.disconnect();
    }
  });

  it('connects to MCP server and lists tools', async () => {
    expect(bridge.isConnected).toBe(true);

    const tools = await bridge.listTools();
    expect(tools.length).toBeGreaterThan(0);
    console.log(`MCP server exposes ${tools.length} tools`);

    // Verify some key tools exist
    const toolNames = tools.map(t => t.name);
    expect(toolNames).toContain('wacc_calculator');
    expect(toolNames).toContain('dcf_model');
    expect(toolNames).toContain('credit_metrics');
  }, 15_000);

  it('calls a single tool directly (wacc_calculator)', async () => {
    const result = await callTool('wacc_calculator', {
      risk_free_rate: 0.04,
      equity_risk_premium: 0.06,
      beta: 1.2,
      cost_of_debt: 0.05,
      tax_rate: 0.21,
      debt_weight: 0.3,
      equity_weight: 0.7,
    });

    expect(result).toBeTruthy();
    console.log('WACC result:', JSON.stringify(result, null, 2));
  }, 15_000);

  it('runs full analysis: equity valuation query', async () => {
    events.length = 0;

    const orchestrator = new Orchestrator({
      callTool,
      confidenceThreshold: 0.3, // lower threshold for testing
      maxSpecialists: 3,
      onEvent: (e) => {
        events.push(e);
        console.log(`  [${e.type}]`, typeof e.payload === 'object'
          ? JSON.stringify(e.payload).slice(0, 120)
          : e.payload);
      },
    });

    console.log('\n--- Starting analysis ---');
    const { request, report, results } = await orchestrator.analyze(
      'Calculate the WACC and run a basic DCF for a company with beta 1.1, risk-free rate 4%, ERP 5.5%, cost of debt 4.5%, tax rate 21%, D/E ratio 0.4',
      'STANDARD',
    );

    console.log('\n--- Report ---');
    console.log(report);
    console.log('\n--- Summary ---');
    console.log(`  Status: ${request.status}`);
    console.log(`  Specialists used: ${results.length}`);
    console.log(`  Confidence: ${request.confidence?.value ?? 'N/A'}`);
    console.log(`  Events emitted: ${events.length}`);
    console.log(`  Tool calls: ${events.filter(e => e.type === 'ToolCalled').length}`);
    console.log(`  Tool successes: ${events.filter(e => e.type === 'ToolSucceeded').length}`);
    console.log(`  Tool failures: ${events.filter(e => e.type === 'ToolFailed').length}`);

    // Assertions
    expect(request.status).toBe('completed');
    expect(report).toBeTruthy();
    expect(report.length).toBeGreaterThan(50);
    expect(results.length).toBeGreaterThan(0);

    // At least one specialist should have completed
    for (const result of results) {
      expect(result.resultId).toBeTruthy();
      expect(result.agentType).toBeTruthy();
      expect(result.confidence).toBeGreaterThanOrEqual(0);
    }

    // Events should have been emitted
    expect(events.some(e => e.type === 'AnalysisRequested')).toBe(true);
    expect(events.some(e => e.type === 'PlanCreated')).toBe(true);
  }, 120_000); // 2 min timeout for full pipeline

  it('runs full analysis: credit assessment query', async () => {
    events.length = 0;

    const orchestrator = new Orchestrator({
      callTool,
      confidenceThreshold: 0.3,
      maxSpecialists: 3,
      onEvent: (e) => events.push(e),
    });

    const { request, report, results } = await orchestrator.analyze(
      'Assess the credit quality: current ratio 1.8, debt-to-equity 0.6, interest coverage 5x, EBITDA margin 22%',
      'STANDARD',
    );

    console.log(`\nCredit analysis: ${results.length} specialists, confidence ${request.confidence?.value}`);
    console.log(report.slice(0, 500));

    expect(request.status).toBe('completed');
    expect(results.length).toBeGreaterThan(0);
  }, 120_000);
});
