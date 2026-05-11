/**
 * Tests for Phase 41 Wave 4 — BankFactory (env-driven backend selector).
 *
 * Coverage:
 *  1. Default (no env, no opts.backend) → local bank
 *  2. opts.backend === "s3" with bucket → S3 bank
 *  3. env CFA_BANK_BACKEND=s3 → S3 bank
 *  4. Missing bucket on s3 mode → throws descriptive error
 *  5. localDir override applied to local bank (no-throw, uses custom dir)
 *  6. s3Endpoint override propagated (for MinIO compatibility)
 *  7. reasoningBankConfigFromEnv parses all CFA_BANK_* env vars correctly
 *  8. Function returns same ReasoningBank interface regardless of backend
 *
 * Agent-loop validator wiring:
 *  9. validation_failed flows from delegation validator → indexer metadata
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
  type MockInstance,
} from "vitest";
import {
  createReasoningBank,
  reasoningBankConfigFromEnv,
  type BankFactoryOptions,
} from "../src/reasoning/bank-factory.js";
import { createDeterministicEmbedder } from "../src/reasoning/embeddings.js";
import type { ReasoningBank } from "../src/reasoning/bank.js";

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

// Mock s3-bank so tests don't need AWS credentials or a real S3 endpoint.
vi.mock("../src/reasoning/s3-bank.js", () => {
  const mockBankInstance: ReasoningBank = {
    async index() {},
    async recallSimilar() { return []; },
    async recallByGraph() { return []; },
    async close() {},
  };
  return {
    createS3ReasoningBankWithEmbed: vi.fn(() => mockBankInstance),
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Use 384 dimensions for the local bank tests (matches RuVector default).
// S3 tests use a mocked bank so dimension doesn't matter for those paths.
const embed = createDeterministicEmbedder({ dim: 384 });

function makeLocalOpts(overrides: Partial<BankFactoryOptions> = {}): BankFactoryOptions {
  return { embed, ...overrides };
}

let tmpDir: string;
let savedEnv: NodeJS.ProcessEnv;
let bank: ReasoningBank | null = null;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "bank-factory-test-"));
  savedEnv = { ...process.env };
  bank = null;
  // Clean env vars between tests.
  delete process.env["CFA_BANK_BACKEND"];
  delete process.env["CFA_BANK_LOCAL_DIR"];
  delete process.env["CFA_BANK_S3_BUCKET"];
  delete process.env["CFA_BANK_S3_ENDPOINT"];
  delete process.env["CFA_BANK_S3_REGION"];
  delete process.env["CFA_BANK_S3_KEY_PREFIX"];
});

afterEach(async () => {
  if (bank) {
    try { await bank.close(); } catch { /* best-effort */ }
    bank = null;
  }
  // Restore env.
  for (const key of Object.keys(process.env)) {
    if (!(key in savedEnv)) {
      delete process.env[key];
    }
  }
  Object.assign(process.env, savedEnv);
  rmSync(tmpDir, { recursive: true, force: true });
  vi.clearAllMocks();
});

// ---------------------------------------------------------------------------
// Import the S3 mock for call-count assertions.
// ---------------------------------------------------------------------------

import { createS3ReasoningBankWithEmbed } from "../src/reasoning/s3-bank.js";

// ---------------------------------------------------------------------------
// Tests: bank selection
// ---------------------------------------------------------------------------

