/**
 * Reasoning module barrel — Phase 34 Wave 1.
 *
 * Exports the ReasoningBank port + RuVector implementation, the pluggable
 * embedding helpers, and the low-level RVIndex wrapper for callers that need
 * direct access (Wave 2 indexer, integration tests).
 */
export * from "./bank.js";
export * from "./embeddings.js";
export * from "./rv-index.js";
export * from "./indexer.js";
export * from "./recall-tool.js";
export * from "./recall-graph-tool.js";
// Phase 41 Wave 0 — S3 reasoning bank
export { createS3ReasoningBank, createS3ReasoningBankWithEmbed, type S3ReasoningBankOptions, type S3ReasoningBankWithEmbedOptions } from "./s3-bank.js";
// Phase 41 Wave 2 — outlier detection
export * from "./outliers.js";
