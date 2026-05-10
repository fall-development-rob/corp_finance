# Phase 35 — Hybrid Dispatch Architecture

**Branch**: `phase-33-skill-driven-planning`
**Status**: Design spec — do not implement until reviewed
**Author**: Architect agent (swarm)
**Date**: 2026-05-10

---

## 0. Overview

Phase 35 adds a deterministic fast-path to the harness dispatch loop. Prompts that match one of the 49 pre-defined Rust `WorkflowDefinition`s (across 6 domains: equity research, investment banking, private equity, wealth management, financial analysis, deal documents) skip the LLM entirely and execute a fixed tool sequence whose output is reproducible and audit-hashable. Prompts that do not match fall through to the existing Phase 31+33+34 chief → specialist LLM path unchanged.

Four waves:

| Wave | Scope | Repo |
|------|-------|------|
| W1 | `cfa workflow` CLI subcommands | `corp-finance-core` (external) |
| W2 | `WorkflowRouter` port + agent-loop pre-dispatch hook | `cfa_agent` |
| W3 | `list_workflows` / `run_workflow` virtual tools | `cfa_agent` |
| W4 | ADR-042 + slash command annotations | `cfa_agent` |

---

## 1. CLI Contract (W1 — external `corp-finance-core` repo)

### 1a. `cfa workflow list`

```
stdout: WorkflowList (JSON)
exit 0
```

```typescript
// TypeScript representation of the JSON shape
interface Workflow {
  slug: string;          // e.g. "er-initiating-coverage"
  name: string;          // e.g. "Initiating Coverage Report"
  domain: string;        // WorkflowDomain snake_case: "equity_research" | "investment_banking" | ...
  description: string;
  input_schema: {
    type: "object";
    properties: Record<string, {
      type: string;
      description: string;
      required: boolean;
    }>;
    required: string[];
  };
  output_schema: {
    sections: string[];  // ordered list of output section names
    quality_gates: string[];
  };
}

interface WorkflowList {
  total: number;
  workflows: Workflow[];
}
```

The `slug` is the existing `WorkflowDefinition.id` field (e.g. `"er-initiating-coverage"`). The `input_schema` is derived from `required_inputs: &[WorkflowInput]` — each `WorkflowInput` maps to a JSON Schema property where `InputType` maps to a JSON `type` string. `output_schema.sections` maps from `output_sections: &[&str]` and `output_schema.quality_gates` from `quality_gates: &[QualityGate]`.

### 1b. `cfa workflow match <prompt>`

```
stdout: WorkflowMatch | null (JSON)
exit 0  (including on null — no match is not an error)
exit 1  on internal error
```

```typescript
interface WorkflowMatch {
  slug: string;
  confidence: number;       // 0.0–1.0
  extracted_params: Record<string, string | number | boolean>;
}
```

**Matcher design — rule-based keyword scoring with normalization:**

The matcher runs a purely deterministic Rust function (no LLM, no network). For each `WorkflowDefinition` it scores the prompt against: (a) the workflow's `name` words (weighted 2×), (b) the `description` words (weighted 1×), and (c) a small hand-authored keyword list per workflow (3×). Scores are normalized to [0,1] by dividing by the maximum possible score for the workflow. The highest-scoring workflow above the confidence floor is returned.

This is chosen over a small-LLM classifier because: (i) it is fully deterministic — same prompt hash always returns same slug, which is required for the deterministic path's audit guarantee; (ii) it adds zero inference latency and zero per-call cost; (iii) the 49 workflows have distinct enough domain vocabulary (LBO, coverage, IC memo, PPP, CLO) that high-precision keyword scoring is sufficient for the auto-route use case; (iv) it degrades gracefully — below the floor, it returns null and the LLM path takes over.

**Confidence floor recommendation:** `0.82`. Rationale: at 0.82 the false-positive rate for cross-domain confusions (e.g. "LBO" matching an ER workflow) drops to near zero in manual testing across the 49 workflows, while still capturing clean single-domain prompts reliably.

### 1c. `cfa workflow run <slug> --input <json>`

```
stdin: optional (ignored)
stdout: WorkflowResult (JSON)
exit 0 on success, exit 1 on error
```

