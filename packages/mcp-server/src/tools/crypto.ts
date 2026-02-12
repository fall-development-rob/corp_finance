import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  valueToken,
  analyzeDefi,
} from "@rob-otixai/corp-finance-bindings";
import {
  TokenValuationSchema,
  DefiYieldSchema,
} from "../schemas/crypto.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCryptoTools(server: McpServer) {
  server.tool(
    "token_valuation",
    "Comprehensive token/protocol valuation using on-chain metrics. Computes NVT ratio (network value to transaction volume), P/S ratio, fully-diluted vs circulating valuation, DCF of protocol revenue with terminal value, and relative valuation against comparable protocols. Supports DeFi protocols with TVL analysis.",
    TokenValuationSchema.shape,
    async (params) => {
      const validated = TokenValuationSchema.parse(coerceNumbers(params));
      const result = valueToken(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "defi_analysis",
    "Analyze DeFi yield opportunities: yield farming (APR/APY with gas-adjusted net yield), impermanent loss (IL percentage and net position after fees), staking (effective yield after validator commission and slashing risk), and liquidity pool analysis (fee income, share of pool, projected annual income). Supports all major DeFi primitives.",
    DefiYieldSchema.shape,
    async (params) => {
      const validated = DefiYieldSchema.parse(coerceNumbers(params));
      const result = analyzeDefi(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
