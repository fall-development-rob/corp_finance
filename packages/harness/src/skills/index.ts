/**
 * Barrel export for the Phase 33 skill loader infrastructure.
 * Phase 40 Wave 4: adds createMultiRootSkillLoader.
 */

export * from "./types.js";
export { parseFrontmatter } from "./frontmatter-parser.js";
export { assembleSystemPrompt } from "./assembler.js";
export { createDirectSkillLoader } from "./loader.js";
export { createMultiRootSkillLoader } from "./multi-root-loader.js";
export type { MultiRootSkillLoaderOptions } from "./multi-root-loader.js";
