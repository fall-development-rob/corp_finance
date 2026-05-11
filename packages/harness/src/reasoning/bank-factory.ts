/**
 * BankFactory — Phase 41 Wave 4.
 *
 * Env-driven factory that returns the appropriate ReasoningBank implementation.
 * Reads `CFA_BANK_BACKEND` (default: "local") to decide between the local
 * RuVector bank and the S3-backed bank. All configuration can be provided via
 * options OR via environment variables, with options taking precedence.
 *
 * Environment variables:
 *   CFA_BANK_BACKEND         — "local" | "s3"
 *   CFA_BANK_S3_BUCKET       — S3 bucket name (required when backend=s3)
 *   CFA_BANK_S3_ENDPOINT     — Optional endpoint override for MinIO
 *   CFA_BANK_S3_REGION       — AWS region (default: us-east-1)
 *   CFA_BANK_S3_KEY_PREFIX   — Object key prefix (default: cfa-bank)
 *   CFA_BANK_LOCAL_DIR       — Storage directory for local bank (default: ./bank-state)
 */
import { createRuVectorBank } from "./bank.js";
import {
  createS3ReasoningBankWithEmbed,
  type S3ReasoningBankWithEmbedOptions,
} from "./s3-bank.js";
import type { ReasoningBank } from "./bank.js";
import type { EmbeddingFn } from "./embeddings.js";

export type BankBackend = "local" | "s3";

export interface BankFactoryOptions {
  /** Override via env CFA_BANK_BACKEND. Defaults to "local". */
  backend?: BankBackend;
  /** Embed function — both backends need this. */
  embed: EmbeddingFn;
  /** Local-only: directory for RuVector storage. Defaults to ./bank-state. */
  localDir?: string;
  /** S3-only: bucket name. Required when backend=s3. */
  s3Bucket?: string;
  /** S3-only: endpoint override for MinIO. Defaults to AWS. */
  s3Endpoint?: string;
  /** S3-only: region. Defaults to us-east-1. */
  s3Region?: string;
  /** S3-only: object key prefix. Defaults to "cfa-bank". */
  s3KeyPrefix?: string;
}

/**
 * Read all `CFA_BANK_*` environment variables and return a
 * Partial<BankFactoryOptions>. Useful for CLI callers that want to merge
 * env config with explicit options.
 */
export function reasoningBankConfigFromEnv(
  env: NodeJS.ProcessEnv = process.env,
): Partial<BankFactoryOptions> {
  const result: Partial<BankFactoryOptions> = {};
  const rawBackend = env["CFA_BANK_BACKEND"];
  if (rawBackend === "local" || rawBackend === "s3") {
    result.backend = rawBackend;
  }
  if (env["CFA_BANK_LOCAL_DIR"]) {
    result.localDir = env["CFA_BANK_LOCAL_DIR"];
  }
  if (env["CFA_BANK_S3_BUCKET"]) {
    result.s3Bucket = env["CFA_BANK_S3_BUCKET"];
  }
  if (env["CFA_BANK_S3_ENDPOINT"]) {
    result.s3Endpoint = env["CFA_BANK_S3_ENDPOINT"];
  }
  if (env["CFA_BANK_S3_REGION"]) {
    result.s3Region = env["CFA_BANK_S3_REGION"];
  }
  if (env["CFA_BANK_S3_KEY_PREFIX"]) {
    result.s3KeyPrefix = env["CFA_BANK_S3_KEY_PREFIX"];
  }
  return result;
}

/**
 * Resolve the effective backend from options + env.
 * Options take precedence over the environment variable.
 */
function resolveBackend(opts: BankFactoryOptions): BankBackend {
  if (opts.backend !== undefined) return opts.backend;
  const envVal = process.env["CFA_BANK_BACKEND"];
  if (envVal === "s3") return "s3";
  return "local";
}

/**
 * Build S3 options from explicit opts fields falling back to env vars.
 * Throws if bucket is missing when backend=s3.
 */
function buildS3Options(opts: BankFactoryOptions): S3ReasoningBankWithEmbedOptions {
  const bucket =
    opts.s3Bucket ?? process.env["CFA_BANK_S3_BUCKET"] ?? "";
  if (!bucket) {
    throw new Error(
      "createReasoningBank: s3Bucket is required when backend=s3. " +
        "Set opts.s3Bucket or CFA_BANK_S3_BUCKET env var.",
    );
  }
  const endpoint =
    opts.s3Endpoint ?? process.env["CFA_BANK_S3_ENDPOINT"];
  const region =
    opts.s3Region ?? process.env["CFA_BANK_S3_REGION"] ?? "us-east-1";
  const keyPrefix =
    opts.s3KeyPrefix ?? process.env["CFA_BANK_S3_KEY_PREFIX"] ?? "cfa-bank";

  return {
    bucket,
    embed: opts.embed,
    ...(endpoint !== undefined ? { endpoint } : {}),
    region,
    keyPrefix,
  };
}

/**
 * Env-driven factory that returns the appropriate ReasoningBank implementation.
 *
 * - backend "s3" (from opts.backend or CFA_BANK_BACKEND=s3) → S3-backed bank.
 * - backend "local" (default) → RuVector-backed local bank.
 *
 * S3: all S3 config is read from opts first, then env vars as fallback.
 *     Throws a descriptive error if the S3 bucket is missing.
 *
 * Local: uses opts.localDir, or "./bank-state" as default.
 *        Returns a Promise<ReasoningBank> (async open).
 */
export async function createReasoningBank(
  opts: BankFactoryOptions,
): Promise<ReasoningBank> {
  const backend = resolveBackend(opts);

  if (backend === "s3") {
    const s3Opts = buildS3Options(opts);
    return createS3ReasoningBankWithEmbed(s3Opts);
  }

  // Local RuVector bank.
  const dir = opts.localDir ?? process.env["CFA_BANK_LOCAL_DIR"] ?? "./bank-state";
  return createRuVectorBank({ dir, embed: opts.embed });
}
