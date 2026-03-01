import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { factsetPost, wrapResponse, CacheTTL } from '../client.js';
import { BatchRequestSchema } from '../schemas/batch.js';

export function registerBatchTools(server: McpServer) {
  server.tool(
    'factset_batch_request',
    'Execute batch API requests for multiple FactSet endpoints in a single call. Reduces round trips when querying multiple data types simultaneously. Each request specifies an endpoint path and parameters.',
    BatchRequestSchema.shape,
    async (params) => {
      const { requests } = BatchRequestSchema.parse(params);
      const body = { requests };
      const data = await factsetPost('batch/v1/run', body, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
