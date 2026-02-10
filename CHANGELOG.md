# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-10

### Added
- Phase 3 jurisdiction modules (reconciliation, withholding tax, NAV, GP economics, investor returns, UBTI) and trading diary analytics
- Phase 2 deal modelling modules (LBO, waterfall, merger model, Altman Z-score, fund fees)
- 81 integration tests with known-answer fixtures
- TypeScript MCP server with 14 tools and Zod validation
- napi-rs bridge with 14 JSON-boundary functions
- CLI binary (`cfa`) with 11 subcommands and 4 output formats
- PE returns, portfolio risk, and scenario analysis modules
- Credit metrics, debt capacity, and covenant compliance modules
- WACC, DCF, and trading comps valuation modules
- Shared kernel with types, errors, and time-value functions
- Initial project setup with workspace config and docs

### Fixed
- Resolved all clippy warnings and fixed pre-commit hook

### Changed
- Applied cargo fmt across entire workspace
- Added git hooks for conventional commits and linting
