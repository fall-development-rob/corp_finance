# ADR-042: Hybrid Deterministic/LLM Dispatch

## Status: Accepted

## Date: 2026-05-10

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`harness`, `dispatch`, `workflow-router`, `determinism`, `audit`, `phase-35`

## Context

Phases 31 through 34 built a capable LLM dispatch harness: a chief-analyst
orchestrator (Phase 31), skill-driven specialists (Phase 33, ADR-031), and
a semantic reasoning bank for cross-dispatch learning (Phase 34, ADR-040).
Every prompt — regardless of how well-defined its tool sequence is — routes
through the chief-analyst LLM turn and then down to a specialist. For a DCF
model, an LBO, or a trading comps table, that turn costs 8-12k tokens and
produces a tool sequence that is structurally identical to the one the
previous run produced.

The harness has never used the 44 pre-defined `WorkflowDefinition`s that
ship in `crates/corp-finance-core/src/workflows/` (external repo). These
workflows were designed in Phase 23 to be byte-deterministic and
audit-hashable: given identical inputs they execute an identical ordered
tool sequence and produce an identical sha256 audit hash. The Phase 31
harness was built with LLM dispatch as the only path; the workflow
definitions were left as static metadata with no runtime consumer.

The result is a correctness gap that compounds with use: the audit chain
captures what the LLM chose to do, but cannot guarantee it will make the
same choices next time. For routine deliverables — IC memo, DCF, LBO,
comps — this non-reproducibility is unacceptable for clients who expect
the same structural output each run and for compliance teams who want a
cross-validatable audit hash.

A pure-workflow system (route all prompts to Rust, bypass LLM entirely)
is not a solution: the harness's value is precisely its ability to handle
novel prompts, multi-step synthesis, and tasks outside the 49 workflow
definitions. Replacing LLM dispatch with workflow-only dispatch defeats
the design.

Phase 35 introduces a hybrid execution model that gives determinism where
it is achievable and falls back to LLM where it is necessary.

## Decision

### 1. Pre-dispatch routing step

The agent loop gains a routing block inserted after `assertAllowlistsValid`
and before the first `provider.turn()` call. The block:

1. Calls `options.workflow.match(prompt)` — a deterministic Rust keyword
   scorer (no LLM, no network) that returns the best-matching workflow slug
   and a normalized confidence score in [0, 1], or null if no match clears
   the floor.
2. If `workflowMode === "auto"` and confidence >= threshold (default 0.82),
   calls `options.workflow.run(slug, params)` and returns the result
   directly, bypassing the LLM path entirely.
3. If the workflow run fails (CLI error, timeout, parse failure), emits a
   `workflow_failed` event and falls through to the LLM path without
   re-throwing. The fallback is always best-effort.

### 2. Three workflow modes

`DispatchOptions.workflowMode` controls harness behaviour:

- **`"auto"`** — If match confidence >= threshold, the harness bypasses the
  LLM and executes the workflow. The user gets a deterministic result with
  no LLM token cost. Correct for production batch jobs, audit-sensitive
  deliverables, and CI regression testing.

- **`"advisory"`** — The harness never auto-routes. It exposes
  `list_workflows` and `run_workflow` as virtual tools injected into the
  chief's tool list. The chief can call `run_workflow` mid-conversation when
  it judges a deterministic path appropriate. The LLM retains full control.

- **`"disabled"`** — The router is ignored entirely even when
  `options.workflow` is set. Useful for debugging, regression isolation, and
  legacy compatibility.

**`"advisory"` is the correct default.** The reasoning bank (Phase 34) and
skill-driven specialists (Phase 33) give the LLM path strong quality
guarantees. Auto-routing at the confidence floor carries misroute risk for
ambiguous prompts. The conservative rollout default is to surface match
candidates to the chief — who can read `list_workflows` output and make a
reasoning-visible decision — rather than bypass it unilaterally. Once the
keyword matcher has been validated against a real-world prompt sample set
and the false-positive rate near 0.82 is measured, operators can flip
to `"auto"` for individual commands or globally.

### 3. WorkflowRouter interface and CLI contract

`WorkflowRouter` at `packages/harness/src/workflow/router.ts` wraps the
`cfa` Rust binary:

- `list()` — shells out to `cfa workflow list`, returns `WorkflowList`.
  Result is memoized in a module-level WeakRef-backed cache for the process
  lifetime.
