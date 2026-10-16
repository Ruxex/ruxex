import path from "node:path";
import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

const rootDir = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(rootDir, "../..");
const protocolRoot = path.join(
  repositoryRoot,
  "crates/app-server-protocol/schema/typescript-experimental",
);

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@ruxex/protocol": protocolRoot,
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    fs: {
      allow: [repositoryRoot],
    },
  },
});
