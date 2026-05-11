#!/usr/bin/env node
/**
 * CFA Harness CLI entry point — Phase 31.
 *
 * Subcommands:
 *   cfa-harness run --agent <id> --prompt <text|@file> [options]
 *   cfa-harness cookbook run <slug> --prompt <text|@file> [options]
 *
 * Agent dispatch (legacy / default):
 *   --agent     (required) Agent id to look up in the registry.
 *   --prompt    (required) Inline text, or @<path> to read from a file.
 *   --output    Write final text to this path instead of stdout.
 *   --max-turns Maximum model turns before forcing stop (default 25).
 *   --events    Write a JSON DispatchEvent log + result to this path.
 *   --delegates Enable chief→specialist delegation.
 *   --audit-dir <dir>    Persist sha256-hashed AuditRecord per dispatch.
 *   --session-dir <dir>  Persist SessionState per dispatch (durable replay).
 *
 * Cookbook dispatch:
 *   cfa-harness cookbook run <slug>
 *   --prompt <text|@file>        (required)
 *   --cookbooks-root <dir>       (default: ./managed-agent-cookbooks)
 *   --output <path>              Write final text to file (default: stdout)
 *   --events <path>              Write JSON DispatchEvent log to file
 *   --max-handoff-depth <n>      (default: 2)
 *   --max-chain-depth <n>        (default: 5)
 *   --audit-dir <dir>
 *   --session-dir <dir>
 *
 *   --help      Print this message and exit 0.
 */

import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createAnthropicProvider } from "../core/providers/anthropic.js";
import { createStdioMCPClient } from "../mcp-client/stdio.js";
import {
  defaultDelegates,
  defaultMCPServers,
  getAgent,
} from "../agents/registry.js";
import { dispatch } from "../core/agent-loop.js";
import { createFileAuditSink } from "../audit/index.js";
import { createFileSessionStore } from "../memory/index.js";
import { dispatchCookbook } from "../runtime/dispatch-cookbook.js";
import { createCookbookLoader } from "../manifests/cookbook-loader.js";
import type { DispatchEvent, DispatchResult } from "../types.js";
import type { ParsedSkill, SkillLoader } from "../skills/types.js";

interface ParsedArgs {
  agent: string;
  prompt: string;
  output: string | undefined;
  maxTurns: number;
  events: string | undefined;
  auditDir: string | undefined;
  sessionDir: string | undefined;
  delegates: boolean;
  help: boolean;
}

function printUsage(): void {
  process.stderr.write(`\
Usage: cfa-harness run --agent <id> --prompt <text|@file> [options]

Required:
  --agent <id>            Agent id (e.g. chief-analyst)
  --prompt <value>        Inline text, or @<path> to read from a file

Output:
  --output <path>         Write final text to file instead of stdout
  --max-turns <N>         Max model turns (default 25)

Audit + memory (Wave 4):
  --audit-dir <dir>       Write sha256-hashed AuditRecord per dispatch
  --session-dir <dir>     Persist SessionState per dispatch (replay support)
  --events <path>         Write DispatchEvent log + result JSON to file

Delegation (Wave 2):
  --delegates             Enable chief→specialist delegation (chief sees
                          the 8 cfa specialists as delegate_to_<id> tools)

  --help                  Print this message and exit

Example:
  cfa-harness run \\
    --agent chief-analyst \\
    --prompt @deal.md \\
    --delegates \\
    --audit-dir out/audits \\
    --session-dir out/sessions
`);
}

function parseArgs(argv: string[]): ParsedArgs {
  const args = argv.slice(2);
  if (args[0] === "run") args.shift();

  const result: ParsedArgs = {
    agent: "",
    prompt: "",
    output: undefined,
    maxTurns: 25,
    events: undefined,
    auditDir: undefined,
    sessionDir: undefined,
    delegates: false,
    help: false,
  };

  for (let i = 0; i < args.length; i++) {
    const flag = args[i] ?? "";
    const next = args[i + 1];

    switch (flag) {
      case "--help":
      case "-h":
        result.help = true;
        break;
      case "--agent":
        result.agent = next ?? "";
        i++;
        break;
      case "--prompt":
        result.prompt = next ?? "";
        i++;
        break;
      case "--output":
        result.output = next ?? "";
        i++;
        break;
      case "--max-turns":
        result.maxTurns = parseInt(next ?? "25", 10);
        i++;
        break;
      case "--events":
        result.events = next ?? "";
        i++;
        break;
      case "--audit-dir":
        result.auditDir = next ?? "";
        i++;
        break;
      case "--session-dir":
        result.sessionDir = next ?? "";
        i++;
        break;
      case "--delegates":
        result.delegates = true;
        break;
      default:
        if (flag.startsWith("--")) {
          process.stderr.write(`[warn] Unknown flag: ${flag}\n`);
        }
    }
  }

  return result;
}

