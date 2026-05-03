# Installing the cfa-* plugins

The three plugins (`cfa-core`, `cfa-data`, `cfa-pro`) are distributed
**privately** via this repository — they are not on the public Claude
plugin marketplace.

## Quick install (for collaborators with this repo cloned)

```bash
# 1. Build all three plugins so they're self-contained.
bash scripts/build-plugins.sh

# 2. Install each plugin you want.
claude plugin install ./plugins/cfa-core
claude plugin install ./plugins/cfa-data    # optional — free public data
claude plugin install ./plugins/cfa-pro     # optional — premium vendors (BYO API keys)
```

After install, Claude Code picks up the plugins on next start and exposes
their MCP tools, skills, and slash commands automatically.

## Install on a fresh machine

```bash
git clone https://github.com/fall-development-rob/corp_finance.git
cd corp_finance
bash scripts/build-plugins.sh

# Then run the install commands above.
```

The build script:
- Builds the WASM artifact for cfa-core (~5 minutes cold, ~30s warm)
- Builds the TS MCP servers for cfa-data and cfa-pro (~10s each)
- Bundles `dist/`, `package.json`, and runtime `node_modules` into each
  plugin folder so the manifests use self-contained relative paths
- After the build, each plugin folder can be zipped, copied, or
  rsync'd standalone — no monorepo dependency

## Sharing with someone

Two options:

1. **Share the repo URL** — they clone, run `build-plugins.sh`, install.
   Best when they may want to read source or contribute back.
2. **Share a built plugin folder** — `tar czf cfa-core.tar.gz plugins/cfa-core`
   and they `claude plugin install ./cfa-core` after extracting. Best for
   non-developer recipients.

## Environment variables

`cfa-core` needs none. `cfa-data` and `cfa-pro` pass through env vars
to their MCP servers — set what you have, leave the rest unset.

```bash
# cfa-data (3 of 14 sources need keys; the other 11 work without)
export ACLED_ACCESS_TOKEN=...   # https://acleddata.com/register/
export NASA_FIRMS_API_KEY=...   # https://firms.modaps.eosdis.nasa.gov/api/
export EIA_API_KEY=...          # https://www.eia.gov/opendata/register.php

# cfa-pro (each vendor independent — set what you license)
export FMP_API_KEY=...
export LSEG_CLIENT_ID=...
export LSEG_CLIENT_SECRET=...
export SP_GLOBAL_API_KEY=...
export FACTSET_USERNAME=...
export FACTSET_API_KEY=...
export MORNINGSTAR_API_KEY=...
export MOODYS_CLIENT_ID=...
export MOODYS_CLIENT_SECRET=...
export PITCHBOOK_API_KEY=...
```

## Updating

```bash
git pull
bash scripts/build-plugins.sh
# Restart Claude Code to pick up the rebuilt plugins.
```

## Uninstalling

```bash
claude plugin uninstall cfa-core
claude plugin uninstall cfa-data
claude plugin uninstall cfa-pro
```

## Troubleshooting

**"WASM export missing: ..."** in cfa-core stderr at startup
→ The WASM artifact is out of date. Run `bash plugins/cfa-core/scripts/build-wasm.sh`.

**"Cannot find module '@modelcontextprotocol/sdk/...'"** in cfa-data or cfa-pro
→ The plugin's `mcp/node_modules` wasn't installed. Re-run `bash scripts/build-plugins.sh`.

**MCP server not connecting**
→ `claude mcp list` to verify it's registered. If it is but shows ✗, run the
plugin's MCP entry script directly with `node ./plugins/cfa-X/mcp/dist/...`
to see startup errors.