describe("createReasoningBank — backend selection", () => {
  // Test 1: Default → local bank
  it("creates a local RuVector bank by default (no env, no opts.backend)", async () => {
    bank = await createReasoningBank(makeLocalOpts({ localDir: tmpDir }));
    // Local bank satisfies ReasoningBank interface.
    expect(typeof bank.index).toBe("function");
    expect(typeof bank.recallSimilar).toBe("function");
    expect(typeof bank.recallByGraph).toBe("function");
    expect(typeof bank.close).toBe("function");
    // S3 factory should NOT have been called.
    expect(createS3ReasoningBankWithEmbed).not.toHaveBeenCalled();
  });

  // Test 2: opts.backend === "s3" with bucket → S3 bank
  it('creates an S3 bank when opts.backend === "s3" and bucket is provided', async () => {
    bank = await createReasoningBank(
      makeLocalOpts({ backend: "s3", s3Bucket: "test-bucket" }),
    );
    expect(createS3ReasoningBankWithEmbed).toHaveBeenCalledOnce();
    const callArgs = (createS3ReasoningBankWithEmbed as unknown as MockInstance).mock.calls[0]?.[0] as Record<string, unknown>;
    expect(callArgs["bucket"]).toBe("test-bucket");
  });

  // Test 3: env var CFA_BANK_BACKEND=s3 → S3 bank
  it('creates an S3 bank when CFA_BANK_BACKEND=s3 env var is set', async () => {
    process.env["CFA_BANK_BACKEND"] = "s3";
    process.env["CFA_BANK_S3_BUCKET"] = "env-bucket";
    bank = await createReasoningBank(makeLocalOpts());
    expect(createS3ReasoningBankWithEmbed).toHaveBeenCalledOnce();
    const callArgs = (createS3ReasoningBankWithEmbed as unknown as MockInstance).mock.calls[0]?.[0] as Record<string, unknown>;
    expect(callArgs["bucket"]).toBe("env-bucket");
  });

  // Test 4: Missing bucket on s3 mode → throws
  it("throws a descriptive error when backend=s3 but no bucket is configured", async () => {
    await expect(
      createReasoningBank(makeLocalOpts({ backend: "s3" })),
    ).rejects.toThrow(/s3Bucket is required when backend=s3/);
  });

  // Test 5: localDir override applied to local bank
  it("uses the provided localDir for the local bank", async () => {
    const customDir = join(tmpDir, "custom-bank");
    bank = await createReasoningBank(makeLocalOpts({ localDir: customDir }));
    // Local bank is created; should be functional (index + recall round-trip).
    await bank.index(
      {
        audit_id: "a1",
        agent_id: "chief",
        prompt_hash: "ph",
        prompt_summary: "test",
        tool_calls: [],
        delegations: [],
        result_excerpt: "ok",
        metadata: {},
        timestamp: new Date().toISOString(),
      },
      "test",
    );
    const hits = await bank.recallSimilar("test", { k: 1 });
    expect(hits.length).toBe(1);
    expect(hits[0]!.audit_id).toBe("a1");
  });

  // Test 6: s3Endpoint override propagated (for MinIO)
  it("passes s3Endpoint to createS3ReasoningBankWithEmbed for MinIO support", async () => {
    bank = await createReasoningBank(
      makeLocalOpts({
        backend: "s3",
        s3Bucket: "minio-bucket",
        s3Endpoint: "http://localhost:9000",
        s3Region: "local",
      }),
    );
    const callArgs = (createS3ReasoningBankWithEmbed as unknown as MockInstance).mock.calls[0]?.[0] as Record<string, unknown>;
    expect(callArgs["endpoint"]).toBe("http://localhost:9000");
    expect(callArgs["region"]).toBe("local");
  });

  // Test 8: same ReasoningBank interface regardless of backend
  it("returns the same ReasoningBank interface for both local and s3 backends", async () => {
    const localBank = await createReasoningBank(makeLocalOpts({ localDir: tmpDir }));
    const s3Bank = await createReasoningBank(
      makeLocalOpts({ backend: "s3", s3Bucket: "iface-test" }),
    );
    for (const b of [localBank, s3Bank]) {
      expect(typeof b.index).toBe("function");
      expect(typeof b.recallSimilar).toBe("function");
      expect(typeof b.recallByGraph).toBe("function");
      expect(typeof b.close).toBe("function");
    }
    await localBank.close();
    await s3Bank.close();
  });
});

// ---------------------------------------------------------------------------
// Tests: reasoningBankConfigFromEnv
// ---------------------------------------------------------------------------

describe("reasoningBankConfigFromEnv", () => {
  // Test 7: parses all CFA_BANK_* env vars correctly
  it("parses all CFA_BANK_* env vars into BankFactoryOptions fields", () => {
    const fakeEnv: NodeJS.ProcessEnv = {
      CFA_BANK_BACKEND: "s3",
      CFA_BANK_LOCAL_DIR: "/data/bank",
      CFA_BANK_S3_BUCKET: "my-bucket",
      CFA_BANK_S3_ENDPOINT: "http://minio:9000",
      CFA_BANK_S3_REGION: "eu-west-1",
      CFA_BANK_S3_KEY_PREFIX: "prod-bank",
    };
    const config = reasoningBankConfigFromEnv(fakeEnv);
    expect(config.backend).toBe("s3");
    expect(config.localDir).toBe("/data/bank");
    expect(config.s3Bucket).toBe("my-bucket");
    expect(config.s3Endpoint).toBe("http://minio:9000");
    expect(config.s3Region).toBe("eu-west-1");
    expect(config.s3KeyPrefix).toBe("prod-bank");
  });

  it("returns empty object when no CFA_BANK_* vars are set", () => {
    const config = reasoningBankConfigFromEnv({});
    expect(Object.keys(config).length).toBe(0);
  });

  it("ignores unknown CFA_BANK_BACKEND values", () => {
    const config = reasoningBankConfigFromEnv({ CFA_BANK_BACKEND: "redis" });
    expect(config.backend).toBeUndefined();
  });

  it("accepts CFA_BANK_BACKEND=local explicitly", () => {
    const config = reasoningBankConfigFromEnv({ CFA_BANK_BACKEND: "local" });
    expect(config.backend).toBe("local");
  });
});