- `match(prompt)` — shells out to `cfa workflow match <prompt>`, returns
  `WorkflowMatch | null`. Purely deterministic: same prompt always returns
  same result.
- `run(slug, params)` — shells out to `cfa workflow run <slug> --input
  <json>`, returns `WorkflowResult` with `deliverable`, `tool_calls`,
  `audit_hash`, `duration_ms`.

A `createMockWorkflowRouter` factory is provided for tests; it never
invokes a subprocess and satisfies the same `WorkflowRouter` interface.
All unit tests use the mock; integration tests skip when the `cfa` binary
is absent.

### 4. Audit chain extensions

`AuditRecord` gains three optional fields:

- `path: "workflow" | "llm"` — which execution path was taken. Absent on
  legacy records; consumers must treat absence as `"llm"`.
- `workflow_slug` — slug of the matched workflow when `path === "workflow"`.
- `workflow_audit_hash` — sha256 produced by the Rust CLI, enabling
  cross-validation between the harness record and the CLI output.

On the workflow path, the session is persisted in a single write with
`status: "completed"` (no spurious `in_progress` write for sub-second
runs). The `messages` array is empty; `final_text` carries the deliverable.

Three new `DispatchEvent` variants provide telemetry:

- `workflow_matched` — slug, confidence, extracted_params
- `workflow_executed` — slug, duration_ms, audit_hash
- `workflow_failed` — slug, reason (always triggers LLM fallback)

### 5. Virtual tools

When `options.workflow` is set and `workflowMode !== "disabled"`, two
virtual tools are injected into the chief's tool list (depth=0 only;
specialists do not receive them):

- `list_workflows` — returns the full `WorkflowList` as compact JSON.
  The chief calls this once to discover deterministic paths available.
- `run_workflow` — executes a named workflow by slug with given params.
  The description instructs the chief to prefer this over LLM delegation
  for computation-derived reports that require no creative synthesis.

`match_workflow` is deliberately not exposed as a virtual tool. It is a
harness-internal routing primitive; the chief does not need it because
`list_workflows` gives full introspection and `run_workflow` gives
execution. Exposing the match signal would create a confusing principal
hierarchy where the chief queries the harness about what the harness
would have done.

### 6. Slash command frontmatter annotations

Slash commands that back pre-defined workflows declare workflow metadata
in their YAML frontmatter:

```yaml
workflow:
  slug: fa-dcf-model         # must match a WorkflowDefinition.id
  auto_route: true           # opt into auto-route for this command
  advisory: false            # advisory: false required when auto_route: true
```

A `parseCommandFrontmatter` utility in the harness reads this block when
constructing `DispatchOptions` for a slash command invocation. Commands
without a `workflow:` block are unaffected (pure LLM path). Slug-to-
workflow existence is validated at build time in W1 (the `cfa` CLI surface
in the external `corp-finance-core` repo); invalid slugs surface as a
build error, not a runtime error.

## Consequences

### Positive

- Common deliverables (DCF, LBO, comps, IC memo, bond analysis, earnings
  updates) execute deterministically: same input hash → same audit hash →
  same output, every run.
- LLM token cost for workflow-matched calls drops to zero. At the `"auto"`
  threshold, a DCF or LBO that previously cost ~10k tokens now costs 0.
- The audit chain records `path: "workflow"` per dispatch, making
  determinism rate measurable over time. Operators can track what fraction
  of production dispatches are taking the deterministic path.
- `"advisory"` default provides a no-risk migration path: the chief gains
  `list_workflows` / `run_workflow` but retains full decision authority.
  Production behaviour is unchanged unless the chief explicitly calls
  `run_workflow`.
- Skills remain as prose documentation. Phase 33's skill bodies are
  unchanged. Behaviour is added by the engine and router, not by editing
  skill prose.
- `createMockWorkflowRouter` ensures the entire workflow path is fully
  unit-testable without the Rust binary.
- The reasoning bank (ADR-040) coexists without conflict: `recall_similar`
  and `recall_by_graph` operate on prior LLM dispatches; the workflow path
  bypasses both but writes its own audit record. Both banks grow
  independently and serve distinct recall surfaces.

### Negative

- Adds a runtime dependency on the `cfa` Rust binary for the workflow path.
  Mitigation: the harness gracefully degrades to LLM-only when the binary
  is absent or returns a non-zero exit; `createMockWorkflowRouter` fully
  substitutes in tests. Binary discovery uses `PATH` by default; the env
  var `CFA_BINARY` overrides for local development against `target/debug/`.
