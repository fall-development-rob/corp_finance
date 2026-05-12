# Phase 27 — Research: claude-for-legal techniques applicable to our harness

**Status**: Research note (no implementation yet — design for review)
**Author**: Research dispatch, 2026-05-12
**Source**: `anthropics/claude-for-legal` @ main (commit at time of survey)

---

## 1. What `claude-for-legal` is

A first-party Anthropic plugin suite for legal practice. Same architectural primitives we use — `agent.yaml` + `subagents/*.yaml` + skills + steering examples + MCP connectors — packaged as **two delivery modes from one source**: install as a Claude Cowork / Claude Code plugin **OR** deploy through the Managed Agents API. Identical system prompt and skills either way.

Repo size at survey time:

| Surface | Count |
|---|---|
| Practice-area plugins | 10 (commercial, privacy, product, corporate, employment, litigation, regulatory, ai-governance, legal-clinic, law-student) + builder-hub + external `cocounsel-legal` |
| Skills | ~140 across all plugins |
| Named agents (one per workflow) | 30+ |
| Managed-agent cookbooks | 5 (renewal-watcher, docket-watcher, reg-monitor, diligence-grid, launch-radar) |
| MCP connectors shipped | 20+ (Slack, Google Drive, Lexis+, Box, Ironclad, DocuSign, iManage, CourtListener, Everlaw, …) |

Our equivalents:

| Surface | We have |
|---|---|
| Vertical plugins | 11 (`plugins/vertical-plugins/`) |
| Skills | ~50 |
| Specialist analyst agents | 9 |
| Managed-agent cookbooks | 15 |
| MCP servers | 4 in-repo (cfa-core, fmp, data, vendor) — 623 tools |

Architecturally we are very close. The interesting deltas are **operational patterns** baked into their prompts and skills, not their plumbing.

## 2. Architectural side-by-side

Both stacks structure a managed-agent cookbook identically:

| Layer | claude-for-legal | cfa_agent |
|---|---|---|
| Cookbook dir | `managed-agent-cookbooks/<slug>/` | `managed-agent-cookbooks/<slug>/` |
| Parent manifest | `agent.yaml` | `agent.yaml` |
| Steering | `steering-examples.json` | `steering-examples.json` |
| Subagents | `subagents/*.yaml` | `subagents/*.yaml` |
| Skill linkage | `skills: - from_plugin: ../../commercial-legal` (whole plugin) | `skills: - from_plugin: ../../plugins/.../some-skill` (single skill) |
| MCP wiring | `mcp_servers: - { type: url, name, url: "${VAR}" }` | identical |
| Orchestrator scope | local tools only (`read`, `grep`, `glob`) | mixed (most parents are `*`) |
| Write tool scoping | "only one leaf with Write" pattern | publishers/exporters have Write |

**Convergent design.** Many things we built independently (3-subagent pattern, explicit-allow gating on compute MCPs, anti-injection in `system.append`, output_schema on every subagent) match their patterns exactly. Phase 25 Tier A1–A4 contracts (MA-001 through MA-008) codify many of the same invariants their cookbooks observe by convention.

**Three real architectural deltas** they have that we don't:

1. **`from_plugin: <whole-plugin-dir>`** — references the entire plugin tree instead of a single skill. The loader walks all `*/SKILL.md` files inside.
2. **Orchestrator-scoped tools — `local-only`** — parent is read/grep/glob; every MCP toolset lives only on subagent leaves. We currently allow parents to use `tools: "*"`.
3. **Only-one-Write-leaf** — exactly one subagent has Write enabled; everyone else is read-only. Subagent prompts state this explicitly ("you are the ONLY worker with Write"). Reduces blast radius if a non-write subagent produces a hallucinated file path.

## 3. Six patterns worth adopting (ranked)

### 3a. Cold-start interview + practice profile (highest impact)

**What they do.** Every plugin ships a `cold-start-interview` skill that runs a 2–15 minute interview and writes a populated `CLAUDE.md` to:

```
~/.claude/plugins/config/claude-for-legal/<plugin>/CLAUDE.md
```

The template ships with the plugin; user-specific data is written to a version-independent config path that survives plugin updates. Every other skill **reads from the config path, not the template**, and refuses to run if the config still contains `[PLACEHOLDER]` markers. Skills literally stop with a message like:

> "This plugin needs setup before it can give you useful output. Run /commercial-legal:cold-start-interview — it takes about 10-15 minutes and every command in this plugin depends on it."

The template is a detailed practice profile: who the company is, who's using the tool, available integrations (table with ✓/✗), playbook positions (e.g. "Sales-side limitation-of-liability cap: 12 months fees paid"), output formatting conventions.

A **shared company profile** at `~/.claude/plugins/config/<marketplace>/company-profile.md` carries facts every plugin reads (name, industry, size).

**Why this is the strongest adoption candidate.** Our 15 cookbooks all currently assume firm conventions that don't have a home: which equity-risk-premium does the firm use, which valuation multiples are house-standard, which countries are in scope, which thresholds trigger escalation, which jurisdictions the GP operates in. Today every CFA cookbook output reflects *Claude's* defaults, not *the firm's* defaults — and refuses-to-run-without-setup forces the conversation.

