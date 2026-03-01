#!/usr/bin/env node
// CFA Agent Analyst — centralized CLI
//
// Usage:
//   cfa analyze "Calculate WACC for beta 1.2, risk-free rate 4%"   # pipeline (multi-agent)
//   cfa analyze --agent cfa-equity-analyst "Calculate WACC"         # single-agent
//   cfa analyze --topology mesh "Credit assessment: D/E 0.6"       # custom topology
//   cfa analyze -i                                                  # interactive REPL
//   cfa list                                                        # list specialist agents
//   cfa tools                                                       # list MCP tools
//   cfa --help                                                      # usage

import 'dotenv/config';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { createRequire } from 'node:module';
import { createToolCaller } from '../bridge/mcp-client.js';
import { FmpBridge, createFmpToolCaller } from '../bridge/fmp-bridge.js';
import { CfaPipeline, injectSkills, type Topology } from './pipeline.js';
import type { McpBridge } from '../bridge/mcp-client.js';
import { BatchAnalyzer } from '../orchestrator/batch-analyzer.js';
import { c, startRepl, printHelp, printAnalyzeHelp } from './cli-repl.js';

// Resolve agentic-flow deep imports via file path (bypasses exports map)
const _require = createRequire(import.meta.url);
const _afDir = dirname(_require.resolve('agentic-flow/package.json'));
const { claudeAgent } = await import(pathToFileURL(join(_afDir, 'dist', 'agents', 'claudeAgent.js')).href) as typeof import('agentic-flow/dist/agents/claudeAgent.js');
const { getAgent, listAgents } = await import(pathToFileURL(join(_afDir, 'dist', 'utils', 'agentLoader.js')).href) as typeof import('agentic-flow/dist/utils/agentLoader.js');

// ── Agent & skill directories ───────────────────────────────────────
const __cliDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__cliDir, '..', '..', '..', '..');
const cfaAgentsDir = join(repoRoot, '.claude', 'agents', 'cfa');
const skillsDir = join(repoRoot, '.claude', 'skills');

// ── Valid topologies ────────────────────────────────────────────────

const VALID_TOPOLOGIES: Topology[] = ['mesh', 'hierarchical', 'ring', 'star'];

function isValidTopology(value: string): value is Topology {
  return VALID_TOPOLOGIES.includes(value as Topology);
}

// ── MCP config helper ───────────────────────────────────────────────

function ensureMcpConfig(mcpServerPath: string): void {
  const configDir = join(homedir(), '.agentic-flow');
  const configPath = join(configDir, 'mcp-config.json');

  let config: { servers: Record<string, unknown> } = { servers: {} };
  if (existsSync(configPath)) {
    config = JSON.parse(readFileSync(configPath, 'utf-8'));
  } else {
    mkdirSync(configDir, { recursive: true });
  }

  config.servers['cfa-tools'] = {
    enabled: true,
    command: 'node',
    args: [mcpServerPath],
    env: {},
  };

  if (process.env.FMP_API_KEY) {
    const fmpServerPath = join(dirname(mcpServerPath), '..', '..', 'fmp-mcp-server', 'dist', 'index.js');
    const fmpEnv: Record<string, string> = {
      FMP_API_KEY: process.env.FMP_API_KEY!,
    };
    if (process.env.FMP_BASE_URL) fmpEnv.FMP_BASE_URL = process.env.FMP_BASE_URL;
    config.servers['fmp-market-data'] = {
      enabled: true,
      command: 'node',
      args: [fmpServerPath],
      env: fmpEnv,
    };
  }

  writeFileSync(configPath, JSON.stringify(config, null, 2));
}

// ── Agent loader helper ─────────────────────────────────────────────

const DEFAULT_AGENT = 'cfa-chief-analyst';

function loadAgent(name: string) {
  const fromCfa = getAgent(name, cfaAgentsDir);
  if (fromCfa) return fromCfa;

  const fromCwd = getAgent(name);
  if (fromCwd) return fromCwd;

  const available = listAllAgents().map(a => a.name);
  throw new Error(
    `Agent "${name}" not found.\n  Available: ${available.join(', ') || '(none)'}`,
  );
}

function listAllAgents() {
  return listAgents(cfaAgentsDir);
}

// ── Environment setup ───────────────────────────────────────────────

