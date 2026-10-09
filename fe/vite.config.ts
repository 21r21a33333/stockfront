import { fileURLToPath, URL } from "node:url";

import react from "@vitejs/plugin-react";
import { configDefaults, defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    port: process.env.PORT ? Number(process.env.PORT) : undefined,
    proxy: {
      "/api": {
        target: process.env.STOCKFRONT_API_TARGET ?? "http://127.0.0.1:3100",
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ""),
      },
    },
  },
  css: { modules: { localsConvention: "camelCaseOnly" } },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          query: ["@tanstack/react-query"],
          web3: ["viem", "wagmi"],
        },
      },
    },
  },
  test: {
    exclude: [...configDefaults.exclude, "e2e/**"],
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    css: true,
  },
});
