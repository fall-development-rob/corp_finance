import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { egnyteFetch, CacheTTL } from '../client.js';
import {
  EgnyteListFilesSchema,
  EgnyteGetFileMetadataSchema,
  EgnyteSearchSchema,
  EgnyteDownloadFileSchema,
} from '../schemas/files.js';
import { wrapResponse } from '@robotixai/mcp-utils';

function encodePath(filePath: string): string {
  return filePath.split('/').map(encodeURIComponent).join('/');
}

function isCredentialsMissing(): boolean {
  return !process.env.EGNYTE_API_KEY || !process.env.EGNYTE_DOMAIN;
}

const CRED_MSG =
  'Set EGNYTE_DOMAIN (e.g. "acme") and EGNYTE_API_KEY in environment to enable Egnyte tools. ' +
  'Egnyte uses domain-scoped bearer tokens; see https://developers.egnyte.com.';

export function registerEgnyteTools(server: McpServer): void {
  server.tool(
    'egnyte_list_files',
    'List files in an Egnyte folder. Returns name, size, modified time, and content hash for each entry. ' +
    'Optionally recursive. Requires EGNYTE_DOMAIN and EGNYTE_API_KEY (paid vendor).',
    EgnyteListFilesSchema.shape,
    async (params) => {
      const validated = EgnyteListFilesSchema.parse(params);
      if (isCredentialsMissing()) {
        return wrapResponse({ credentials_required: true, vendor: 'egnyte', env_var: 'EGNYTE_API_KEY', message: CRED_MSG, tool: 'egnyte_list_files', requested_params: validated });
      }
      const { folder_path, recursive, limit } = validated;
      const endpoint = `/fs${encodePath(folder_path)}`;
      const data = await egnyteFetch(endpoint, { list_content: true, count: limit, maxdepth: recursive ? 1000 : 1 }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'egnyte_get_file_metadata',
    'Fetch metadata for a single Egnyte file: type, size, sha-1, version count, and sharing status. ' +
    'Requires EGNYTE_DOMAIN and EGNYTE_API_KEY (paid vendor).',
    EgnyteGetFileMetadataSchema.shape,
    async (params) => {
      const validated = EgnyteGetFileMetadataSchema.parse(params);
      if (isCredentialsMissing()) {
        return wrapResponse({ credentials_required: true, vendor: 'egnyte', env_var: 'EGNYTE_API_KEY', message: CRED_MSG, tool: 'egnyte_get_file_metadata', requested_params: validated });
      }
      const endpoint = `/fs${encodePath(validated.file_path)}`;
      const data = await egnyteFetch(endpoint, {}, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'egnyte_search',
    'Full-text search across Egnyte content with optional folder-scope and file-type filter. ' +
    'Requires EGNYTE_DOMAIN and EGNYTE_API_KEY (paid vendor).',
    EgnyteSearchSchema.shape,
    async (params) => {
      const validated = EgnyteSearchSchema.parse(params);
      if (isCredentialsMissing()) {
        return wrapResponse({ credentials_required: true, vendor: 'egnyte', env_var: 'EGNYTE_API_KEY', message: CRED_MSG, tool: 'egnyte_search', requested_params: validated });
      }
      const { query, folder_scope, file_type, limit } = validated;
      const body: Record<string, unknown> = { query, count: limit };
      if (folder_scope) body['folder'] = folder_scope;
      if (file_type !== 'all') body['type'] = file_type;
      const data = await egnyteFetch('/search', {}, { method: 'POST', body, cacheTtl: 0 });
      return wrapResponse(data);
    },
  );

  server.tool(
    'egnyte_download_file',
    'Get Egnyte file bytes (base64) or a temporary download URL. Set return_content=false for the URL. ' +
    'Requires EGNYTE_DOMAIN and EGNYTE_API_KEY (paid vendor).',
    EgnyteDownloadFileSchema.shape,
    async (params) => {
      const validated = EgnyteDownloadFileSchema.parse(params);
      if (isCredentialsMissing()) {
        return wrapResponse({ credentials_required: true, vendor: 'egnyte', env_var: 'EGNYTE_API_KEY', message: CRED_MSG, tool: 'egnyte_download_file', requested_params: validated });
      }
      const endpoint = `/fs-content${encodePath(validated.file_path)}`;
      if (!validated.return_content) {
        const domain = process.env.EGNYTE_DOMAIN ?? '';
        const downloadUrl = `https://${domain}.egnyte.com/pubapi/v1${endpoint}`;
        return wrapResponse({ download_url: downloadUrl, note: 'Authenticate with Bearer token to access this URL.' });
      }
      const data = await egnyteFetch<string>(endpoint, {}, { cacheTtl: 0 });
      return wrapResponse({ content_base64: data });
    },
  );
}