function setupEnv(): void {
  process.env.COMPLETION_MODEL = process.env.CFA_MODEL ?? 'claude-haiku-4-5-20251001';

  const mcpServerPath = join(__cliDir, '..', '..', '..', 'mcp-server', 'dist', 'index.js');
  ensureMcpConfig(mcpServerPath);

  process.env.ENABLE_CLAUDE_FLOW_MCP = 'false';
  process.env.ENABLE_FLOW_NEXUS_MCP = 'false';
  process.env.ENABLE_AGENTIC_PAYMENTS_MCP = 'false';
  process.env.ENABLE_CLAUDE_FLOW_SDK = 'false';
}

// ── CLI class ───────────────────────────────────────────────────────

class CfaCli {
  private bridge: McpBridge | null = null;
  private fmpBridge: FmpBridge | null = null;

  async start(): Promise<void> {
    const rawArgs = process.argv.slice(2);

    if (rawArgs.includes('--help') || rawArgs.includes('-h') || rawArgs.length === 0) {
      printHelp();
      return;
    }

    const command = rawArgs[0];
    const rest = rawArgs.slice(1);

    switch (command) {
      case 'analyze':
        await this.handleAnalyze(rest);
        break;
      case 'list':
        this.listAgents();
        break;
      case 'tools':
        await this.listTools();
        break;
      case 'help':
        printHelp();
        break;
      default:
        console.error(`Unknown command: ${command}\n`);
        printHelp();
        process.exit(1);
    }
  }

  // ── Subcommand: analyze ─────────────────────────────────────────

  private async handleAnalyze(args: string[]): Promise<void> {
    let interactive = false;
    let agentName: string | undefined;
    let batchCompanies: string | undefined;
    let topology: Topology = 'hierarchical';
    const queryParts: string[] = [];

    for (let i = 0; i < args.length; i++) {
      const arg = args[i];
      if (arg === '-i' || arg === '--interactive') {
        interactive = true;
      } else if (arg === '--agent' && args[i + 1]) {
        agentName = args[++i];
      } else if (arg === '--batch' && args[i + 1]) {
        batchCompanies = args[++i];
      } else if (arg === '--topology' && args[i + 1]) {
        const val = args[++i];
        if (!isValidTopology(val)) {
          console.error(`  ${c('red', 'Error:')} Invalid topology "${val}". Valid: ${VALID_TOPOLOGIES.join(', ')}\n`);
          process.exit(1);
        }
        topology = val;
      } else if (arg === '--max-turns' && args[i + 1]) {
        process.env.MAX_TURNS = args[++i];
      } else if (arg === '--help' || arg === '-h') {
        printAnalyzeHelp();
        return;
      } else {
        queryParts.push(arg);
      }
    }

    if (!process.env.ANTHROPIC_API_KEY) {
      console.error(`  ${c('red', 'Error:')} ANTHROPIC_API_KEY environment variable is required.\n`);
      console.error(`  Set it with: export ANTHROPIC_API_KEY=your-key-here\n`);
      process.exit(1);
    }

    if (interactive) {
      setupEnv();
      await startRepl({
        agentName,
        initialTopology: topology,
        model: process.env.COMPLETION_MODEL!,
        loadAgent,
        defaultAgent: DEFAULT_AGENT,
        skillsDir,
        connectFn: () => this.connect(),
        listAgentsFn: () => this.listAgents(),
        listToolsFn: () => this.listToolsInner(),
        bridge: this.bridge,
        fmpBridge: this.fmpBridge,
      });
    } else {
      const query = queryParts.join(' ').trim();
      if (!query) {
        console.error('Error: No query provided. Use "cfa analyze --help" for usage.\n');
        process.exit(1);
      }

      if (batchCompanies) {
        await this.runBatch(batchCompanies, query);
      } else if (agentName) {
        await this.runSingleAgent(query, agentName);
      } else {
        await this.runPipeline(query, topology);
      }
    }
  }

  // ── Pipeline mode (multi-agent) ────────────────────────────────

