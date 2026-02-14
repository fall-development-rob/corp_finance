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
import { createInterface } from 'node:readline';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { createRequire } from 'node:module';
import { createToolCaller } from '../bridge/mcp-client.js';
import { FmpBridge, createFmpToolCaller } from '../bridge/fmp-bridge.js';
import { CfaPipeline, injectSkills, type Topology } from './pipeline.js';
import type { McpBridge } from '../bridge/mcp-client.js';

// Resolve agentic-flow deep imports via file path (bypasses exports map)
const _require = createRequire(import.meta.url);
const _afDir = dirname(_require.resolve('agentic-flow/package.json'));
const { claudeAgent } = await import(pathToFileURL(join(_afDir, 'dist', 'agents', 'claudeAgent.js')).href) as typeof import('agentic-flow/dist/agents/claudeAgent.js');
const { getAgent, listAgents } = await import(pathToFileURL(join(_afDir, 'dist', 'utils', 'agentLoader.js')).href) as typeof import('agentic-flow/dist/utils/agentLoader.js');

// ── Agent & skill directories ───────────────────────────────────────
const __cliDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__cliDir, '..', '..', '..', '..');
const cfaAgentsDir = join(repoRoot, '.claude', 'agents', 'cfa');

// ── ANSI helpers (no chalk dependency) ──────────────────────────────

const isTTY = process.stdout.isTTY ?? false;

const ansi = {
  reset: isTTY ? '\x1b[0m' : '',
  bold: isTTY ? '\x1b[1m' : '',
  dim: isTTY ? '\x1b[2m' : '',
  cyan: isTTY ? '\x1b[36m' : '',
  green: isTTY ? '\x1b[32m' : '',
  yellow: isTTY ? '\x1b[33m' : '',
  red: isTTY ? '\x1b[31m' : '',
  magenta: isTTY ? '\x1b[35m' : '',
  white: isTTY ? '\x1b[37m' : '',
  gray: isTTY ? '\x1b[90m' : '',
};

