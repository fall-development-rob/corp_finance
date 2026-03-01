// Agent runner with abort support and output deduplication
// Extracted from pipeline.ts for ARCH-003 compliance (500-line limit)

import type { AgentDefinition } from 'agentic-flow/dist/utils/agentLoader.js';

// ── Dedup helper ────────────────────────────────────────────────────

export function deduplicateOutput(text: string): string {
  if (text.length < 1500) return text;
  const headingMatch = text.match(/^(#{1,3}\s+.{5,})/m);
  if (!headingMatch) return text;
  const anchor = headingMatch[1];
  const firstIdx = text.indexOf(anchor);
  const secondIdx = text.indexOf(anchor, firstIdx + anchor.length);
  if (secondIdx > firstIdx) {
    const firstHalf = text.slice(firstIdx, secondIdx).trim();
    const secondHalf = text.slice(secondIdx).trim();
    return secondHalf.length >= firstHalf.length ? secondHalf : firstHalf;
  }
  const mid = Math.floor(text.length / 2);
  const probe = text.slice(mid, mid + 200);
  const probeIdx = text.indexOf(probe);
  if (probeIdx >= 0 && probeIdx < mid - 200) {
    return text.slice(0, mid).trim();
  }
  return text;
}

// ── Agent runner with abort support ─────────────────────────────────

export async function runAgentWithAbort(
  agent: AgentDefinition,
  input: string,
  opts: { timeoutMs: number; maxTurns: number; onToolCall?: (name: string, count: number) => void },
): Promise<{ output: string; agent: string }> {
  const { query } = await import('@anthropic-ai/claude-agent-sdk');
  const ac = new AbortController();
  const timer = setTimeout(() => ac.abort(), opts.timeoutMs);

  // Load MCP servers from user config
  const mcpServers: Record<string, any> = {};
  try {
    const fs = await import('fs');
    const path = await import('path');
    const os = await import('os');
    const configPath = path.join(os.homedir(), '.agentic-flow', 'mcp-config.json');
    if (fs.existsSync(configPath)) {
      const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
      for (const [name, server] of Object.entries(config.servers || {})) {
        const s = server as any;
        if (s.enabled) {
          mcpServers[name] = { type: 'stdio', command: s.command, args: s.args || [], env: { ...process.env, ...s.env } };
        }
      }
    }
  } catch { /* no config */ }

  const assistantChunks: string[] = [];
  let resultOutput = '';
  let toolCallCount = 0;
  const toolResults: string[] = [];

  try {
    const hasMcp = Object.keys(mcpServers).length > 0;
    const stream = query({
      prompt: input,
      options: {
        systemPrompt: agent.systemPrompt,
        model: process.env.CFA_MODEL || process.env.COMPLETION_MODEL || 'claude-haiku-4-5-20251001',
        permissionMode: 'bypassPermissions',
        allowDangerouslySkipPermissions: true,
        maxTurns: opts.maxTurns,
        abortController: ac,
        tools: ['Bash'],
        disallowedTools: ['WebSearch', 'WebFetch', 'Write', 'Edit', 'Read', 'Glob', 'Grep', 'NotebookEdit', 'Task'],
        mcpServers: hasMcp ? mcpServers : undefined,
      } as any,
    });
    for await (const msg of stream) {
      if (ac.signal.aborted) break;
      const msgType = (msg as any).type;
      if (msgType === 'assistant') {
        const chunk = (msg as any).message?.content?.map((c: any) => c.type === 'text' ? c.text : '').join('') || '';
        if (chunk.length > 0) assistantChunks.push(chunk);
        const toolBlocks = (msg as any).message?.content?.filter((c: any) => c.type === 'tool_use') || [];
        for (const tb of toolBlocks) {
          toolCallCount++;
          opts.onToolCall?.(tb.name || 'unknown', toolCallCount);
        }
      } else if (msgType === 'user') {
        const toolResult = (msg as any).tool_use_result;
        if (toolResult && typeof toolResult === 'string' && toolResult.length > 50) {
          toolResults.push(toolResult.slice(0, 4000));
        } else if (toolResult && typeof toolResult === 'object') {
          const s = JSON.stringify(toolResult).slice(0, 4000);
          if (s.length > 50) toolResults.push(s);
        }
      } else if (msgType === 'result') {
        resultOutput = (msg as any).result || '';
      }
    }

    clearTimeout(timer);
    const longestChunk = assistantChunks.reduce((a, b) => b.length > a.length ? b : a, '');
    const raw = (longestChunk.length > 500 ? longestChunk : resultOutput) || assistantChunks.join('\n');
    return { output: deduplicateOutput(raw), agent: agent.name };
  } catch (err) {
    clearTimeout(timer);
    if (ac.signal.aborted) {
      const longestPartial = assistantChunks.reduce((a, b) => b.length > a.length ? b : a, '');
      const partial = resultOutput || longestPartial || assistantChunks.join('\n');
      if (partial.length > 100) {
        return { output: `[Partial — timed out after ${opts.timeoutMs / 1000}s]\n\n${partial}`, agent: agent.name };
      }
      if (toolResults.length > 0) {
        const synthesized = toolResults.slice(-5).join('\n\n---\n\n');
        return { output: `[Partial — timed out after ${opts.timeoutMs / 1000}s, ${toolCallCount} tool calls]\n\nTool results:\n${synthesized}`, agent: agent.name };
      }
      throw new Error(`Agent ${agent.name} timed out after ${opts.timeoutMs / 1000}s`);
    }
    throw err;
  }
}
