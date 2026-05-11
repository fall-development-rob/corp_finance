# ADR-044: Phase 41 — Deterministic Closed Learning Loop

- **Status**: Accepted
- **Date**: 2026-05-11
- **Deciders**: Robert Fall, CFA Agent platform engineering
- **Tags**: phase-41, learning-loop, determinism, closed-loop

---

## Context

### Background — The Reasoning Bank as a Passive Recorder

Phase 34 (ADR-040) built the RuVector reasoning bank: every chief-analyst dispatch
is indexed with a prompt embedding, linked to an `AuditRecord` via `audit_id`, and
queryable via `recallSimilar` and `recallByGraph`. Phase 37 (Waves 1-2) introduced
the output_schema validator, which rejects subagent responses that do not conform to
their declared schema and marks `validation_failed: true` on the corresponding
`ReasoningEntry`. Phase 39 added manifest linter strict mode, which validates every
YAML field in a cookbook or plugin manifest against a fixed schema.

Despite this observability infrastructure, nothing reads the bank to improve the
system. The bank is a passive recorder. No automated process analyses outlier
dispatches, surfaces recurring failure modes, or proposes corrections to skill prose.

### The Original Phase 41 Design

The original design (documented in `docs/plans/archive/phase-41-original-llm-driven.md`)
closed the loop by running three LLM subagents inside a `skill-editor` cookbook:

1. **outlier-detector** — queried the reasoning bank and emitted a structured
   `OutlierReport` (schema-validated JSON).
2. **pattern-analyst** — read the current `SKILL.md` prose for the affected skill
   and produced a `proposed_addition` field of up to 800 characters of free prose,
   identifying what section was missing and what text should be added.
3. **proposal-writer** — took the `PatternAnalysis` result and wrote two files:
   a unified diff (`.diff`) and a machine-readable metadata sidecar (`.json`).

The outlier-detector subagent was read-only and its output was fully schema-validated.
The proposal-writer was write-guarded to `docs/proposed-skill-updates/` only. The
structural safety properties of both were sound.

### Why the Design Was Rejected

The problem was the `pattern-analyst` subagent. Its `proposed_addition` field is
LLM-generated free prose. Two runs of the skill-editor cookbook against the same
reasoning bank state and the same `SKILL.md` file produce different diffs because
`proposed_addition` is nondeterministic across LLM invocations.

The diffs are written to `docs/proposed-skill-updates/` and, after human approval,
applied to version-controlled `SKILL.md` files via `git apply`. If the proposal
content is nondeterministic:

- Version control history is unstable: the same failure cluster produces different
  patch content each run, making it impossible to compare proposals across weeks.
- Reviewability is degraded: reviewers cannot distinguish between two runs that
  analysed the same cluster but produced different prose.
