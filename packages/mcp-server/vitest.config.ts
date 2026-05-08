import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
    // Use the bundler module resolution matching tsconfig.
    server: {
      sourcemap: "inline",
    },
  },
  resolve: {
    // Allow .js extension imports (TypeScript ESM pattern) to resolve .ts files.
    extensions: [".ts", ".js"],
  },
});