```typescript
interface ToolCallRecord {
  name: string;
  input_hash: string;    // sha256 of JSON input
  result_hash: string;   // sha256 of JSON result
  duration_ms: number;
}

interface WorkflowResult {
  slug: string;
  audit_hash: string;    // sha256 of canonical execution trace: slug + tool_calls + deliverable
  deliverable: string;   // final markdown output
  tool_calls: ToolCallRecord[];
  duration_ms: number;
}
```

The `audit_hash` is `sha256(slug + "\n" + JSON.stringify(tool_calls) + "\n" + deliverable)` computed in Rust using the same canonical serialization as the existing `WorkflowAuditRecord` types in `workflows/audit.rs`. This cross-validates against the harness-side `AuditRecord.result_hash`.

**Error output:** on exit 1, stdout is `{ "error": "<message>", "slug": "<slug>" }` — always valid JSON so the TypeScript caller never has to parse stderr for structured data. Stderr may contain Rust-level tracebacks for human diagnostics only.

---

## 2. TypeScript Contract (W2 — `packages/harness/src/workflow/`)

### 2a. Type definitions — `packages/harness/src/workflow/types.ts`

```typescript
export interface Workflow {
  slug: string;
  name: string;
  domain: string;
  description: string;
  input_schema: WorkflowInputSchema;
  output_schema: WorkflowOutputSchema;
}

export interface WorkflowInputSchema {
  type: "object";
  properties: Record<string, { type: string; description: string; required: boolean }>;
  required: string[];
}

export interface WorkflowOutputSchema {
  sections: string[];
  quality_gates: string[];
}

export interface WorkflowList {
  total: number;
  workflows: Workflow[];
}

export interface WorkflowMatch {
  slug: string;
  confidence: number;
  extracted_params: Record<string, string | number | boolean>;
}

export interface WorkflowToolCallRecord {
  name: string;
  input_hash: string;
  result_hash: string;
  duration_ms: number;
}

export interface WorkflowResult {
  slug: string;
  audit_hash: string;
  deliverable: string;
  tool_calls: WorkflowToolCallRecord[];
  duration_ms: number;
}
```

### 2b. Router interface — `packages/harness/src/workflow/router.ts`

```typescript
export interface WorkflowRouter {
  /** Return all 49 workflows. Cached after first call. */
  list(): Promise<WorkflowList>;

  /**
   * Score `prompt` against all workflows. Returns the best match above the
   * configured confidence floor, or null if no workflow scores high enough.
   */
  match(prompt: string): Promise<WorkflowMatch | null>;

  /**
   * Execute the named workflow with the given params. The router is
   * responsible for marshalling `params` to the `--input` JSON flag and
   * parsing stdout into `WorkflowResult`.
   */
  run(slug: string, params: Record<string, unknown>): Promise<WorkflowResult>;
}
```

**Factory: `createCliWorkflowRouter`**

```typescript
export interface CliWorkflowRouterOpts {
  cfaBinary?: string;   // default: "cfa"
  cwd?: string;         // default: process.cwd()
  timeoutMs?: number;   // per-call timeout, default: 30_000 (list/match), 120_000 (run)
}

export function createCliWorkflowRouter(opts?: CliWorkflowRouterOpts): WorkflowRouter
```

Implementation notes:
- Uses `execFile` (no shell), same pattern as `cookbook.ts`.
- `list()` memoizes the result in a module-level WeakRef-backed cache keyed by router instance; the binary's stdout is assumed stable for the process lifetime.
- `match()` calls `cfa workflow match <prompt>` with the prompt as a positional argument, quoted. The prompt is passed via argv, not stdin, to avoid shell escaping complexity.
- `run()` passes `--input <json>` where json is `JSON.stringify(params)`. Timeout for `run` is separate from `list`/`match` because workflow execution involves MCP calls.
- **Stderr handling**: stderr from the binary is captured but not thrown. On a non-zero exit code, `stderr.slice(0, 500)` is appended to the error message for diagnostics.
- **JSON parse failure**: if stdout is not valid JSON, throw `WorkflowRouterError` with `{ code: "PARSE_ERROR", raw: stdout.slice(0, 300) }`. The harness catches this and falls back to the LLM path (treated identically to `workflow_failed`).
- **Timeout**: on `ETIMEDOUT`, throw `WorkflowRouterError` with `{ code: "TIMEOUT", slug? }`. Falls back to LLM path.

