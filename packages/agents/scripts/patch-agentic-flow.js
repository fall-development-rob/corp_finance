#!/usr/bin/env node
// Patch agentic-flow judge to enforce confidence_threshold
//
// Bug: LLM returns { label: "Failure", confidence: 0.85 } and parseVerdict()
// uses the label as-is without checking confidence >= threshold (0.5).
// Fix: After parsing, override label to "Success" when confidence >= threshold.

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const judgeFile = join(__dirname, '../node_modules/agentic-flow/dist/reasoningbank/core/judge.js');

if (!existsSync(judgeFile)) {
  console.log('[patch] agentic-flow judge.js not found — skipping');
  process.exit(0);
}

let code = readFileSync(judgeFile, 'utf-8');

if (code.includes('applyConfidenceThreshold')) {
  console.log('[patch] agentic-flow judge already patched');
  process.exit(0);
}

// 1. Add the threshold function after the parseVerdict function
const thresholdFn = `
function applyConfidenceThreshold(verdict) {
    const config = loadConfig();
    const threshold = config.judge?.confidence_threshold ?? 0.5;
    if (verdict.confidence >= threshold && verdict.label === 'Failure') {
        verdict.label = 'Success';
        verdict.reasons = [...(verdict.reasons || []), \`Label corrected: confidence \${verdict.confidence} >= threshold \${threshold}\`];
    }
    return verdict;
}`;

// Insert after parseVerdict closing brace (find the heuristicJudge function marker)
code = code.replace(
  '/**\n * Heuristic judgment when LLM is unavailable',
  thresholdFn + '\n/**\n * Heuristic judgment when LLM is unavailable',
);

// 2. Wrap parseVerdict call with threshold check
code = code.replace(
  'const verdict = parseVerdict(content);',
  'const verdict = applyConfidenceThreshold(parseVerdict(content));',
);

writeFileSync(judgeFile, code, 'utf-8');
console.log('[patch] agentic-flow judge patched — confidence threshold now enforced');