  private async runPipeline(userQuery: string, topology: Topology): Promise<void> {
    setupEnv();

    const model = process.env.COMPLETION_MODEL!;
    console.log(`\n  ${c('bold', 'CFA Agent Analyst')} ${c('dim', '— multi-agent pipeline')}`);
    console.log(`  ${c('dim', `Model: ${model} | Topology: ${topology}`)}\n`);

    const startTime = Date.now();

    const pipeline = new CfaPipeline({
      topology,
      onStatus: (stage, message) => {
        process.stderr.write(`  ${c('magenta', `[${stage}]`)} ${c('dim', message)}\n`);
      },
    });

    let streamed = false;
    const result = await pipeline.execute(
      userQuery,
      (chunk: string) => {
        streamed = true;
        process.stdout.write(chunk);
      },
    );

    const duration = ((Date.now() - startTime) / 1000).toFixed(1);

    if (!streamed && result.synthesis) {
      console.log(result.synthesis);
    }

    console.log(`\n  ${c('green', '✓')} ${c('bold', 'Complete')} ${c('dim', `— ${duration}s`)}`);
    console.log(`  ${c('dim', `Agents: ${result.routedAgents.join(', ')}`)}`);
    console.log(`  ${c('dim', `Timings: routing ${result.timings.routingMs}ms | memory ${result.timings.memoryMs}ms | agents ${result.timings.agentsMs}ms | coordination ${result.timings.coordinationMs}ms | synthesis ${result.timings.synthesisMs}ms`)}`);
    if (result.coordination) {
      console.log(`  ${c('dim', `Coordination: ${result.coordination.mechanism} | top: ${result.coordination.topAgents.join(', ')}`)}`);
    }
    console.log();

    process.exit(0);
  }

  // ── Batch mode (ADR-006: multi-company portfolio analysis) ─────

  private async runBatch(companiesArg: string, query: string): Promise<void> {
    setupEnv();

    let companies: string[];
    if (companiesArg.startsWith('@')) {
      const filePath = companiesArg.slice(1);
      const { readFileSync: readFile } = await import('node:fs');
      companies = readFile(filePath, 'utf-8')
        .split('\n')
        .map(l => l.trim())
        .filter(Boolean);
    } else {
      companies = companiesArg.split(',').map(c => c.trim()).filter(Boolean);
    }

    if (companies.length === 0) {
      console.error(`  ${c('red', 'Error:')} No companies specified.\n`);
      process.exit(1);
    }

    console.log(`\n  ${c('bold', 'CFA Agent Analyst')} ${c('dim', '— batch portfolio analysis')}`);
    console.log(`  ${c('dim', `Companies: ${companies.join(', ')} | Query: ${query}`)}\n`);

    const startTime = Date.now();
    const { callTool, callFmpTool } = await this.connect();
    const analyzer = new BatchAnalyzer({ callTool, callFmpTool });

    const result = await analyzer.analyze(companies, query, {
      concurrency: 3,
      onProgress: (progress) => {
        const pct = Math.round((progress.completed / progress.total) * 100);
        const statusIcon = progress.status === 'completed' ? c('green', '✓')
          : progress.status === 'failed' ? c('red', '✗')
          : c('yellow', '⟳');
        process.stderr.write(`  ${statusIcon} [${pct}%] ${progress.current} — ${progress.status}\n`);
      },
    });

    const duration = ((Date.now() - startTime) / 1000).toFixed(1);

    for (const company of result.companies) {
      console.log(`\n${c('bold', `═══ ${company.company} ═══`)}\n`);
      if (company.error) {
        console.log(`  ${c('red', 'Error:')} ${company.error}\n`);
      } else {
        console.log(company.report);
      }
    }

    console.log(`\n${c('bold', '═══ Comparative Summary ═══')}\n`);
    console.log(result.comparative);
    console.log(`\n  ${c('green', '✓')} ${c('bold', 'Batch Complete')} ${c('dim', `— ${duration}s | ${companies.length} companies`)}\n`);

    await this.bridge?.disconnect().catch(() => {});
    if (this.fmpBridge) await this.fmpBridge.disconnect().catch(() => {});
  }

  // ── Single-agent mode (direct claudeAgent) ─────────────────────