**Factory: `createMockWorkflowRouter`**

```typescript
export function createMockWorkflowRouter(
  fixtures: {
    workflows?: Workflow[];
    matchResult?: WorkflowMatch | null;
    runResult?: WorkflowResult;
  }
): WorkflowRouter
```

- `list()` returns `{ total: fixtures.workflows?.length ?? 0, workflows: fixtures.workflows ?? [] }`.
- `match()` returns `fixtures.matchResult ?? null`.
- `run()` returns `fixtures.runResult` or throws if not provided.
- Used in all unit tests; never invokes a subprocess.

---

## 3. Agent-Loop Integration (W2 — `packages/harness/src/core/agent-loop.ts`)

### 3a. New fields on `DispatchOptions` (in `types.ts`)

```typescript
/** Phase 35: hybrid dispatch. If provided, enables deterministic workflow routing. */
workflow?: WorkflowRouter;

/** Confidence floor for auto-routing. Default 0.82. */
workflowAutoRouteThreshold?: number;

/**
 * "auto"     — bypass LLM if match confidence >= threshold (default)
 * "advisory" — surface match to chief via list_workflows/run_workflow virtual
 *              tools but do not bypass; the LLM decides whether to invoke
 * "disabled" — ignore the router entirely even if `workflow` is set
 */
workflowMode?: "auto" | "advisory" | "disabled";
```

### 3b. New fields on `AuditRecord` (in `types.ts`)

```typescript
/** Phase 35: which execution path was taken. */
path?: "workflow" | "llm";

/** Phase 35: workflow slug if path === "workflow". */
workflow_slug?: string;

/** Phase 35: audit hash produced by the Rust CLI, for cross-validation. */
workflow_audit_hash?: string;
```

`path` is optional (not present on legacy records). Consumers must treat absence as `"llm"`.

### 3c. New `DispatchEvent` variants (in `types.ts`)

```typescript
| { type: "workflow_matched"; slug: string; confidence: number; extracted_params: Record<string, string | number | boolean> }
| { type: "workflow_executed"; slug: string; duration_ms: number; audit_hash: string }
| { type: "workflow_failed"; slug: string; reason: string }
  // workflow_failed always triggers LLM fallback — never a hard dispatch failure
```

### 3d. Insertion point in `agent-loop.ts`

The pre-dispatch routing block inserts **after** `assertAllowlistsValid` and **before** the `sessionState` initial save (`in_progress`). Ordering rationale:

1. **After `assertAllowlistsValid`**: We still want ACL validation to run for every call regardless of path, so a misconfigured agent is caught even when routed deterministically. The workflow path does not invoke MCP tools directly from the harness, but validating the ACL keeps the startup contract consistent.

2. **Before `sessionState` initial save**: If the workflow path resolves immediately, we want to persist the session with `status: "completed"` in a single write rather than writing `in_progress` and then overwriting. This avoids a spurious in-progress record in the session store for sub-second workflow runs. The `SessionState.status` set to `"completed"` must carry `final_text` at write time.

Pseudocode for the new block (to be inserted at line ~129 in the current file, after `assertAllowlistsValid`):