function c(color: keyof typeof ansi, text: string): string {
  return `${ansi[color]}${text}${ansi.reset}`;
}

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

  // Register/update CFA MCP server
  config.servers['cfa-tools'] = {
    enabled: true,
    command: 'node',
    args: [mcpServerPath],
    env: {},
  };

  // Register FMP market data MCP server (if FMP_API_KEY is set)
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
  // 1. Search .claude/agents/cfa/ (9 CFA specialist agents)
  const fromCfa = getAgent(name, cfaAgentsDir);
  if (fromCfa) return fromCfa;

  // 2. Search CWD (agentic-flow default)
  const fromCwd = getAgent(name);
  if (fromCwd) return fromCwd;

  // List available agents for the error message
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

  // Register CFA MCP server with agentic-flow
  const mcpServerPath = join(__cliDir, '..', '..', '..', 'mcp-server', 'dist', 'index.js');
  ensureMcpConfig(mcpServerPath);

  // Disable default agentic-flow MCP servers (not relevant for CFA analysis)
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

    // Handle --help / -h at top level
    if (rawArgs.includes('--help') || rawArgs.includes('-h') || rawArgs.length === 0) {
      this.printHelp();
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
        this.printHelp();
        break;
      default:
        console.error(`Unknown command: ${command}\n`);
        this.printHelp();
        process.exit(1);
    }
  }

  // ── Subcommand: analyze ─────────────────────────────────────────

  private async handleAnalyze(args: string[]): Promise<void> {
    let interactive = false;
    let agentName: string | undefined;
    let topology: Topology = 'hierarchical';
    const queryParts: string[] = [];

    for (let i = 0; i < args.length; i++) {
      const arg = args[i];
      if (arg === '-i' || arg === '--interactive') {
        interactive = true;
      } else if (arg === '--agent' && args[i + 1]) {
        agentName = args[++i];
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
        this.printAnalyzeHelp();
        return;
      } else {
        queryParts.push(arg);
      }
    }

    // Validate API key
    if (!process.env.ANTHROPIC_API_KEY) {
      console.error(`  ${c('red', 'Error:')} ANTHROPIC_API_KEY environment variable is required.\n`);
      console.error(`  Set it with: export ANTHROPIC_API_KEY=your-key-here\n`);
      process.exit(1);
    }

    if (interactive) {
      await this.startRepl(agentName, topology);
    } else {
      const query = queryParts.join(' ').trim();
      if (!query) {
        console.error('Error: No query provided. Use "cfa analyze --help" for usage.\n');
        process.exit(1);
      }

      if (agentName) {
        // Explicit --agent → single-agent mode
        await this.runSingleAgent(query, agentName);
      } else {
        // No --agent → pipeline mode
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

    const result = await pipeline.execute(
      userQuery,
      (chunk: string) => {
        process.stdout.write(chunk);
      },
    );

    const duration = ((Date.now() - startTime) / 1000).toFixed(1);

    // If nothing was streamed and single-agent, print the synthesis
    if (result.synthesis && !process.stdout.isTTY && result.agentResults.length === 1) {
      console.log(result.synthesis);
    }

    // Summary
    console.log(`\n  ${c('green', '✓')} ${c('bold', 'Complete')} ${c('dim', `— ${duration}s`)}`);
    console.log(`  ${c('dim', `Agents: ${result.routedAgents.join(', ')}`)}`);
    console.log(`  ${c('dim', `Timings: routing ${result.timings.routingMs}ms | memory ${result.timings.memoryMs}ms | agents ${result.timings.agentsMs}ms | coordination ${result.timings.coordinationMs}ms | synthesis ${result.timings.synthesisMs}ms`)}`);
    if (result.coordination) {
      console.log(`  ${c('dim', `Coordination: ${result.coordination.mechanism} | top: ${result.coordination.topAgents.join(', ')}`)}`);
    }
    console.log();
  }

  // ── Single-agent mode (direct claudeAgent) ─────────────────────

  private async runSingleAgent(userQuery: string, agentName: string = DEFAULT_AGENT): Promise<void> {
    const agent = injectSkills(loadAgent(agentName));

    setupEnv();
    const model = process.env.COMPLETION_MODEL!;

    console.log(`\n  ${c('bold', 'CFA Agent Analyst')} ${c('dim', '— single-agent mode')}`);
    console.log(`  ${c('dim', `Model: ${model} | Agent: ${agent.name}`)}\n`);

    const startTime = Date.now();

    const { output } = await claudeAgent(
      agent,
      userQuery,
      (chunk: string) => {
        // Stream callback — real-time output to stdout
        process.stdout.write(chunk);
      },
      model,
    );

    const duration = ((Date.now() - startTime) / 1000).toFixed(1);

    // If nothing was streamed, print the final output
    if (output && !process.stdout.isTTY) {
      console.log(output);
    }

    console.log(`\n  ${c('green', '✓')} ${c('bold', 'Complete')} ${c('dim', `— ${duration}s`)}\n`);
  }

  // ── Interactive REPL ────────────────────────────────────────────

  private async startRepl(agentName?: string, initialTopology: Topology = 'hierarchical'): Promise<void> {
    let currentAgent = agentName;
    let usePipeline = !agentName; // pipeline mode unless explicit agent
    let topology = initialTopology;

    setupEnv();
    const model = process.env.COMPLETION_MODEL!;

    const modeLabel = () => usePipeline ? 'pipeline' : `single-agent (${currentAgent ?? DEFAULT_AGENT})`;

    console.log(
      `\n  ${c('bold', 'CFA Agent Analyst')} ${c('dim', `— powered by agentic-flow`)}`,
    );
    console.log(`  ${c('dim', `Model: ${model} | Mode: ${modeLabel()} | Topology: ${topology}`)}`);
    console.log(`  ${c('dim', 'Type a query, or /help for commands.')}\n`);

    const rl = createInterface({
      input: process.stdin,
      output: process.stdout,
      prompt: `${c('cyan', 'cfa>')} `,
    });

    rl.prompt();

    rl.on('line', async (line: string) => {
      const input = line.trim();

      if (!input) {
        rl.prompt();
        return;
      }

      // REPL commands
      if (input === 'exit' || input === 'quit') {
        console.log(`  ${c('dim', 'Goodbye.')}\n`);
        rl.close();
        return;
      }

      if (input === '/help') {
        this.printReplHelp();
        rl.prompt();
        return;
      }

      if (input === '/agents') {
        this.listAgents();
        rl.prompt();
        return;
      }

      if (input.startsWith('/agent ')) {
        const name = input.slice(7).trim();
        try {
          loadAgent(name); // validate it exists
          currentAgent = name;
          usePipeline = false;
          console.log(`  ${c('green', '✓')} Switched to single-agent: ${c('cyan', name)}\n`);
        } catch (err) {
          console.error(`  ${c('red', 'Error:')} ${err instanceof Error ? err.message : String(err)}\n`);
        }
        rl.prompt();
        return;
      }

      if (input === '/pipeline') {
        usePipeline = !usePipeline;
        if (usePipeline) {
          currentAgent = undefined;
          console.log(`  ${c('green', '✓')} Pipeline mode ${c('bold', 'enabled')} (multi-agent routing + coordination)\n`);
        } else {
          currentAgent = currentAgent ?? DEFAULT_AGENT;
          console.log(`  ${c('yellow', '✓')} Pipeline mode ${c('bold', 'disabled')} — using single-agent: ${c('cyan', currentAgent)}\n`);
        }
        rl.prompt();
        return;
      }

      if (input.startsWith('/topology ')) {
        const val = input.slice(10).trim();
        if (!isValidTopology(val)) {
          console.error(`  ${c('red', 'Error:')} Invalid topology "${val}". Valid: ${VALID_TOPOLOGIES.join(', ')}\n`);
        } else {
          topology = val;
          console.log(`  ${c('green', '✓')} Topology set to ${c('cyan', topology)}\n`);
        }
        rl.prompt();
        return;
      }

      if (input === '/tools') {
        await this.listToolsInner();
        rl.prompt();
        return;
      }

      if (input === '/clear') {
        console.clear();
        rl.prompt();
        return;
      }

      // Anything else is an analysis query
      try {
        if (usePipeline) {
          const pipeline = new CfaPipeline({
            topology,
            onStatus: (stage, message) => {
              process.stderr.write(`  ${c('magenta', `[${stage}]`)} ${c('dim', message)}\n`);
            },
          });

          const startTime = Date.now();
          const result = await pipeline.execute(input, (chunk: string) => {
            process.stdout.write(chunk);
          });
          const duration = ((Date.now() - startTime) / 1000).toFixed(1);

          console.log(`\n  ${c('green', '✓')} ${c('bold', 'Complete')} ${c('dim', `— ${duration}s | agents: ${result.routedAgents.join(', ')}`)}\n`);
        } else {
          const agent = injectSkills(loadAgent(currentAgent ?? DEFAULT_AGENT));
          const startTime = Date.now();

          await claudeAgent(
            agent,
            input,
            (chunk: string) => {
              process.stdout.write(chunk);
            },
            model,
          );

          const duration = ((Date.now() - startTime) / 1000).toFixed(1);
          console.log(`\n  ${c('green', '✓')} ${c('bold', 'Complete')} ${c('dim', `— ${duration}s`)}\n`);
        }
      } catch (err) {
        console.error(`  ${c('red', 'Error:')} ${err instanceof Error ? err.message : String(err)}\n`);
      }

      rl.prompt();
    });

    rl.on('close', async () => {
      if (this.fmpBridge) await this.fmpBridge.disconnect().catch(() => {});
      if (this.bridge) await this.bridge.disconnect().catch(() => {});
      process.exit(0);
    });

    // Handle Ctrl+C gracefully
    rl.on('SIGINT', () => {
      console.log(`\n  ${c('dim', 'Goodbye.')}\n`);
      rl.close();
    });
  }

  // ── Subcommand: list ────────────────────────────────────────────

  listAgents(): void {
    const agents = listAllAgents();

    console.log(`\n  ${c('bold', `${agents.length} agents available:`)} ${c('dim', '(use --agent <name> to select)')}\n`);

    for (const agent of agents) {
      // Truncate long descriptions to fit terminal
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

    // Group tools by domain prefix (part before first underscore)
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

  // ── MCP connection (for `cfa tools` command) ──────────────────

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

    // Connect FMP bridge if FMP_API_KEY is available
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

    // Clear the "Connecting..." line
    process.stderr.write('                                        \r');

    return { callTool, callFmpTool, bridge };
  }

  // ── Help screens ────────────────────────────────────────────────

  printHelp(): void {
    console.log(`
  ${c('bold', 'CFA Agent Analyst')} — AI-powered financial analysis

  ${c('bold', 'Usage:')}
    cfa analyze "<query>"           Run multi-agent pipeline analysis
    cfa analyze --agent <name> ...  Run single-agent analysis
    cfa analyze --topology <type>   Set swarm topology (default: hierarchical)
    cfa analyze -i                  Start interactive REPL
    cfa list                        List available agents
    cfa tools                       List available MCP tools
    cfa --help                      Show this help

  ${c('bold', 'Examples:')}
    cfa analyze "Calculate WACC for beta 1.2, risk-free rate 4%, ERP 6%"
    cfa analyze --agent cfa-equity-analyst "Run DCF for revenue \\$500M"
    cfa analyze --topology mesh "Assess credit quality: D/E 0.6, coverage 5x"
    cfa analyze -i
`);
  }

  private printAnalyzeHelp(): void {
    console.log(`
  ${c('bold', 'cfa analyze')} — Run financial analysis

  ${c('bold', 'Usage:')}
    cfa analyze [options] "<query>"
    cfa analyze -i

  ${c('bold', 'Options:')}
    -i, --interactive             Start interactive REPL mode
    --agent <name>                Single-agent mode (skip pipeline)
    --topology <type>             Swarm topology: mesh, hierarchical, ring, star
    --max-turns <n>               Max agent turns (default: 25)
    -h, --help                    Show this help

  ${c('bold', 'Modes:')}
    ${c('cyan', 'Pipeline (default)')}     Multi-agent: routing → agents → coordination → synthesis
    ${c('cyan', 'Single-agent')}           Direct agent call (use --agent to select)

  ${c('bold', 'Environment:')}
    ANTHROPIC_API_KEY             Required. Your Anthropic API key.
    CFA_MODEL                     Default model override.
    PROVIDER                      Multi-provider routing (default: anthropic).

  ${c('bold', 'Examples:')}
    cfa analyze "Calculate WACC for beta 1.2, risk-free rate 4%, ERP 6%"
    cfa analyze --agent cfa-equity-analyst "Run DCF for revenue \\$500M"
    cfa analyze --topology mesh "Assess credit quality: D/E 0.6, coverage 5x"
    cfa analyze -i
`);
  }

  private printReplHelp(): void {
    console.log(`
  ${c('bold', 'REPL commands:')}
    /help              Show this help
    /agents            List available agents
    /agent <name>      Switch to single-agent mode with <name>
    /pipeline          Toggle pipeline mode (multi-agent routing)
    /topology <type>   Set swarm topology (mesh/hierarchical/ring/star)
    /tools             List MCP tools
    /clear             Clear screen
    exit               Exit REPL
`);
  }
}

// ── Entry point ─────────────────────────────────────────────────────

const cli = new CfaCli();
cli.start().catch((err) => {
  console.error(`${c('red', 'Fatal:')} ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});