async function resolvePrompt(raw: string): Promise<string> {
  if (raw.startsWith("@")) {
    const filePath = raw.slice(1);
    const text = await readFile(filePath, "utf8");
    return text;
  }
  return raw;
}

// ---------------------------------------------------------------------------
// Cookbook subcommand
// ---------------------------------------------------------------------------

interface ParsedCookbookArgs {
  slug: string;
  prompt: string;
  cookbooksRoot: string;
  output: string | undefined;
  events: string | undefined;
  auditDir: string | undefined;
  sessionDir: string | undefined;
  maxHandoffDepth: number;
  maxChainDepth: number;
  help: boolean;
}

function printCookbookUsage(): void {
  process.stderr.write(`\
Usage: cfa-harness cookbook run <slug> --prompt <text|@file> [options]

Required:
  <slug>                      Cookbook slug to dispatch
  --prompt <value>            Inline text, or @<path> to read from a file

Output:
  --output <path>             Write final text to file instead of stdout
  --events <path>             Write DispatchEvent log + result JSON to file

Cookbook root:
  --cookbooks-root <dir>      Path to managed-agent-cookbooks/ (default: ./managed-agent-cookbooks)

Handoff tuning:
  --max-handoff-depth <n>     Max depth at which initiate_handoff is injected (default: 2)
  --max-chain-depth <n>       Max orchestrator chain depth before rejection (default: 5)

Audit + memory:
  --audit-dir <dir>           Write sha256-hashed AuditRecord per dispatch
  --session-dir <dir>         Persist SessionState per dispatch (replay support)

  --help                      Print this message and exit

Example:
  cfa-harness cookbook run equity-analyst \\
    --prompt @deal.md \\
    --cookbooks-root ./managed-agent-cookbooks \\
    --audit-dir out/audits
`);
}

function parseCookbookArgs(argv: string[]): ParsedCookbookArgs {
  // argv is process.argv; expected shape: [..., "cookbook", "run", <slug>, ...flags]
  const args = argv.slice(2);

  // Consume "cookbook" and "run" tokens
  const cookbookIdx = args.indexOf("cookbook");
  if (cookbookIdx !== -1) args.splice(cookbookIdx, 1);
  const runIdx = args.indexOf("run");
  if (runIdx !== -1) args.splice(runIdx, 1);

  const result: ParsedCookbookArgs = {
    slug: "",
    prompt: "",
    cookbooksRoot: resolve(process.cwd(), "managed-agent-cookbooks"),
    output: undefined,
    events: undefined,
    auditDir: undefined,
    sessionDir: undefined,
    maxHandoffDepth: 2,
    maxChainDepth: 5,
    help: false,
  };

  for (let i = 0; i < args.length; i++) {
    const flag = args[i] ?? "";
    const next = args[i + 1];

    switch (flag) {
      case "--help":
      case "-h":
        result.help = true;
        break;
      case "--prompt":
        result.prompt = next ?? "";
        i++;
        break;
      case "--cookbooks-root":
        result.cookbooksRoot = resolve(next ?? "");
        i++;
        break;
      case "--output":
        result.output = next ?? "";
        i++;
        break;
      case "--events":
        result.events = next ?? "";
        i++;
        break;
      case "--audit-dir":
        result.auditDir = next ?? "";
        i++;
        break;
      case "--session-dir":
        result.sessionDir = next ?? "";
        i++;
        break;
      case "--max-handoff-depth":
        result.maxHandoffDepth = parseInt(next ?? "2", 10);
        i++;
        break;
      case "--max-chain-depth":
        result.maxChainDepth = parseInt(next ?? "5", 10);
        i++;
        break;
      default:
        if (!flag.startsWith("--") && result.slug === "") {
          result.slug = flag;
        } else if (flag.startsWith("--")) {
          process.stderr.write(`[warn] Unknown flag: ${flag}\n`);
        }
    }
  }

  return result;
}