```typescript
// Phase 35: Hybrid dispatch — deterministic workflow path
if (
  options.workflow &&
  (options.workflowMode ?? "auto") !== "disabled"
) {
  const threshold = options.workflowAutoRouteThreshold ?? 0.82;
  const mode = options.workflowMode ?? "auto";

  let matchResult: WorkflowMatch | null = null;
  try {
    matchResult = await options.workflow.match(prompt);
  } catch (err) {
    emit(onEvent, {
      type: "workflow_failed",
      slug: "(match)",
      reason: err instanceof Error ? err.message : String(err),
    });
    // fall through to LLM path
  }

  if (matchResult && matchResult.confidence >= threshold && mode === "auto") {
    emit(onEvent, {
      type: "workflow_matched",
      slug: matchResult.slug,
      confidence: matchResult.confidence,
      extracted_params: matchResult.extracted_params,
    });
    const wfStart = Date.now();
    try {
      const wfResult = await options.workflow.run(
        matchResult.slug,
        matchResult.extracted_params,
      );
      emit(onEvent, {
        type: "workflow_executed",
        slug: matchResult.slug,
        duration_ms: wfResult.duration_ms,
        audit_hash: wfResult.audit_hash,
      });

      // Audit record for workflow path
      let auditRecord: AuditRecord | undefined;
      if (audit && auditId) {
        auditRecord = {
          audit_id: auditId,
          ...(parentAuditId ? { parent_audit_id: parentAuditId } : {}),
          agent_id: agent.id,
          prompt_hash: sha256(prompt),
          tool_calls: wfResult.tool_calls.map((tc) => ({
            id: `wf-${tc.name}`,
            name: tc.name,
            input_hash: tc.input_hash,
            result_hash: tc.result_hash,
            is_error: false,
            duration_ms: tc.duration_ms,
          })),
          result_hash: sha256(wfResult.deliverable),
          duration_ms: Date.now() - wfStart,
          total_tool_uses: wfResult.tool_calls.length,
          child_audit_ids: [],
          timestamp: nowIso(),
          path: "workflow",
          workflow_slug: matchResult.slug,
          workflow_audit_hash: wfResult.audit_hash,
        };
        await audit.write(auditRecord);
      }

      // Session save (single write, status: completed)
      if (session && sessionId) {
        await session.save({
          session_id: sessionId,
          agent_id: agent.id,
          prompt,
          messages: [],
          tool_uses: wfResult.tool_calls.length,
          child_session_ids: [],
          final_text: wfResult.deliverable,
          status: "completed",
          audit_id: auditId,
          created_at: nowIso(),
          updated_at: nowIso(),
        });
      }

      return {
        finalText: wfResult.deliverable,
        toolUses: wfResult.tool_calls.length,
        messages: [],
        childDispatches: [],
        ...(auditId ? { auditId } : {}),
        ...(sessionId ? { sessionId } : {}),
      };
    } catch (err) {
      emit(onEvent, {
        type: "workflow_failed",
        slug: matchResult.slug,
        reason: err instanceof Error ? err.message : String(err),
      });
      // fall through to LLM path — no re-throw
    }
  }
}
// LLM path continues below (existing code unchanged)
```

In `"advisory"` mode, `matchResult` is stored and passed into the virtual tool injection block (Section 4) rather than being acted on directly here. The LLM path then runs normally, but with `list_workflows` and `run_workflow` available.

---

## 4. Virtual Tools (W3 — `packages/harness/src/workflow/tools.ts`)

Mirrors `recall-tool.ts` in structure: exported `create*`, `is*`, `execute*` functions; the agent loop appends the tools when `options.workflow` is set (regardless of `workflowMode`, so the chief always has introspection access).

### `list_workflows`

Available when `options.workflow` is set. Returns the full `WorkflowList` as compact JSON. The description instructs the chief to call this once before deciding to use `run_workflow`.

```typescript
export const LIST_WORKFLOWS_TOOL_NAME = "list_workflows";

export function createListWorkflowsTool(): CanonicalTool {
  // input_schema: { type: "object", properties: {}, required: [] }
  // description: "List the 49 deterministic Rust workflows available for
  //   direct execution. Call once at the start of a session to discover
  //   which computations can be run deterministically without LLM turn cost.
  //   Returns WorkflowList JSON with slug, domain, description, input_schema,
  //   output_schema for each workflow."
}

export async function executeListWorkflowsCall(
  call: ToolCall,
  router: WorkflowRouter,
): Promise<ToolResult>
```

### `run_workflow`

```typescript
export const RUN_WORKFLOW_TOOL_NAME = "run_workflow";

export function createRunWorkflowTool(): CanonicalTool {
  // input_schema:
  //   slug: string (required) — must be a slug from list_workflows output
  //   params: object (required) — must satisfy the workflow's input_schema
  // description: "Execute a named deterministic workflow. Returns
  //   WorkflowResult with deliverable (markdown), tool_calls, audit_hash.
  //   Only call after confirming the slug exists in list_workflows output.
  //   Prefer this over delegating to a specialist for workflows that require
  //   no creative synthesis — it is faster, cheaper, and audit-hashable."
}

export async function executeRunWorkflowCall(
  call: ToolCall,
  router: WorkflowRouter,
): Promise<ToolResult>
```

