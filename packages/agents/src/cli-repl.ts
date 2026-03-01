// Interactive REPL for CFA Agent Analyst
// Extracted from cli.ts for ARCH-003 compliance (500-line limit)

import { createInterface } from 'node:readline';
import { CfaPipeline, injectSkills, type Topology } from './pipeline.js';
import type { McpBridge } from '../bridge/mcp-client.js';
import { FmpBridge } from '../bridge/fmp-bridge.js';
import { BatchAnalyzer } from '../orchestrator/batch-analyzer.js';

// ── ANSI helpers ────────────────────────────────────────────────────

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
};

export function c(color: keyof typeof ansi, text: string): string {
  return `${ansi[color]}${text}${ansi.reset}`;
}

// ── REPL implementation ─────────────────────────────────────────────

export async function startRepl(
  opts: {
    agentName?: string;
    initialTopology?: Topology;
    model: string;
    loadAgent: (name: string) => any;
    defaultAgent: string;
    skillsDir: string;
    connectFn: () => Promise<{
      callTool: (name: string, params: Record<string, unknown>) => Promise<unknown>;
      callFmpTool?: (name: string, params: Record<string, unknown>) => Promise<unknown>;
      bridge: McpBridge;
    }>;
    listAgentsFn: () => void;
    listToolsFn: () => Promise<void>;
    bridge: McpBridge | null;
    fmpBridge: FmpBridge | null;
  },
): Promise<void> {
  let currentAgent = opts.agentName;
  let usePipeline = !opts.agentName;
  let topology: Topology = opts.initialTopology ?? 'hierarchical';

  const { loadAgent, defaultAgent, model, skillsDir } = opts;
  const claudeAgent = (await import('agentic-flow/dist/agents/claudeAgent.js')).claudeAgent;

  const modeLabel = () => usePipeline ? 'pipeline' : `single-agent (${currentAgent ?? defaultAgent})`;

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
    if (!input) { rl.prompt(); return; }

    if (input === 'exit' || input === 'quit') {
      console.log(`  ${c('dim', 'Goodbye.')}\n`);
      rl.close();
      return;
    }

    if (input === '/help') {
      printReplHelp();
      rl.prompt();
      return;
    }

    if (input === '/agents') {
      opts.listAgentsFn();
      rl.prompt();
      return;
    }

    if (input.startsWith('/agent ')) {
      const name = input.slice(7).trim();
      try {
        loadAgent(name);
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
        currentAgent = currentAgent ?? defaultAgent;
        console.log(`  ${c('yellow', '✓')} Pipeline mode ${c('bold', 'disabled')} — using single-agent: ${c('cyan', currentAgent)}\n`);
      }
      rl.prompt();
      return;
    }

    if (input.startsWith('/topology ')) {
      const val = input.slice(10).trim();
      const valid: Topology[] = ['mesh', 'hierarchical', 'ring', 'star'];
      if (!valid.includes(val as Topology)) {
        console.error(`  ${c('red', 'Error:')} Invalid topology "${val}". Valid: ${valid.join(', ')}\n`);
      } else {
        topology = val as Topology;
        console.log(`  ${c('green', '✓')} Topology set to ${c('cyan', topology)}\n`);
      }
      rl.prompt();
      return;
    }

    if (input.startsWith('/batch ')) {
      const parts = input.slice(7).trim();
      const firstSpace = parts.indexOf(' ');
      if (firstSpace === -1) {
        console.error(`  ${c('red', 'Usage:')} /batch AAPL,MSFT,TSLA "query"\n`);
      } else {
        const companiesStr = parts.slice(0, firstSpace);
        const batchQuery = parts.slice(firstSpace + 1).trim().replace(/^["']|["']$/g, '');
        const batchCompanies = companiesStr.split(',').map(s => s.trim()).filter(Boolean);

        if (batchCompanies.length === 0 || !batchQuery) {
          console.error(`  ${c('red', 'Usage:')} /batch AAPL,MSFT,TSLA "query"\n`);
        } else {
          try {
            const { callTool, callFmpTool } = await opts.connectFn();
            const analyzer = new BatchAnalyzer({ callTool, callFmpTool });

            console.log(`  ${c('dim', `Batch: ${batchCompanies.join(', ')} | Query: ${batchQuery}`)}\n`);

            const batchStart = Date.now();
            const result = await analyzer.analyze(batchCompanies, batchQuery, {
              concurrency: 3,
              onProgress: (p) => {
                const statusIcon = p.status === 'completed' ? c('green', '✓')
                  : p.status === 'failed' ? c('red', '✗')
                  : c('yellow', '⟳');
                process.stderr.write(`  ${statusIcon} ${p.current} — ${p.status}\n`);
              },
            });

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

            const batchDuration = ((Date.now() - batchStart) / 1000).toFixed(1);
            console.log(`\n  ${c('green', '✓')} ${c('bold', 'Batch Complete')} ${c('dim', `— ${batchDuration}s`)}\n`);
          } catch (err) {
            console.error(`  ${c('red', 'Error:')} ${err instanceof Error ? err.message : String(err)}\n`);
          }
        }
      }
      rl.prompt();
      return;
    }

    if (input === '/tools') {
      await opts.listToolsFn();
      rl.prompt();
      return;
    }

    if (input === '/clear') {
      console.clear();
      rl.prompt();
      return;
    }

    // Analysis query
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
        const agent = injectSkills(loadAgent(currentAgent ?? defaultAgent), skillsDir);
        const startTime = Date.now();

        await claudeAgent(
          agent,
          input,
          (chunk: string) => { process.stdout.write(chunk); },
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
    if (opts.fmpBridge) await opts.fmpBridge.disconnect().catch(() => {});
    if (opts.bridge) await opts.bridge.disconnect().catch(() => {});
    process.exit(0);
  });

  rl.on('SIGINT', () => {
    console.log(`\n  ${c('dim', 'Goodbye.')}\n`);
    rl.close();
  });
}

// ── Help screens ────────────────────────────────────────────────────

export function printHelp(): void {
  console.log(`
  ${c('bold', 'CFA Agent Analyst')} — AI-powered financial analysis

  ${c('bold', 'Usage:')}
    cfa analyze "<query>"           Run multi-agent pipeline analysis
    cfa analyze --agent <name> ...  Run single-agent analysis
    cfa analyze --batch <cos> ...   Batch portfolio analysis (comma-separated or @file)
    cfa analyze --topology <type>   Set swarm topology (default: hierarchical)
    cfa analyze -i                  Start interactive REPL
    cfa list                        List available agents
    cfa tools                       List available MCP tools
    cfa --help                      Show this help

  ${c('bold', 'Examples:')}
    cfa analyze "Calculate WACC for beta 1.2, risk-free rate 4%, ERP 6%"
    cfa analyze --agent cfa-equity-analyst "Run DCF for revenue \\$500M"
    cfa analyze --batch "AAPL,MSFT,TSLA,JPM" "Compare credit risk"
    cfa analyze --batch @portfolio.txt "Full analysis"
    cfa analyze --topology mesh "Assess credit quality: D/E 0.6, coverage 5x"
    cfa analyze -i
`);
}

export function printAnalyzeHelp(): void {
  console.log(`
  ${c('bold', 'cfa analyze')} — Run financial analysis

  ${c('bold', 'Usage:')}
    cfa analyze [options] "<query>"
    cfa analyze -i

  ${c('bold', 'Options:')}
    -i, --interactive             Start interactive REPL mode
    --agent <name>                Single-agent mode (skip pipeline)
    --batch <companies>           Batch mode: comma-separated tickers or @file.txt
    --topology <type>             Swarm topology: mesh, hierarchical, ring, star
    --max-turns <n>               Max agent turns (default: 25)
    -h, --help                    Show this help

  ${c('bold', 'Modes:')}
    ${c('cyan', 'Pipeline (default)')}     Multi-agent: routing → agents → coordination → synthesis
    ${c('cyan', 'Single-agent')}           Direct agent call (use --agent to select)
    ${c('cyan', 'Batch')}                  Parallel portfolio analysis across N companies

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

function printReplHelp(): void {
  console.log(`
  ${c('bold', 'REPL commands:')}
    /help              Show this help
    /agents            List available agents
    /agent <name>      Switch to single-agent mode with <name>
    /pipeline          Toggle pipeline mode (multi-agent routing)
    /batch <cos> <q>   Batch portfolio analysis (e.g. /batch AAPL,MSFT "credit risk")
    /topology <type>   Set swarm topology (mesh/hierarchical/ring/star)
    /tools             List MCP tools
    /clear             Clear screen
    exit               Exit REPL
`);
}