**Concrete proposal for us.** Add a single `cold-start` skill at `plugins/cfa-core/skills/cold-start-interview/` that drives an interview and writes a firm profile to `~/.claude/plugins/config/cfa-agent/firm-profile.md`. Per-cookbook skills check for the profile and stop with a clear pointer if missing.

### 3b. Reinforced anti-injection block (high impact, easy)

Our MA-006 contract requires "DATA, not directives" in `system.append`. They go significantly further. The `alert-writer.yaml` for renewal-watcher carries a 30-line mandatory anti-injection block covering:

- **Spreadsheet formula injection**: if any input-derived string starts with `=`, `+`, `-`, `@`, tab, or `\r`, prefix with `'` to neutralize Excel formula interpretation if the reviewer copies the Markdown into a spreadsheet.
- **Markdown table breakage**: escape `|` as `\|` so an input-derived value can't break table structure.
- **HTML smuggling**: strip or escape `<` `>` as `&lt;` `&gt;` so Markdown can't smuggle HTML.
- **URL phishing**: never render input-derived URLs as clickable Markdown links; use inert backticks (`` `https://...` ``). Hidden-target links and `[text](url)` with upstream-controlled URL are vectors for phishing.

**Concrete proposal.** Codify this as a canonical template at `docs/skill-editor-templates/add-skill-section/anti-injection-reinforced.md` (Phase 41 templates dir already exists) and apply via the deterministic skill-editor pipeline to every publisher subagent. Tighten MA-006 with substring checks for at least one of the four classes.

### 3c. Verification / "lead, not conclusion" footer (high impact, easy)

Every legal cookbook output ends with a **required verification footer** that the publisher subagent emits verbatim, regardless of content:

> *This report was produced by an automated agent from CLM metadata. Every cancel-by date, renewal term, deviation flag, and escalation route requires verification against the signed agreement by a licensed attorney before it is relied on, calendared, or used to inform a termination or renewal decision. Metadata drifts from executed documents.*

The subagent prompt explicitly says "Do not rephrase or shorten the verification footer. It prints once per report, at the end, regardless of alert count."

**Concrete proposal for us.** Every CFA cookbook output should carry a domain-appropriate footer:

- Valuation cookbooks: "This valuation is a model output, not a recommendation. Verify inputs against the current trading screen; the analyst is responsible for the final number."
- Credit cookbooks: "This credit assessment is screening, not a rating opinion. A rated opinion requires the firm's credit committee."
- KYC: "Sanctions screening is a tooling check, not a compliance conclusion. The MLRO signs off."

Add as new contract MA-009: "Every publisher subagent's system prompt includes a fixed verification footer." Codify the canonical footer per domain in `docs/skill-editor-templates/`.

### 3d. Matter workspace (medium impact)

Every legal plugin has a `matter-workspace` skill. Concept: an "active matter" is a per-engagement slug; the user runs `/<plugin>:matter-workspace switch <slug>` to make it active; every other skill checks for an active matter at the start, loads `matters/<slug>/matter.md` for matter-specific context, and writes outputs to `matters/<slug>/`. **Never reads another matter's files unless `Cross-matter context` is on.**

For in-house users the machinery is invisible by default (`Enabled: ✗`). For firm users it's the default scoping for every output.

**Concrete proposal for us.** Adopt as `plugins/cfa-core/skills/engagement-workspace/`. Every cookbook output goes to `engagements/<slug>/` (e.g., `engagements/2026-Q1-aapl-coverage/equity-analyst.json`). For LP/fund-admin cookbooks, scope is per-fund-vintage. Avoids cross-engagement leak by default; users opt in to cross-engagement when they actually want it (e.g., portfolio-level views).

### 3e. Orchestrator local-only tool scoping (medium impact)

Their cookbook parents have **only** `read`, `grep`, `glob` enabled at the orchestrator level. MCP toolsets live exclusively on subagent leaves. Comment in the YAML: "Orchestrator is scoped to local-only tools; MCP toolsets are held by the subagent leaves (see callable_agents)."

Ours allow `tools: "*"` on most parents — which means the orchestrator can directly hit any MCP server's tool surface, bypassing the structured handoff to subagents.

**Concrete proposal.** Tighten the cookbook authoring pattern: parents get `agent_toolset_20260401` configs only (no `mcp_toolset` blocks). MCP toolsets are subagent-scoped. Encode as **contract MA-010** + update the scaffolder template.

### 3f. Two delivery modes from one source (already there, document explicitly)

Their README states: "Everything here is available **two ways from one source**: install it as a [Claude Cowork](https://claude.com/product/cowork) or [Claude Code](https://claude.com/product/claude-code) plugin, or deploy it through the [Claude Managed Agents API](https://docs.claude.com/en/api/managed-agents) behind your own workflow engine. Same system prompt, same skills — you choose where it runs."