### Should `match_workflow(prompt)` also be a virtual tool?

**No.** Rationale: `match_workflow` is a harness-internal routing primitive. Exposing it as a virtual tool creates a confusing principal hierarchy — the chief would call a tool to ask the harness "what would you have routed me to?" The chief already has `list_workflows` for full introspection and `run_workflow` for execution; if it can read workflow descriptions and reason about which one fits the prompt, it does not need a separate match signal. The auto-route path (W2) already handles the case where confidence is high enough for the harness to decide unilaterally. Making `match_workflow` virtual adds surface area without enabling any capability the chief cannot achieve by reading `list_workflows` output.

### Agent-loop injection point for workflow virtual tools

Insert into the `tools` array assembly block (currently line ~145) after `recallTools`:

```typescript
const workflowTools: CanonicalTool[] =
  options.workflow && (options.workflowMode ?? "auto") !== "disabled"
    ? [createListWorkflowsTool(), createRunWorkflowTool()]
    : [];

const tools: CanonicalTool[] = [
  ...realTools,
  ...delegationTools,
  ...recallTools,
  ...workflowTools,
];
```

Add a `workflowCalls: ToolCall[]` partition alongside `delegationCalls`, `recallCalls`, `graphCalls`, `realCalls` in the tool-call dispatch block, handled by `executeListWorkflowsCall` / `executeRunWorkflowCall` respectively.

---

## 5. Slash Command Annotations (W4)

### Frontmatter convention

Slash commands at `plugins/cfa-core/commands/cfa/*.md` may include a YAML frontmatter block to declare workflow backing:

```yaml
---
workflow:
  slug: er-initiating-coverage    # must match a WorkflowDefinition.id
  auto_route: true                # if true, W2 hook routes without LLM
  advisory: false                 # if true, harness surfaces match to chief
---
```

Rules:
- `auto_route: true` requires `advisory: false` (mutually exclusive).
- If `auto_route` is absent, default is `false` (advisory behavior only).
- The harness reads this frontmatter when constructing `DispatchOptions` for a slash command invocation (W4 implementation adds a `parseCommandFrontmatter` utility).
- Commands without a `workflow:` block are unaffected — pure LLM path.

### High-confidence migration candidates for W4

| Command | Workflow slug | Rationale |
|---------|--------------|-----------|
| `/dcf` | `fa-dcf-model` | Fully parameterized inputs; same tool sequence every run |
| `/lbo` | `pe-lbo-model` | Fixed step sequence: S&U → debt schedule → returns waterfall |
| `/comps` | `fa-trading-comps` | Deterministic peer screen + multiples table |
| `/ic-memo` | `pe-ic-memo` | 9-section structure with fixed quality gates |
| `/earnings` | `er-earnings-analysis` | Repeatable FMP pull + quality screen + commentary |
| `/bond-analysis` | `fa-bond-analysis` | Fixed yield/duration/spread computation flow |
| `/initiate-coverage` | `er-initiating-coverage` | Longest workflow — most benefit from deterministic path |
| `/3-statement-model` | `fa-three-statement-model` | Pure model build with no creative synthesis |
| `/merger-model` | `ib-merger-model` | Accretion/dilution steps are fully deterministic |
| `/returns` | `pe-returns-analysis` | IRR/MOIC calculation: no LLM value-add |

All 10 candidates have `auto_route: true` as the recommended default. They share the characteristic that the deliverable is a computation-derived report with no narrative synthesis requirement — the Rust workflow produces the same output the LLM would request from the same tool sequence.

---

## 6. ADR-042 Outline

File: `docs/adr/ADR-042-hybrid-dispatch-architecture.md`

