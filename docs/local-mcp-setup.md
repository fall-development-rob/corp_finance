# Local MCP Setup: Full Surface Access for CFA Subagents

## Why This Exists

The corp-finance-mcp system ships with two complementary surfaces:

1. **Plugin surface** (`plugins/cfa-core`) — A lightweight, cross-platform, deployable WASM-based MCP server. Ships with 6 core tools optimized for equity and credit workflows.
2. **Packages surface** (`packages/mcp-server`) — The full local-development surface. Exposes ~90 NAPI-backed tools covering 85+ domains (valuation, fixed income, derivatives, credit, risk, regulatory, ESG, etc.).

Without explicit registration of the packages surface, CFA subagents default to the plugin surface and silently miss 84 tools. This guide wires both surfaces together for development, so subagents have full institutional-grade capability access.

See [ADR-026 (pending)](adr/ADR-026-multi-surface-mcp-routing.md) for the architectural rationale behind plugin (slim, deployable) vs. packages (full, local-dev) separation.

---

## Surface Comparison

| Aspect | Plugin (`cfa-core`) | Packages (`corp-finance-mcp`) |
|--------|-------------------|------------------------------|
| **Location** | `plugins/cfa-core/mcp/` | `packages/mcp-server/` |
| **Build** | WASM (cross-platform) | NAPI (linux-x64-gnu binary) |
| **Tools** | 6 | ~90 |
| **Tool Families** | WACC, DCF, comps, credit metrics, debt capacity, covenant compliance | Valuation, Fixed Income, Derivatives, Credit, Risk & Quant, Regulatory, ESG, PE & VC, Real Estate, Macro, and 14 others |
| **Network** | Offline | Offline (pure Rust compute) |
| **Deployment** | /plugin install (Claude Code UI) | Local ~/.claude.json registration |

### Headline Tool Families (Packages Surface)

| Family | Count | Examples |
|--------|-------|----------|
| Valuation & Modelling | 18 | DCF, WACC, comps, LBO, merger model, three-statement |
| Fixed Income | 12 | Bond pricing, duration, MBS, repo, rate models, inflation-linked |
| Derivatives | 8 | Options (BS/CRR), Greeks, vol surface, SABR, convertibles |
| Credit & Restructuring | 10 | Ratings, Altman Z, CDS, CVA, CLO waterfall, CECL |
| Risk & Quant | 13 | Factor models, Black-Litterman, VaR/CVaR, pairs trading, index construction |
| Remaining domains | 19 | Regulatory capital, ESG scoring, PE/VC, real estate, geopolitical, etc. |

---

## Enable the FULL Surface (Local Development)

Follow these steps to register the full NAPI-backed MCP server so CFA subagents can reach ~90 tools.

### Step 1: Build the NAPI binary

```bash
cd /home/robert/cfa_agent/packages/bindings
npm run build
```

**Verify:** Check that `corp-finance.linux-x64-gnu.node` exists and is recent:

```bash
ls -lh corp-finance.linux-x64-gnu.node
```

The pre-built binary is shipped in the repo and should match your `src/lib.rs` version. If you've made changes to `src/lib.rs`, `npm run build` regenerates the binary.

### Step 2: Build the MCP server

```bash
cd /home/robert/cfa_agent/packages/mcp-server
npm run build
```

**Verify:** Check that `dist/index.js` exists:

```bash
ls -lh dist/index.js
```

### Step 3: Register in ~/.claude.json