We already do this — the same skill files and agent definitions back both the plugin-install path (Claude Code via `plugins/*/`) and the managed-agent deploy path (cookbooks resolve `from_plugin` against the same plugin tree). The README doesn't make this explicit. Worth highlighting.

## 4. Deploy claude-for-legal-style cookbooks via our managed-agent infrastructure

The user's prompt asked specifically: *"use the cookbooks to deploy the agents to our servers"*.

Two interpretations, both useful:

### Interpretation A — adopt the patterns inside our existing cookbooks

Take the six techniques in §3 above and apply them to our 15 finance cookbooks. The result: our cookbooks become "claude-for-legal-grade" — firm-aware (via cold-start interview), hardened against prompt injection (reinforced clause), explicit about their role (verification footer), per-engagement scoped (matter workspace), and architecturally tighter (local-only orchestrator). **No new cookbooks needed.** Our existing managed-agent cookbook infrastructure (Tier A1–A4 catalogs/audits/replays/traces/cost/version + Tier D14 scaffolder) carries them.

This is the high-ROI path. ~5–10 days of focused work, no new tools or MCP servers.

### Interpretation B — actually deploy legal cookbooks alongside our finance cookbooks

Authors a `legal/` directory of cookbooks alongside `managed-agent-cookbooks/` that target legal workflows using our deploy pipeline. Requires:

- New MCP servers for the legal stack (Lexis+ Protégé, CourtListener, Ironclad, iManage, DocuSign) — these don't exist in our `packages/`. Most are HTTP MCP endpoints; we'd add stub connectors with credential-gating like we did for `chronograph` and `egnyte` in Wave 5 of the upstream-gap work.
- A `plugins/vertical-plugins/legal-*/` tree (commercial-legal, litigation-legal, etc.) shipped as our own skills.
- An "intake" cookbook per practice area following our existing 3-subagent pattern.

**Cost.** A full port of even 1 of their 10 plugins is several weeks. Most of the value sits in the firm-specific config (which we can't ship), the connectors (we don't operate them), and the unsettled-law disclaimers (out of scope for a finance team's repo).

**Recommendation.** Don't take this path. We are a finance stack, not a legal stack. The patterns transfer; the domain doesn't. Interpretation A captures the strategic value.

## 5. Proposed Phase 28 wave plan

If the patterns above land for the user, Phase 28 would be a focused 5–10 day adoption sprint:

| Wave | Description | Estimated effort |
|---|---|---|
| W1 | Add `plugins/cfa-core/skills/cold-start-interview/` + canonical firm-profile.md template; bind into all 9 specialist agents. | 2 days |
| W2 | Tighten MA-006 with specific anti-injection substrings; ship the reinforced clause template; apply via skill-editor pipeline. | 1 day |
| W3 | Add MA-009 (verification footer); author per-domain footer templates; apply to publishers. | 1 day |
| W4 | Add MA-010 (orchestrator local-only tools); tighten the scaffolder; migrate the 15 existing cookbooks (parent `tools: "*"` → local-only block). | 2 days |
| W5 | Add `engagement-workspace` skill + `engagements/<slug>/` output convention. Optional opt-in for in-house users. | 2 days |

Aside: a small **doc PR** could update our README to explicitly state the two-delivery-modes story.

## 6. Open questions

**6a. Cold-start scope.** One unified `cold-start-interview` for the whole stack, or per-cookbook? Legal does per-plugin. We'd probably want per-stack (one firm profile) for finance because the conventions are firm-wide, not per-domain. But specialist-vertical opt-ins (e.g., a PE-specific cold-start for fund parameters) layered on top.

**6b. Verification footer wording.** Domain-specific. Need attorney/CFA review before codifying.

**6c. Orchestrator local-only retrofitting.** The 15 existing cookbooks use `tools: "*"` on parents. Tightening to local-only is a behavior change — need to verify no parent actually depends on direct MCP access (we expect not, since the architecture intent has always been parents-delegate-only).

**6d. Skill-editor templates.** Tying this to Phase 41's deterministic learning loop means the canonical templates live in `docs/skill-editor-templates/` and the `apply` CLI does the byte-deterministic edit. No prose synthesis at runtime. This keeps Phase 27 adoption fully deterministic.

---

## Appendix: source references

- Repo: https://github.com/anthropics/claude-for-legal
- README: full reference of agents, plugins, connectors
- CONNECTORS.md: how to add a new MCP connector
- QUICKSTART.md: 60-second install path
- `commercial-legal/CLAUDE.md`: the canonical practice profile template
- `commercial-legal/skills/nda-review/SKILL.md`: example of "Matter context", "Destination check", "Load the playbook first", GREEN/YELLOW/RED triage
- `managed-agent-cookbooks/renewal-watcher/agent.yaml`: same-shape cookbook as ours, orchestrator-local-only tooling
- `managed-agent-cookbooks/renewal-watcher/subagents/alert-writer.yaml`: reinforced anti-injection block + verification footer pattern
- `ai-governance-legal/skills/cold-start-interview/SKILL.md`: cold-start interview pattern
- `ai-governance-legal/.mcp.json`: per-plugin MCP connector list