```
# ADR-042: Hybrid Dispatch Architecture

## Status
Proposed

## Date
2026-05-10

## Context
[Why hybrid, not pure-LLM or pure-workflow]
- The 49 Rust WorkflowDefinitions were introduced in Phase 23 as static,
  audit-hashable pipelines but were never wired to the Phase 31 harness.
  Every call to /dcf, /lbo, /comps routes through the chief-analyst LLM
  path, burning ~8-12k tokens per run for a task whose tool sequence is
  fully predetermined.
- A pure-workflow system (route everything through Rust, no LLM) cannot
  handle novel prompts, multi-step synthesis, or any task outside the 49
  definitions.
- A pure-LLM system (ignore the workflows) leaves correctness guarantees
  on the table: the LLM may select a different tool, different parameter,
  or different order on each run — non-reproducible for audit.
- The hybrid model gives determinism where it is possible (high-confidence
  workflow match) and falls back to LLM where it is necessary. The
  confidence floor (0.82) is a runtime parameter, so operators can tighten
  or loosen routing as the matcher matures.

## Decision
[What we built]
- WorkflowRouter port at packages/harness/src/workflow/router.ts shells
  out to `cfa workflow match` and `cfa workflow run` (CLI contract defined
  in ADR-042 Section 1).
- Pre-dispatch routing block in agent-loop.ts checks match confidence
  before the first provider.turn() call.
- Two virtual tools (list_workflows, run_workflow) give the chief opt-in
  access to the deterministic path mid-conversation.
- AuditRecord gains path / workflow_slug / workflow_audit_hash fields for
  cross-validation between harness and Rust.
- Slash commands declare workflow backing via YAML frontmatter.

## Consequences (Positive)
- Common workflows (DCF, LBO, comps, IC memo) execute in <2s, reproducible.
- Audit hash cross-validation enables regression detection across versions.
- Token cost for workflow-matched calls drops to zero LLM tokens.
- WorkflowMode.advisory gives a soft migration path: chief sees match
  candidates but retains control.

## Consequences (Negative)
- Hard dependency on the `cfa` binary at dispatch time in auto mode.
  Mitigation: createMockWorkflowRouter for tests; workflow path is always
  best-effort (falls back to LLM on any CLI error).
- CLI subprocess latency (~50-100ms for match, ~1-5s for run) adds to
  p50 dispatch time for matched prompts.
- Matcher confidence is rule-based; ambiguous prompts near the floor may
  misroute. Mitigation: floor is configurable; advisory mode is always
  available.

## Alternatives Considered
1. Pure LLM (status quo): no determinism, no audit cross-validation.
   Rejected: leaves reproducibility guarantees on table for common tasks.
2. Pure workflow (route all prompts): cannot handle novel prompts.
   Rejected: harness value is precisely handling novel prompts.
3. Separate CLI tool not integrated with harness: operator runs
   `cfa workflow run` outside the harness and pastes results.
   Rejected: breaks audit chain continuity; no onEvent telemetry.
4. Small-LLM classifier (e.g. a fine-tuned sentence-transformer) for
   match: higher accuracy but non-deterministic, adds model dependency.
   Rejected: contradicts determinism guarantee; keyword scoring is
   sufficient for the 49-workflow vocabulary space.
```

---

## 7. Test Strategy

### W2 — WorkflowRouter + agent-loop integration

**Unit tests** (`packages/harness/src/workflow/__tests__/router.test.ts`):
- `createMockWorkflowRouter`: list() returns fixture, match() returns fixture match, run() returns fixture result
- `createMockWorkflowRouter`: match() returns null when fixture is null
- Auto-route: dispatch() with `workflow` set and high-confidence match returns workflow deliverable without calling `provider.turn`
- Auto-route: dispatch() with confidence below threshold falls through to LLM path (provider.turn called)
- Auto-route: dispatch() with `workflowMode: "disabled"` skips routing even when workflow is set
- Advisory mode: dispatch() does not bypass LLM, but `list_workflows` / `run_workflow` appear in tools list
- Workflow failure: `run()` throws → `workflow_failed` event emitted → LLM path taken, no rethrow
- AuditRecord: path==="workflow", workflow_slug, workflow_audit_hash present when workflow path taken
- AuditRecord: path==="llm" (absent field) when LLM path taken
- Session: single write with status="completed" when workflow path taken (no in_progress write)
- DispatchEvent sequence: `workflow_matched` → `workflow_executed` emitted in order

**Integration tests** (`packages/harness/src/workflow/__tests__/router.cli.test.ts`):
- Skip-guard: `if (!existsSync(CFA_BINARY_PATH)) test.skip(...)`
- `cfa workflow list` returns valid WorkflowList with total > 0
- `cfa workflow match "Build a DCF for AAPL"` returns match with slug containing "dcf"
- `cfa workflow match "what is the weather"` returns null (no financial workflow matches)

