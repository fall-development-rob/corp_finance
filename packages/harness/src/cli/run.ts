#!/usr/bin/env node
/**
 * CFA Harness CLI entry point — Phase 31 Wave 1.
 *
 * Usage:
 *   cfa-harness run --agent <id> --prompt <text|@file>
 *                  [--output <path>] [--max-turns N] [--audit <path>]
 *
 * Flags:
 *   --agent     (required) Agent id to look up in the registry.
 *   --prompt    (required) Inline text, or @<path> to read from a file.
 *   --output    Write final text to this path instead of stdout.
 *   --max-turns Maximum model turns before forcing stop (default 25).
 *   --audit     Write a JSON audit log (DispatchEvent[] + result) to this path.
 *   --help      Print this usage message and exit 0.
 */

import { readFile, writeFile } from "node:fs/promises";
import { createAnthropicProvider } from "../core/providers/anthropic.js";
import { createStdioMCPClient } from "../mcp-client/stdio.js";
import { getAgent, defaultMCPServers } from "../agents/registry.js";
import { dispatch } from "../core/agent-loop.js";
import type { DispatchEvent, DispatchResult } from "../types.js";

// ---------------------------------------------------------------------------
// Arg parsing
// ---------------------------------------------------------------------------

interface ParsedArgs {
  agent: string;
  prompt: string;
  output: string | undefined;
  maxTurns: number;
  audit: string | undefined;
  help: boolean;
}

function printUsage(): void {
  process.stderr.write(`\
Usage: cfa-harness run --agent <id> --prompt <text|@file> [options]

Required:
  --agent <id>       Agent id (e.g. chief-analyst)
  --prompt <value>   Inline prompt text, or @<path> to read from a file

Optional:
  --output <path>    Write final text to file instead of stdout
  --max-turns <N>    Maximum model turns (default 25)
  --audit <path>     Write JSON audit log (events + result) to file
  --help             Print this message and exit

Example:
  cfa-harness run --agent chief-analyst --prompt @prompt.txt --max-turns 30
`);
}

function parseArgs(argv: string[]): ParsedArgs {
  const args = argv.slice(2); // strip node + script path
  // Strip leading "run" sub-command if present
  if (args[0] === "run") args.shift();

  const result: ParsedArgs = {
    agent: "",
    prompt: "",
    output: undefined,
    maxTurns: 25,
    audit: undefined,
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
      case "--audit":
        result.audit = next ?? "";
        i++;
        break;
      default:
        if (flag.startsWith("--")) {
          process.stderr.write(`[warn] Unknown flag: ${flag}\n`);
        }
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Prompt resolution
// ---------------------------------------------------------------------------

async function resolvePrompt(raw: string): Promise<string> {
  if (raw.startsWith("@")) {
    const filePath = raw.slice(1);
    const text = await readFile(filePath, "utf8");
    return text;
  }
  return raw;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

export async function main(): Promise<void> {
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
    `[dispatch] agent=${parsed.agent} prompt-len=${promptText.length}\n`,
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

  const startMs = Date.now();
  let result: DispatchResult;

  try {
    result = await dispatch({
      agent,
      prompt: promptText,
      provider: createAnthropicProvider(),
      mcp,
      maxTurns: parsed.maxTurns,
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
          case "turn_completed":
            process.stderr.write(
              `[turn ${event.turn}] turn_completed ${event.stopReason}\n`,
            );
            break;
          case "dispatch_completed":
            // handled after dispatch returns
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
    `[dispatch] tool_uses=${result.toolUses} turns=${result.messages.length} elapsed=${elapsedSec}s\n`,
  );

  // Write final text
  if (parsed.output) {
    await writeFile(parsed.output, result.finalText, "utf8");
  } else {
    process.stdout.write(result.finalText);
    if (!result.finalText.endsWith("\n")) {
      process.stdout.write("\n");
    }
  }

  // Write audit log
  if (parsed.audit) {
    const auditPayload = JSON.stringify({ events, result }, null, 2);
    await writeFile(parsed.audit, auditPayload, "utf8");
  }

  process.exit(0);
}

// Run when invoked directly (ESM: check import.meta.url vs process.argv[1])
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
