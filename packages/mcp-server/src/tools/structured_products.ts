import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  priceStructuredNote,
  priceExotic,
} from "@robotixai/corp-finance-bindings";
import {
  StructuredNoteSchema,
  ExoticProductSchema,
} from "../schemas/structured_products.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerStructuredProductsTools(server: McpServer) {
  server.tool(
    "structured_note_pricing",
    "Price structured notes: capital-protected notes (zero-coupon bond + call option decomposition), yield enhancement / reverse convertibles (short put + bond, barrier analysis), participation notes (leveraged upside with optional cap/floor), and credit-linked notes (risky bond pricing with default/recovery). Returns issue price, component values, embedded option value, payoff scenarios, and risk metrics.",
    StructuredNoteSchema.shape,
    async (params) => {
      const validated = StructuredNoteSchema.parse(coerceNumbers(params));
      const result = priceStructuredNote(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "exotic_product_pricing",
    "Price exotic derivative products: autocallables (observation schedule with autocall/coupon/knock-in probabilities, expected life and return), barrier options (up/down and in/out with analytical pricing and Greeks), and digital/binary options (cash-or-nothing and asset-or-nothing with full Greeks). Uses closed-form solutions where available.",
    ExoticProductSchema.shape,
    async (params) => {
      const validated = ExoticProductSchema.parse(coerceNumbers(params));
      const result = priceExotic(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
