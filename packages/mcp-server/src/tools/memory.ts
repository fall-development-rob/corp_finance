import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  surfaceMemoryIngest,
  surfaceMemoryFind,
  surfaceSessionSave,
  surfaceSessionRestore,
} from "../bindings.js";
import {
  SurfaceMemoryIngestSchema,
  SurfaceMemoryFindSchema,
  SurfaceSessionSaveSchema,
  SurfaceSessionRestoreSchema,
} from "../schemas/memory.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMemoryTools(server: McpServer) {
  server.tool(
    "surface_memory_ingest",
    "Ingest a RunSummary into the HNSW + BM25 native memory store. Input: a RunSummary record from a CLI / MCP / skill / plugin surface event. Output: indexed run_id and timestamp. RUF-MEM-001.",
    SurfaceMemoryIngestSchema.shape,
    async (params) => {
      const validated = SurfaceMemoryIngestSchema.parse(coerceNumbers(params));
      const result = surfaceMemoryIngest(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "surface_memory_find",
    "Hybrid HNSW + BM25 retrieval over indexed RunSummaries. Input: a MemoryQuery (embedding and/or text, optional surface and tenant filters, limit). Output: ranked list of RunSummary records with fused scores. RUF-MEM-003 / RUF-MEM-004.",
    SurfaceMemoryFindSchema.shape,
    async (params) => {
      const validated = SurfaceMemoryFindSchema.parse(coerceNumbers(params));
      const result = surfaceMemoryFind(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "surface_session_save",
    "Persist a CfaSession aggregate to the .cfa-session portable archive (gzip-compressed JSON). Input: in-memory session object and absolute output path. Output: bytes written and final path. RUF-MEM-006.",
    SurfaceSessionSaveSchema.shape,
    async (params) => {
      const validated = SurfaceSessionSaveSchema.parse(coerceNumbers(params));
      const result = surfaceSessionSave(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "surface_session_restore",
    "Restore a CfaSession aggregate from a .cfa-session archive on disk. Input: absolute path to the archive. Output: rehydrated session object with all RunSummaries. RUF-MEM-007.",
    SurfaceSessionRestoreSchema.shape,
    async (params) => {
      const validated = SurfaceSessionRestoreSchema.parse(coerceNumbers(params));
      const result = surfaceSessionRestore(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