async function runCookbookSubcommand(): Promise<void> {
  const parsed = parseCookbookArgs(process.argv);

  if (parsed.help) {
    printCookbookUsage();
    process.exit(0);
  }

  if (!parsed.slug) {
    process.stderr.write("[error] cookbook slug is required\n");
    printCookbookUsage();
    process.exit(1);
  }

  if (!parsed.prompt) {
    process.stderr.write("[error] --prompt is required\n");
    printCookbookUsage();
    process.exit(1);
  }

  let promptText: string;
  try {
    promptText = await resolvePrompt(parsed.prompt);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[error] Failed to read prompt file: ${msg}\n`);
    process.exit(1);
  }

  process.stderr.write(
    `[cookbook] slug=${parsed.slug} cookbooks-root=${parsed.cookbooksRoot}` +
      ` prompt-len=${promptText.length}` +
      (parsed.auditDir ? ` audit=${parsed.auditDir}` : "") +
      (parsed.sessionDir ? ` session=${parsed.sessionDir}` : "") +
      "\n",
  );

  const events: DispatchEvent[] = [];
  const mcp = createStdioMCPClient(defaultMCPServers);

  try {
    await mcp.initialize();
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[error] MCP initialization failed: ${msg}\n`);
    process.exit(1);
  }

  const audit = parsed.auditDir
    ? createFileAuditSink({ dir: parsed.auditDir })
    : undefined;
  const session = parsed.sessionDir
    ? createFileSessionStore({ dir: parsed.sessionDir })
    : undefined;

  // Build a null skill loader — CLI cookbooks use from_plugin exclusively
  const nullSkillLoader: SkillLoader = {
    async loadSkill(id: string): Promise<ParsedSkill> {
      throw new Error(`Unexpected from_skill reference to "${id}" in cookbook`);
    },
    async loadAgent() {
      throw new Error("cfa-harness cookbook: null skill loader does not support loadAgent");
    },
    clearCache() {},
  };

  const cookbookLoader = createCookbookLoader({
    cookbooksRoot: parsed.cookbooksRoot,
    skillLoader: nullSkillLoader,
  });

  const startMs = Date.now();
  let result: DispatchResult;

  try {
    result = await dispatchCookbook({
      slug: parsed.slug,
      cookbookLoader,
      prompt: promptText,
      provider: createAnthropicProvider(),
      mcp,
      maxHandoffDepth: parsed.maxHandoffDepth,
      maxChainDepth: parsed.maxChainDepth,
      ...(audit ? { audit } : {}),
      ...(session ? { session } : {}),
      onEvent: (event) => {
        events.push(event);
        switch (event.type) {
          case "turn_started":
            process.stderr.write(
              `[turn ${event.turn}] started agent=${event.agentId}\n`,
            );
            break;
          case "tool_call":
            process.stderr.write(
              `[turn ${event.turn}] tool_call ${event.call.name}\n`,
            );
            break;
          case "handoff_initiated":
            process.stderr.write(
              `[handoff] → ${event.target_agent}\n`,
            );
            break;
          case "handoff_returned":
            process.stderr.write(
              `[handoff] ← ${event.target_agent} ok=${event.ok} ${event.duration_ms}ms\n`,
            );
            break;
          case "turn_completed":
            process.stderr.write(
              `[turn ${event.turn}] turn_completed ${event.stopReason}\n`,
            );
            break;
          default:
            break;
        }
      },
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[error] Cookbook dispatch failed: ${msg}\n`);
    await mcp.close();
    process.exit(1);
  }

  await mcp.close();

  const elapsedSec = ((Date.now() - startMs) / 1000).toFixed(1);
  process.stderr.write(
    `[cookbook] tool_uses=${result.toolUses} turns=${result.messages.length}` +
      ` elapsed=${elapsedSec}s` +
      (result.auditId ? ` audit_id=${result.auditId}` : "") +
      (result.sessionId ? ` session_id=${result.sessionId}` : "") +
      "\n",
  );

  if (parsed.output) {
    await writeFile(parsed.output, result.finalText, "utf8");
  } else {
    process.stdout.write(result.finalText);
    if (!result.finalText.endsWith("\n")) {
      process.stdout.write("\n");
    }
  }

  if (parsed.events) {
    const eventsPayload = JSON.stringify({ events, result }, null, 2);
    await writeFile(parsed.events, eventsPayload, "utf8");
  }

  process.exit(0);
}

// ---------------------------------------------------------------------------
// Top-level subcommand router
// ---------------------------------------------------------------------------

function isCookbookSubcommand(): boolean {
  const args = process.argv.slice(2);
  // Match: cfa-harness cookbook run <slug> [flags]
  // args[0] === "cookbook" OR args[0] === "run" && args[1] === "cookbook"
  return args[0] === "cookbook" || (args[0] === "run" && args[1] === "cookbook");
}

export async function main(): Promise<void> {
  if (isCookbookSubcommand()) {
    await runCookbookSubcommand();
    return;
  }

  const parsed = parseArgs(process.argv);

  if (parsed.help) {
    printUsage();
    process.exit(0);
  }

  if (!parsed.agent) {
    process.stderr.write("[error] --agent is required\n");
    printUsage();
    process.exit(1);
  }

  if (!parsed.prompt) {
    process.stderr.write("[error] --prompt is required\n");
    printUsage();
    process.exit(1);
  }

  let agent;
  try {
    agent = getAgent(parsed.agent);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[error] ${msg}\n`);
    process.exit(1);
  }

  let promptText: string;
  try {
    promptText = await resolvePrompt(parsed.prompt);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[error] Failed to read prompt file: ${msg}\n`);
    process.exit(1);
  }

  process.stderr.write(
    `[dispatch] agent=${parsed.agent} prompt-len=${promptText.length}` +
      (parsed.delegates ? " delegates=on" : "") +
      (parsed.auditDir ? ` audit=${parsed.auditDir}` : "") +
      (parsed.sessionDir ? ` session=${parsed.sessionDir}` : "") +
      "\n",
  );

  const events: DispatchEvent[] = [];
  const mcp = createStdioMCPClient(defaultMCPServers);

  try {
    await mcp.initialize();
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[error] MCP initialization failed: ${msg}\n`);
    process.exit(1);
  }

  const audit = parsed.auditDir
    ? createFileAuditSink({ dir: parsed.auditDir })
    : undefined;
  const session = parsed.sessionDir
    ? createFileSessionStore({ dir: parsed.sessionDir })
    : undefined;

  const startMs = Date.now();
  let result: DispatchResult;

  try {
    result = await dispatch({
      agent,
      prompt: promptText,
      provider: createAnthropicProvider(),
      mcp,
      maxTurns: parsed.maxTurns,
      ...(parsed.delegates ? { delegates: defaultDelegates } : {}),
      ...(audit ? { audit } : {}),
      ...(session ? { session } : {}),
      onEvent: (event) => {
        events.push(event);
        switch (event.type) {
          case "turn_started":
            process.stderr.write(
              `[turn ${event.turn}] started agent=${event.agentId}\n`,
            );
            break;
          case "tool_call":
            process.stderr.write(
              `[turn ${event.turn}] tool_call ${event.call.name}\n`,
            );
            break;
          case "delegation":
            process.stderr.write(
              `[delegation] → ${event.targetAgent}\n`,
            );
            break;
          case "turn_completed":
            process.stderr.write(
              `[turn ${event.turn}] turn_completed ${event.stopReason}\n`,
            );
            break;
          default:
            break;
        }
      },
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[error] Dispatch failed: ${msg}\n`);
    await mcp.close();
    process.exit(1);
  }

  await mcp.close();

  const elapsedSec = ((Date.now() - startMs) / 1000).toFixed(1);
  process.stderr.write(
    `[dispatch] tool_uses=${result.toolUses} turns=${result.messages.length}` +
      ` elapsed=${elapsedSec}s` +
      (result.auditId ? ` audit_id=${result.auditId}` : "") +
      (result.sessionId ? ` session_id=${result.sessionId}` : "") +
      "\n",
  );

  if (parsed.output) {
    await writeFile(parsed.output, result.finalText, "utf8");
  } else {
    process.stdout.write(result.finalText);
    if (!result.finalText.endsWith("\n")) {
      process.stdout.write("\n");
    }
  }

  if (parsed.events) {
    const eventsPayload = JSON.stringify({ events, result }, null, 2);
    await writeFile(parsed.events, eventsPayload, "utf8");
  }

  process.exit(0);
}

import { fileURLToPath } from "node:url";
const _thisFile = fileURLToPath(import.meta.url);
const _invokedFile = process.argv[1] ?? "";

if (
  _invokedFile === _thisFile ||
  _invokedFile.endsWith("/run.js") ||
  _invokedFile.endsWith("/run.ts")
) {
  main().catch((err) => {
    process.stderr.write(`[fatal] ${String(err)}\n`);
    process.exit(1);
  });
}