Open your project-level Claude Code configuration at `~/.claude.json` (or create it if it doesn't exist) and add the MCP server registration. Find (or create) the `projects` key with an entry for `/home/robert/cfa_agent`:

```json
{
  "projects": {
    "/home/robert/cfa_agent": {
      "mcpServers": {
        "cfa-core": {
          "command": "node",
          "args": ["/path/to/plugins/cfa-core/mcp/dist/index.js"]
        },
        "corp-finance-mcp": {
          "command": "node",
          "args": ["/home/robert/cfa_agent/packages/mcp-server/dist/index.js"]
        }
      }
    }
  }
}
```

**Important**: Replace `/home/robert/cfa_agent` with your absolute repo path if different. The path must be absolute (not relative).

### Step 4: Update CFA Agent Frontmatter

Each CFA subagent declaration at `.claude/agents/cfa/*.md` has a YAML frontmatter block with a `tools:` list. Update each agent to include `corp-finance-mcp`:

**Before:**
```yaml
tools: cfa-core, fmp-market-data
```

**After:**
```yaml
tools: cfa-core, corp-finance-mcp, fmp-market-data
```

Update all 9 agents:
- `chief-analyst.md`
- `equity-analyst.md`
- `credit-analyst.md`
- `fixed-income-analyst.md`
- `derivatives-analyst.md`
- `quant-risk-analyst.md`
- `macro-strategist.md`
- `esg-regulatory-analyst.md`
- `private-markets-analyst.md`

### Step 5: Restart Claude Code

Agent definitions are loaded at session start. Close and reopen Claude Code for the subagent YAML changes to take effect. (No hot-reload for agent frontmatter.)

### Step 6: Verify Tool Access

Spawn a CFA subagent and ask it to list its registered tools:

```
/agent cfa-equity-analyst

Use ListTools to show me your registered MCP tools.
```

**Expected output:** You should see two MCP server entries:
- `cfa-core` with 6 tools (WACC, DCF, comps, credit metrics, debt capacity, covenant compliance)
- `corp-finance-mcp` with ~90 tools across 85+ domains

If you only see `cfa-core`, the `corp-finance-mcp` server wasn't registered or the agent frontmatter wasn't updated. See **Troubleshooting** below.

---

## Enable the SLIM Surface (Plugin)

If you prefer the cross-platform plugin surface without local NAPI binaries:

### Step 1: Install the Plugin

From the Claude Code prompt:

```
/plugin install ./plugins/cfa-core
```

Follow the interactive install wizard. The plugin will be registered to your `~/.claude.json` under `enabledPlugins` and will be available as the `cfa-core` MCP server.

### Step 2: Restart Claude Code

Plugin changes take effect on restart.

### Step 3: Verify

Spawn a CFA subagent. It should now have access to the 6 `cfa-core` tools. (The full `corp-finance-mcp` surface will not be available via the plugin path.)

---

## Troubleshooting

### **Agent declares a server name that doesn't resolve**

**Symptom**: Subagent says "MCP server 'corp-finance-mcp' not found."

**Fix**: Check that the server name in agent frontmatter matches the key in `~/.claude.json`'s `mcpServers` dict. The key is case-sensitive. Typical mistake: `corp-finance-mcp` (correct) vs. `corp_finance_mcp` (wrong).

### **Plugin not enabled in ~/.claude.json**

**Symptom**: `cfa-core` tools unavailable even after `/plugin install`.

**Fix**: Check `~/.claude.json` for the `enabledPlugins` list. It should include `"cfa-core"`. If missing, re-run `/plugin install ./plugins/cfa-core`.

### **NAPI binary out of sync with packages/bindings/src/lib.rs**

**Symptom**: MCP server crashes or tools return "failed to call NAPI function" errors.

**Fix**: Regenerate the NAPI binary:

```bash
cd /home/robert/cfa_agent/packages/bindings
npm run build
```

Then rebuild the MCP server:

```bash
cd /home/robert/cfa_agent/packages/mcp-server
npm run build
```

Restart Claude Code.

---

## Surface Parity Audit

If you add a new MCP tool to `packages/mcp-server/src/tools/`, run the surface-parity checker to ensure the plugin surface is in sync (or intentionally out of sync, as designed):

```bash
cd /home/robert/cfa_agent/packages/mcp-server
npm run check:surface-parity
```

This script (defined in `packages/mcp-server/scripts/check-surface-parity.mjs`) compares the packages surface against the plugin surface and flags drift before commit. See [`packages/mcp-server/scripts/check-surface-parity.mjs`](../packages/mcp-server/scripts/check-surface-parity.mjs) for details.

---

## Next Steps

- Read the [corp-finance-mcp README](../packages/mcp-server/README.md) for tool reference and input/output schemas.
- See [`docs/VENDOR_FREE_PATH.md`](VENDOR_FREE_PATH.md) for which tools run offline vs. requiring paid vendor integrations.
- Explore the [Architecture Decision Records](adr/) for the design history of the plugin/packages split and multi-surface routing.