  private async runSingleAgent(userQuery: string, agentName: string = DEFAULT_AGENT): Promise<void> {
    const agent = injectSkills(loadAgent(agentName), skillsDir);

    setupEnv();
    const model = process.env.COMPLETION_MODEL!;

    console.log(`\n  ${c('bold', 'CFA Agent Analyst')} ${c('dim', '— single-agent mode')}`);
    console.log(`  ${c('dim', `Model: ${model} | Agent: ${agent.name}`)}\n`);

    const startTime = Date.now();

    const { output } = await claudeAgent(
      agent,
      userQuery,
      (chunk: string) => { process.stdout.write(chunk); },
      model,
    );

    const duration = ((Date.now() - startTime) / 1000).toFixed(1);

    if (output && !process.stdout.isTTY) {
      console.log(output);
    }

    console.log(`\n  ${c('green', '✓')} ${c('bold', 'Complete')} ${c('dim', `— ${duration}s`)}\n`);
  }

  // ── Subcommand: list ────────────────────────────────────────────

  listAgents(): void {
    const agents = listAllAgents();

    console.log(`\n  ${c('bold', `${agents.length} agents available:`)} ${c('dim', '(use --agent <name> to select)')}\n`);

    for (const agent of agents) {
      const desc = agent.description.length > 80
        ? agent.description.slice(0, 77) + '...'
        : agent.description;
      const padded = agent.name.padEnd(30, ' ');
      const marker = agent.name === DEFAULT_AGENT ? ` ${c('green', '(default)')}` : '';
      console.log(`    ${c('cyan', padded)} ${c('dim', desc)}${marker}`);
    }

    console.log();
  }

  // ── Subcommand: tools ───────────────────────────────────────────

  async listTools(): Promise<void> {
    await this.connect();
    await this.listToolsInner();
    await this.bridge!.disconnect();
    if (this.fmpBridge) await this.fmpBridge.disconnect();
  }

  private async listToolsInner(): Promise<void> {
    if (!this.bridge) {
      await this.connect();
    }

    const tools = await this.bridge!.listTools();

    const grouped = new Map<string, string[]>();
    for (const tool of tools) {
      const parts = tool.name.split('_');
      const domain = parts.length > 1 ? parts.slice(0, 2).join('_') : parts[0];
      if (!grouped.has(domain)) grouped.set(domain, []);
      grouped.get(domain)!.push(tool.name);
    }

    console.log(`\n  ${c('bold', `${tools.length} MCP tools`)} ${c('dim', `across ${grouped.size} domains:`)}\n`);

    for (const [domain, domainTools] of grouped) {
      console.log(`    ${c('yellow', domain)} ${c('dim', `(${domainTools.length})`)}`);
      for (const t of domainTools) {
        console.log(`      ${c('dim', '●')} ${t}`);
      }
    }

    console.log();
  }

  // ── MCP connection ────────────────────────────────────────────

  private async connect(): Promise<{
    callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
    callFmpTool?: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
    bridge: McpBridge;
  }> {
    if (this.bridge) {
      const callFmpTool = this.fmpBridge ? (name: string, params: Record<string, unknown>) => this.fmpBridge!.callTool(name, params) : undefined;
      return { callTool: this.bridge.callTool.bind(this.bridge), callFmpTool, bridge: this.bridge };
    }

    const mcpServerPath = join(__cliDir, '..', '..', '..', 'mcp-server', 'dist', 'index.js');

    process.stderr.write(`  ${c('dim', 'Connecting to MCP server...')}\r`);
    const { callTool, bridge } = await createToolCaller({ serverPath: mcpServerPath });
    this.bridge = bridge;

    let callFmpTool: ((toolName: string, params: Record<string, unknown>) => Promise<unknown>) | undefined;
    if (process.env.FMP_API_KEY) {
      try {
        process.stderr.write(`  ${c('dim', 'Connecting to FMP MCP server...')}\r`);
        const fmpServerPath = join(__cliDir, '..', '..', '..', 'fmp-mcp-server', 'dist', 'index.js');
        const fmpResult = await createFmpToolCaller({ serverPath: fmpServerPath });
        this.fmpBridge = fmpResult.bridge;
        callFmpTool = fmpResult.callFmpTool;
      } catch (err) {
        process.stderr.write(`  ${c('yellow', 'FMP bridge unavailable:')} ${err instanceof Error ? err.message : String(err)}\n`);
      }
    }

    process.stderr.write('                                        \r');

    return { callTool, callFmpTool, bridge };
  }
}

// ── Entry point ─────────────────────────────────────────────────────

const cli = new CfaCli();
cli.start().catch((err) => {
  console.error(`${c('red', 'Fatal:')} ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});
