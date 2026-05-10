/**
 * Name resolver — translates between bare tool names and wire-prefixed names.
 *
 * Bare name: "option_pricer"
 * Wire name: "mcp__plugin_cfa-core_cfa-core__option_pricer"
 */
import type { MCPServerConfig } from "../types.js";

export interface ResolvedTool {
  serverName: string;
  prefix: string;
  wireName: string;
}

/**
 * Builds a bare→ResolvedTool map from the server configs and their
 * reported wire tool names. Throws on duplicate bare names across servers.
 */
export function buildNameMap(
  servers: MCPServerConfig[],
  wireToolsByServer: Map<string, string[]>,
): Map<string, ResolvedTool> {
  const map = new Map<string, ResolvedTool>();

  for (const server of servers) {
    const wireNames = wireToolsByServer.get(server.name) ?? [];
    for (const wireName of wireNames) {
      const bareName = extractBareFromWire(wireName, server.prefix);
      if (map.has(bareName)) {
        throw new Error(
          `Tool name collision: bare name "${bareName}" is provided by both ` +
            `"${map.get(bareName)!.serverName}" and "${server.name}"`,
        );
      }
      map.set(bareName, { serverName: server.name, prefix: server.prefix, wireName });
    }
  }

  return map;
}

/**
 * Resolves a bare tool name to its server config and wire name.
 * Returns undefined if the bare name is not found.
 */
export function resolveBareToWire(
  nameMap: Map<string, ResolvedTool>,
  bareName: string,
): ResolvedTool | undefined {
  return nameMap.get(bareName);
}

/**
 * Extracts the bare name from a wire name by stripping the server prefix.
 * If the wire name does not start with the prefix, returns the wire name as-is.
 */
export function extractBareFromWire(wireName: string, prefix: string): string {
  if (wireName.startsWith(prefix)) {
    return wireName.slice(prefix.length);
  }
  return wireName;
}
