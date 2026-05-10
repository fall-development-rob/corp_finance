/**
 * Specialist agents — Phase 31 Wave 2.
 *
 * Eight CFA specialists with domain-curated MCP tool subsets and
 * institutional-grade system prompts. Dispatched by chief-analyst via
 * `delegate_to_<id>` virtual tools (see ../delegate.ts).
 */

export { equityAnalyst } from "./equity.js";
export { creditAnalyst } from "./credit.js";
export { fixedIncomeAnalyst } from "./fixed-income.js";
export { derivativesAnalyst } from "./derivatives.js";
export { quantRiskAnalyst } from "./quant-risk.js";
export { macroAnalyst } from "./macro.js";
export { privateMarketsAnalyst } from "./private-markets.js";
export { esgRegulatoryAnalyst } from "./esg-regulatory.js";