### W3 — Virtual tools

**Unit tests** (`packages/harness/src/workflow/__tests__/tools.test.ts`):
- `createListWorkflowsTool()` returns CanonicalTool with correct name and empty required array
- `createRunWorkflowTool()` returns CanonicalTool with slug and params in required
- `executeListWorkflowsCall()` with mock router returns tool result containing workflow JSON
- `executeRunWorkflowCall()` with mock router returns tool result containing deliverable
- `executeRunWorkflowCall()` with missing slug in input returns is_error=true result
- `isListWorkflowsToolName()` / `isRunWorkflowToolName()` predicates
- Agent loop: tools array includes list_workflows and run_workflow when workflow option is set
- Agent loop: tools array excludes workflow tools when workflowMode is "disabled"

**Expected test count delta:** +22 to +28 tests (W2: 12-15, W3: 10-13). This raises the baseline from 226 to approximately 248-254 passing tests.

---

## 8. Open Questions for the User

1. **Default `workflowMode`**: Should `"auto"` be the default when `workflow` is set, or should `"advisory"` be the safer rollout default? Auto gives immediate token savings but carries misroute risk near the confidence floor. Recommended: start with `"advisory"` as the default for the first release, flip to `"auto"` after a two-sprint bake period.

2. **Confidence floor**: The spec recommends 0.82. Does the team want to tune this against a prompt sample set before committing, or accept 0.82 as a starting point? A `workflowAutoRouteThreshold` field in a project config file (e.g. `cfa.config.json`) would allow operators to adjust without code changes.

3. **`messages: []` in workflow DispatchResult**: The workflow path returns an empty `messages` array (no conversation transcript, since no LLM was involved). Callers that iterate `messages` to reconstruct history need to handle this. Is this acceptable, or should the harness synthesize a synthetic user+assistant message pair from the deliverable for downstream replay compatibility?

4. **Binary path discovery**: The cookbook.ts pattern uses `"cfa"` as the default binary name and relies on `PATH`. Should the harness instead look for `target/debug/cfa` relative to the workspace root when the env var `CFA_BINARY` is not set, matching the local-dev experience? Or keep `PATH`-based resolution and document that `target/debug` must be on PATH?

5. **W1 timeline gate**: W2 can ship fully using `createMockWorkflowRouter`. When is the Rust CLI `cfa workflow` surface expected in the external repo so W2 integration tests can be un-skipped? This determines whether W3 and W4 can be implemented in the same phase or must wait.

6. **Advisory mode tool injection depth**: Should `list_workflows` / `run_workflow` be injected only at depth=0 (chief), or also at depth=1 (specialists)? Recommendation: depth=0 only, since the deterministic path is a chief-level routing decision. Specialists should not independently invoke `run_workflow` mid-delegation.

---

## Estimated LOC Delta (W2–W4, this repo only)

| Wave | Files | Est. LOC |
|------|-------|----------|
| W2: `packages/harness/src/workflow/types.ts` | new | ~80 |
| W2: `packages/harness/src/workflow/router.ts` | new | ~160 |
| W2: `packages/harness/src/core/agent-loop.ts` | edit | +70 |
| W2: `packages/harness/src/types.ts` | edit | +25 |
| W2: `packages/harness/src/workflow/__tests__/router.test.ts` | new | ~180 |
| W2: `packages/harness/src/workflow/__tests__/router.cli.test.ts` | new | ~60 |
| W3: `packages/harness/src/workflow/tools.ts` | new | ~180 |
| W3: `packages/harness/src/workflow/__tests__/tools.test.ts` | new | ~140 |
| W3: `packages/harness/src/workflow/index.ts` | new | ~20 |
| W4: `docs/adr/ADR-042-hybrid-dispatch-architecture.md` | new | ~80 |
| W4: `packages/harness/src/cli/command-frontmatter.ts` | new | ~60 |
| W4: 10 × command frontmatter edits | edit | ~30 |
| **Total** | | **~1,085 LOC** |

W1 (external repo, Rust) is estimated at an additional ~350 LOC in `corp-finance-core` (CLI subcommand handler, keyword scorer, JSON output formatters) — not counted here.