- Cross-repo coupling: keyword matcher and `WorkflowDefinition` metadata
  live in the external `corp-finance-core` repo. Adding a new workflow or
  updating a keyword list requires a coordinated release across both repos.
  A stale binary can serve an outdated matcher against newer harness slugs.
  Mitigation: semantic versioning + integration test skip-guard.
- The workflow path and LLM path produce structurally different
  `DispatchResult` shapes: the workflow path returns `messages: []` and
  populates only `finalText`. Session replay consumers that iterate
  `messages` to reconstruct history must handle the empty-messages case.
  This is a known open question documented in the architect plan (§8.3);
  synthetic message synthesis is deferred to a follow-on phase if replay
  compatibility is required.
- CLI subprocess latency (~50-100ms for `match`, ~1-5s for `run` including
  MCP calls) adds to p50 dispatch time on the workflow path. This is
  acceptable because the alternative (LLM turn) is 5-30s; the workflow
  path is always faster end-to-end.
- The keyword matcher is rule-based. Prompts near the confidence floor
  (0.82) may misroute in `"auto"` mode. Mitigation: floor is a runtime
  parameter (`workflowAutoRouteThreshold`), configurable per project in
  `cfa.config.json` without code changes. The `"advisory"` default
  eliminates this risk for the first release.

### Neutral

- The 44-workflow vocabulary space (DCF, LBO, LBO-returns, coverage, IC
  memo, CLO, bond, comps, merger, three-statement) has sufficiently distinct
  domain keywords that the 0.82 floor produces near-zero false positives in
  manual testing. Cross-domain confusions (e.g. "LBO" matching an ER
  workflow) do not arise at this threshold.
- `"auto"` is available today and can be enabled per-command via the slash
  command frontmatter (`auto_route: true`) without waiting for a global
  mode flip. The 10 annotated commands in W4 are the first cohort.

## Alternatives Considered

**Pure LLM dispatch (status quo through Phase 34)** — Rejected. The harness
burns 8-12k tokens per routine deliverable and produces non-reproducible
results. For clients requiring audit-hashable outputs this is a compliance
gap, not a style preference.

**Pure workflow dispatch (no LLM at all)** — Rejected. The 44 workflows cover
well-defined computations but cannot handle novel multi-step synthesis,
exploratory prompts, or tasks outside the defined vocabulary. Replacing LLM
dispatch with workflow-only dispatch defeats the purpose of the harness.

**Separate CLI tool not integrated with the harness** — Rejected. Running
`cfa workflow run dcf-valuation` outside the harness breaks audit chain
continuity (no `AuditRecord` written, no `DispatchEvent` emitted, no session
state updated), presents two surfaces for users to learn, and severs the
graceful-fallback chain. Integration in the agent loop is the only design
that keeps the audit record complete.

**LLM-only with reasoning bank fallback (Phase 34 alone)** — Rejected.
`recall_similar` retrieves semantically similar prior dispatches and gives
the chief a starting point, but does not make execution deterministic.
Two runs of the same DCF prompt may still invoke different tools in a
different order if the chief's tool selection varies. The reasoning bank and
the workflow router are complementary, not alternatives.

**Small-LLM classifier for matching** — Rejected. A fine-tuned sentence-
transformer or similar model would achieve higher recall near the floor but
is itself non-deterministic across versions, adds a model dependency, and
contradicts the determinism guarantee that motivates the workflow path in
the first place. The keyword scorer is sufficient for the 49-workflow
vocabulary and produces identically reproducible routing decisions.

## Links

- Full design: `docs/plans/phase-35-hybrid-dispatch.md`
- Harness pre-dispatch block: `packages/harness/src/core/agent-loop.ts` (~line 129 post-Phase-35)
- Router implementation: `packages/harness/src/workflow/router.ts`
- Virtual tools: `packages/harness/src/workflow/tools.ts`
- Workflow definitions (external): `crates/corp-finance-core/src/workflows/`
- Slash command frontmatter: `plugins/cfa-core/commands/cfa/*.md` (W4 annotated cohort)
- Depends on: ADR-031 (skill-driven CFA specialists) — skill bodies are unchanged by Phase 35
- Depends on: ADR-040 (RuVector reasoning bank) — coexists; distinct concern
- Companion: ADR-041 (indexer hook in the agent loop) — indexing runs on LLM-path dispatches only
- External repo: `corp-finance-core` — W1 CLI surface (`cfa workflow list/match/run`)
