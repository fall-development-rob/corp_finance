/**
 * Minimal YAML frontmatter parser for SKILL.md and agent manifest files.
 *
 * Frontmatter parser is intentionally minimal; for full YAML, use the
 * cfa CLI loader (Wave 3).
 *
 * Supported subset:
 *   - key: value             (string, number, boolean)
 *   - key: [a, b, c]         (inline array of strings/numbers/booleans)
 *   - key:\n  - a\n  - b     (block sequence — strings)
 *   - Quoted values          (single or double quotes, values may contain colons)
 *   - description: |         (literal block scalar — multi-line string)
 *
 * Not supported: nested mappings, anchors/aliases, multi-document streams.
 */

const FENCE = "---";

/**
 * Split a markdown file into its YAML frontmatter and body.
 *
 * If no opening fence is found (or the file does not start with `---`),
 * returns `{ frontmatter: {}, body: content }`.
 */
export function parseFrontmatter(content: string): {
  frontmatter: Record<string, unknown>;
  body: string;
} {
  const lines = content.split("\n");

  // Must start with --- (possibly with trailing spaces)
  if (lines[0]?.trimEnd() !== FENCE) {
    return { frontmatter: {}, body: content };
  }

  // Find closing ---
  let closingIdx = -1;
  for (let i = 1; i < lines.length; i++) {
    if (lines[i]?.trimEnd() === FENCE) {
      closingIdx = i;
      break;
    }
  }

  if (closingIdx === -1) {
    // Unclosed frontmatter — treat entire content as body
    return { frontmatter: {}, body: content };
  }

  const yamlLines = lines.slice(1, closingIdx);
  const bodyLines = lines.slice(closingIdx + 1);
  // Normalize: strip leading blank line from body
  if (bodyLines[0] === "") {
    bodyLines.shift();
  }

  const frontmatter = parseYamlSubset(yamlLines);
  return { frontmatter, body: bodyLines.join("\n") };
}

// ---------------------------------------------------------------------------
// Internal mini-parser
// ---------------------------------------------------------------------------

function parseYamlSubset(lines: string[]): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  let i = 0;

  while (i < lines.length) {
    const line = lines[i] ?? "";
    i++;

    // Skip blank lines and comments
    if (line.trim() === "" || line.trimStart().startsWith("#")) continue;

    // Top-level key: value (unindented)
    const colonIdx = line.indexOf(":");
    if (colonIdx === -1 || line.startsWith(" ") || line.startsWith("\t")) continue;

    const key = line.slice(0, colonIdx).trim();
    const rawVal = line.slice(colonIdx + 1); // everything after the first colon

    // Literal block scalar: `key: |`
    if (rawVal.trimStart() === "|") {
      const { value, consumed } = parseLiteralBlock(lines, i);
      result[key] = value;
      i += consumed;
      continue;
    }

    // Block sequence: `key:\n  - item`
    if (rawVal.trim() === "" && i < lines.length && isSequenceItem(lines[i] ?? "")) {
      const { items, consumed } = parseBlockSequence(lines, i);
      result[key] = items;
      i += consumed;
      continue;
    }

    // Inline value
    result[key] = parseScalarOrInlineArray(rawVal.trim());
  }

  return result;
}

function isSequenceItem(line: string): boolean {
  return /^\s+-\s/.test(line) || /^\s+-$/.test(line);
}

function parseBlockSequence(
  lines: string[],
  startIdx: number,
): { items: unknown[]; consumed: number } {
  const items: unknown[] = [];
  let i = startIdx;

  while (i < lines.length) {
    const line = lines[i] ?? "";
    if (!isSequenceItem(line)) break;
    const item = line.replace(/^\s*-\s*/, "").trim();
    items.push(parseScalar(item));
    i++;
  }

  return { items, consumed: i - startIdx };
}

function parseLiteralBlock(
  lines: string[],
  startIdx: number,
): { value: string; consumed: number } {
  const blockLines: string[] = [];
  let i = startIdx;
  let indent = -1;

  while (i < lines.length) {
    const line = lines[i] ?? "";

    // A non-empty, unindented line signals end of block
    if (line.trim() !== "" && !line.startsWith(" ") && !line.startsWith("\t")) {
      break;
    }

    if (line.trim() === "") {
      blockLines.push("");
      i++;
      continue;
    }

    // Determine indent level from first indented line
    if (indent === -1) {
      indent = line.length - line.trimStart().length;
    }

    blockLines.push(line.slice(indent));
    i++;
  }

  // Trim trailing blank lines; add final newline per YAML spec
  while (blockLines.length > 0 && blockLines[blockLines.length - 1] === "") {
    blockLines.pop();
  }

  return { value: blockLines.join("\n") + "\n", consumed: i - startIdx };
}

function parseScalarOrInlineArray(raw: string): unknown {
  // Inline array: [a, b, c]
  if (raw.startsWith("[") && raw.endsWith("]")) {
    const inner = raw.slice(1, -1);
    if (inner.trim() === "") return [];
    return inner.split(",").map((s) => parseScalar(s.trim()));
  }
  return parseScalar(raw);
}

function parseScalar(raw: string): unknown {
  // Strip surrounding quotes
  if (
    (raw.startsWith('"') && raw.endsWith('"')) ||
    (raw.startsWith("'") && raw.endsWith("'"))
  ) {
    return raw.slice(1, -1);
  }
  if (raw === "true") return true;
  if (raw === "false") return false;
  if (raw === "null" || raw === "~") return null;
  // Number (int or float)
  if (/^-?\d+(\.\d+)?$/.test(raw)) return Number(raw);
  return raw;
}