- Auditability breaks: the chain-of-custody claim ("this patch was produced
  deterministically from these audit IDs") is false if any prose was synthesised
  by an LLM.

This is the same category of failure that prompted the output_schema validator in
Phase 39: unstructured LLM output is not a reliable mechanical input to a downstream
system. The original design applied the lesson to subagent response validation but
missed that the proposal file itself was unstructured LLM output being committed to
version control.

The user's architectural review on 2026-05-11 identified this as a determinism
violation: "this no longer seems deterministic." The design was rejected and archived.

### The Determinism Invariant

Every component in the production path that writes to version-controlled files is
byte-deterministic: given identical inputs it produces identical outputs. The manifest
linter validates YAML with a fixed schema and fixed rules. The output_schema validator
accepts or rejects a subagent response using a compiled JSON Schema. The hybrid
dispatcher (ADR-042) routes to the deterministic workflow path or LLM path based on
a deterministic keyword scorer. No component in the production path synthesises free
prose at runtime and writes it to a versioned file. Phase 41 must uphold this invariant
end-to-end.

---

## Decision

All of Phase 41's proposal and apply paths are pure code. No LLM subagents exist in
the proposal or apply path. The three original LLM subagents are deleted; the
`skill-editor` cookbook and `plugins/agent-plugins/skill-editor/` plugin are removed
by parallel work on this branch.

### Component Architecture

```
Reasoning Bank (Phase 34)
  └── recallByGraph() / recallSimilar()
            │
      outliers.ts ──── detectAllOutliers()
      (W2, pure TS)    (4 pure query functions, no LLM)
            │
      OutlierReport (typed struct, schema-validated)
            │
      remediation-emitter.ts ──── W3 (pure fn)
      (OutlierReport → YAML[])
            │
      docs/proposed-skill-updates/
        <timestamp>-<cluster-id>.yaml   ← structured remediation YAML
            │
      cfa-harness skill-editor apply <file>.yaml ── W3 (deterministic edit)
            │
      SKILL.md / manifest YAML
      (version-controlled; human PR review before merge)
```

Every arrow in this pipeline is a pure function or a deterministic file operation.
The loop is closed without any LLM in the proposal or apply path.

### Remediation YAML: A Discriminated Union

The emitter (`remediation-emitter.ts`) maps each `OutlierCluster.recommended_action`
value to exactly one of four YAML types. All four types share a common header
(`cluster_id`, `affected_skill`, `motivating_audit_ids`, `confidence_score`). Each
type's `change` block carries only fields already present in the `OutlierCluster`
struct or references to version-controlled template IDs. No prose is synthesised.

**Type 1 — `add-skill-section`**: references a `section_body_template_id` from the
finite library under `docs/skill-editor-templates/add-skill-section/`. The apply CLI
looks up the template by ID and inserts its body verbatim. String fields are
constrained: `section_title` is an enum of five canonical values; `template_id` matches
`^[a-z0-9][a-z0-9-]{2,62}[a-z0-9]$` with `maxLength: 64`.

**Type 2 — `tighten-output-schema`**: carries a `field_path` (dot-notation, maxLength
256) and an `add_constraint` object (minProperties 1) specifying pattern, maxLength,
minimum, maximum, or enum constraints to merge into the SKILL.md `output_schema` block.

**Type 3 — `adjust-tool-allowlist`**: carries `action` (enum: `add` | `remove`) and
`tool_name` (pattern-constrained slug) targeting a `target_path` (enum of three
permitted YAML array paths within a manifest). The apply CLI adds or removes the tool
name from the correct YAML array.

**Type 4 — `no-action`**: carries a `notes` field populated by fixed-template string
interpolation using only cluster metadata (type, count, agent). No LLM prose. The
`notes` field is pattern-validated: `^Cluster type (novel|validation-failure|
tool-thrashing|delegation-mismatch): [0-9]+ dispatches from [^ ]+ do not match a
structured remediation pattern\.$`.

All string fields in all four types carry explicit `maxLength` or `pattern` constraints.
The manifest linter in strict mode validates every remediation YAML against the schema
before any apply step is permitted.

### Byte-Determinism Guarantee

The emitter is a pure TypeScript function with no random state, no `Date.now()`, and
no LLM calls. Running `cfa-harness skill-editor analyse` twice against the same
reasoning bank state produces byte-identical YAML files (the `run_window` header comment
uses the bank window start/end timestamps derived from the query, not the wall clock).

Formally: `f(OutlierReport) = YAML[]` where `f` is referentially transparent.

### Apply Semantics

The apply CLI (`cfa-harness skill-editor apply <file>.yaml`) performs deterministic
file edits:

- **`add-skill-section`**: locate `insert_after_section` by exact string match; insert
  `section_title` heading and template body verbatim. Idempotent: if `section_title`
  already exists anywhere in the file, exit 0 with a no-op log.
- **`tighten-output-schema`**: locate the first `yaml` fenced block containing
  `output_schema:`; navigate to `field_path`; merge `add_constraint` keys additively
  (existing keys are not overwritten); serialise and replace the block. Idempotent.
- **`adjust-tool-allowlist`**: locate the YAML array at `target_path`; add or remove
  `tool_name` as specified. Idempotent in both directions.
- **`no-action`**: log `notes` to stdout. Exit 0. No file touched.

Idempotent apply means applying the same YAML twice to the same file produces the same
result as applying it once. The apply CLI exits 0 (not an error) on a repeat application.

### Path Guard

Before any file read or write, the apply CLI validates `affected_skill` against:
`^plugins/.+/SKILL\.md$` or `^plugins/.+/agents/cfa/.+\.yaml$`. Any path outside
these patterns exits 1 with a path guard violation message. The check runs before the
file is opened, preventing partial writes. The manifest linter applies the same pattern
constraint to `affected_skill` in the YAML before the apply CLI is invoked — providing
a two-layer check with no single point of failure.

### Evidence Threshold

Clusters with fewer than 3 `motivating_audit_ids` are emitted only as `no-action`
remediations regardless of `recommended_action`. The emitter enforces this invariant
independently of the detector (defense-in-depth).

### Template Library

All template content lives under `docs/skill-editor-templates/` as static Markdown or
YAML files. Template IDs are filenames minus extension. Adding a new template is a
pure content commit; no code change is required. The apply CLI fails with exit 1 if a
referenced template ID does not exist on disk — there is no fallback synthesis.

### CI Cron and Human-in-the-Loop

A weekly cron (`.github/workflows/skill-editor-cron.yml`, Monday 06:00 UTC) runs
`cfa-harness skill-editor analyse` and opens a PR containing the emitted YAML files.
CI does not run `skill-editor apply`. Apply requires a deliberate human action after
PR review. There is no auto-merge path in Phase 41.

---

## Consequences

### Positive

- **Reproducibility**: the same bank state always produces the same proposal YAML files
  (byte-stable). A reviewer who re-runs `analyse` against the same bank window will see
  the same proposals, enabling meaningful comparison across runs.
- **Auditability**: every step is a pure function or deterministic file operation. The
  chain from `ReasoningEntry` to `SKILL.md` diff is fully traceable without LLM
  nondeterminism introducing ambiguity. Regulators can replay the proposal pipeline
  against any historical bank snapshot.
- **Reviewability**: reviewers see structured YAML with typed fields and constrained
  values, not arbitrary unified diff prose. A linter validates the YAML before human
  review begins. Reviewers can reject individual fields rather than having to parse
  prose intent.
- **Cost discipline**: zero LLM tokens in the proposal and apply path. The weekly cron
  runs at infrastructure cost only (pure TypeScript, no API calls).
- **Testability**: the emitter pure function and apply CLI operations are fully
  unit-testable. Each of the four remediation types can be tested with synthetic
  `OutlierReport` fixtures; apply operations can be tested with synthetic SKILL.md
  fixtures. No mock LLM provider is needed in the test path.
- **Composable operations**: remediations can be batch-applied, batch-rejected, or
  selectively applied. The YAML format is machine-readable; tooling can filter by type,
  confidence, or affected skill without parsing prose.
- **Failure mode predictability**: if a template ID is missing or a field violates a
  constraint, the failure is a schema validation error with a clear error message. There
  are no hallucination failure modes in the proposal path.

### Negative

- **Fixed proposal vocabulary**: the system can only generate proposals matching one of
  the four `RemediationType` enum values. Novel cluster patterns that do not fit any of
  the four types fall through to `no-action` and require manual human authorship of a
  new skill section. The original LLM-driven design could in principle propose arbitrary
  prose additions for any cluster pattern.
- **Template library maintenance burden**: as new failure cluster patterns emerge from
  operational use, new templates must be authored manually (initial library is Wave 5;
  ongoing community maintenance). This is a deliberate cost: template content is
  version-controlled and reviewable, unlike LLM-generated prose.
- **Heuristic detector thresholds**: the four detector functions use configurable
  thresholds (p95 percentile cap for tool-thrashing, similarity floor for novel
  clusters, minimum evidence count). These thresholds are deterministic given a fixed
  config but require operator tuning as operational patterns evolve. Miscalibration
  produces wrong-type YAMLs rather than wrong-prose diffs — a more contained failure
  but still requiring correction.
- **No cross-skill cluster proposals**: the delegation-mismatch detector identifies
  chief-to-specialist pairs; the emitter cannot target two SKILL.md files in a single
  remediation. These clusters fall to `no-action` until a multi-target remediation type
  is designed in a future phase.

### Neutral

- The reasoning bank, outlier detectors (W2), and S3 backend (W0) are unchanged by the
  pivot. Only the proposal emission and apply layers are new (W3). The learning loop
  is closed by adding deterministic components downstream of already-shipped components.
- The human-in-the-loop principle is preserved identically to the original design: no
  skill prose change is ever applied without an explicit human action. The pivot does
  not change the trust model; it changes the proposal mechanism.
- The cron cadence (weekly) and PR-based review flow are unchanged from the original
  design. Only the format of the artifacts in the PR changes (YAML instead of diffs).

---

## Alternatives Considered

**Pure LLM-driven proposals (original Phase 41 design)** — Rejected. The
`pattern-analyst` subagent produces `proposed_addition` as free LLM prose. Two runs
against the same bank state produce different diffs. This breaks the determinism
invariant upheld by every other component in the production path. The original design
was sound on safety (write guards, schema validation, path guards) but not on
reproducibility. Rejected at architectural review 2026-05-11.

**Hybrid: LLM proposes, deterministic apply** — Rejected. If the proposal content is
nondeterministic (LLM-generated), version control history is unstable: the same failure
cluster produces different YAML content each run. The proposal YAML is what ends up
reviewed and committed; nondeterminism in the proposal defeats the auditability goal
even if the apply step is deterministic.

**Auto-apply over a confidence threshold** — Rejected. Any silent prose change to a
version-controlled skill file breaks the human-in-the-loop principle established by
the original design and required by the institutional finance compliance posture.
Even high-confidence proposals may be contextually incorrect. The cost of a delayed
skill improvement is much lower than the cost of a silently mis-applied skill change.
Deferred indefinitely; not in scope for Phase 42+.

**New LLM tier with a constrained output schema** — Considered. An LLM constrained to
output only one of four typed schemas (not free prose) would be nondeterministic at the
token level but semantically deterministic if schema-validated. Rejected because: (1)
schema validation catches structural invalidity, not semantic duplicates — two valid
schema-conformant proposals for the same cluster can still differ in template_id choice;
(2) the template library is finite and enumerable, making a pure-code mapping from
cluster type to template ID both simpler and more auditable; (3) adds LLM token cost
to the cron path with no quality advantage over the pure-code emitter.

---

## Links

- Supersedes (design): `docs/plans/archive/phase-41-original-llm-driven.md`
- Implementation design: `docs/plans/phase-41-closed-learning-loop.md`
- Depends on: ADR-040 (RuVector reasoning bank) — emitter queries the bank via W2 detectors
- Depends on: ADR-041 (indexer hook) — indexer writes `validation_failed` + `affected_skill`
  to `ReasoningEntry.metadata`, which feeds the validation-failure detector
- Related: ADR-042 (hybrid deterministic/LLM dispatch) — same pattern: deterministic engine
  for reproducible path, LLM only where determinism is impossible; Phase 41 extends the
  principle to the feedback/improvement loop
- Related: ADR-043 (three-tier plugin architecture) — affected_skill paths in remediation
  YAMLs use the Phase 40 three-tier plugin path layout (`plugins/<tier>/<slug>/...`)
- Phase 39 (manifest linter strict mode) — linter validates remediation YAMLs before apply
- Phase 37 W1-W2 (output_schema validator) — validator writes `validation_failed` metadata
  that feeds the validation-failure outlier detector
- Template library: `docs/skill-editor-templates/`
- Proposed skill updates: `docs/proposed-skill-updates/`

---

## Future Work

- **Phase 42+: expand the template library.** As new outlier patterns emerge from
  operational use, new templates should be authored and committed to
  `docs/skill-editor-templates/`. The emitter requires no code change for new templates —
  only the `section_title` enum (for `add-skill-section` types) may need extension.

- **Cross-skill cluster proposals.** The delegation-mismatch detector identifies
  chief-to-specialist failures that span two SKILL.md files. A future multi-target
  remediation type could target both files in a single YAML with an ordered `changes`
  array. Deferred: current cross-skill clusters fall through to `no-action`.

- **Confidence threshold tuning.** Accepted-vs-rejected proposal ratios from human PR
  reviews can inform detector threshold adjustments. A lightweight analytics step
  (count of applied vs closed-without-apply proposals per cluster type) would surface
  systematic miscalibration. This is operational instrumentation, not a code change.

- **Validated auto-apply gate.** If the proposal acceptance rate over a sustained
  operational window exceeds a defined threshold for a given remediation type (e.g.,
  all `tighten-output-schema` proposals for a specific skill are accepted over 12 weeks),
  a guarded auto-apply CI step could be considered. Requires a formal confidence model
  and explicit operator opt-in. Not in scope until Phase 42+ operational data exists.
