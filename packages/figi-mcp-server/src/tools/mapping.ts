import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { figiPost, CacheTTL } from '../client.js';
import {
  SingleMappingSchema,
  BulkMappingSchema,
  IsinMappingSchema,
  CusipMappingSchema,
} from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerMappingTools(server: McpServer) {
  server.tool(
    'figi_map',
    'Map a single financial identifier (ISIN, CUSIP, SEDOL, TICKER, etc.) to its FIGI and associated instrument data. Returns matching FIGI records with exchange, ticker, name, and security type.',
    SingleMappingSchema.shape,
    async (params) => {
      const { idType, idValue, exchCode, micCode, currency } = SingleMappingSchema.parse(params);
      const job: Record<string, string> = { idType, idValue };
      if (exchCode) job.exchCode = exchCode;
      if (micCode) job.micCode = micCode;
      if (currency) job.currency = currency;
      const data = await figiPost('mapping', [job], { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'figi_bulk_map',
    'Batch-map up to 100 financial identifiers to FIGIs in a single request. Each job specifies idType, idValue, and optional filters. Returns array of result sets, one per job.',
    BulkMappingSchema.shape,
    async (params) => {
      const { jobs } = BulkMappingSchema.parse(params);
      const payload = jobs.map(j => {
        const job: Record<string, string> = { idType: j.idType, idValue: j.idValue };
        if (j.exchCode) job.exchCode = j.exchCode;
        if (j.micCode) job.micCode = j.micCode;
        if (j.currency) job.currency = j.currency;
        return job;
      });
      const data = await figiPost('mapping', payload, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'figi_isin_to_ticker',
    'Convenience tool: map an ISIN to its FIGI, ticker, and exchange details. Optionally filter by exchange or MIC code.',
    IsinMappingSchema.shape,
    async (params) => {
      const { isin, exchCode, micCode } = IsinMappingSchema.parse(params);
      const job: Record<string, string> = { idType: 'ID_ISIN', idValue: isin };
      if (exchCode) job.exchCode = exchCode;
      if (micCode) job.micCode = micCode;
      const data = await figiPost('mapping', [job], { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'figi_cusip_to_ticker',
    'Convenience tool: map a CUSIP to its FIGI, ticker, and exchange details. Optionally filter by exchange or MIC code.',
    CusipMappingSchema.shape,
    async (params) => {
      const { cusip, exchCode, micCode } = CusipMappingSchema.parse(params);
      const job: Record<string, string> = { idType: 'ID_CUSIP', idValue: cusip };
      if (exchCode) job.exchCode = exchCode;
      if (micCode) job.micCode = micCode;
      const data = await figiPost('mapping', [job], { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
