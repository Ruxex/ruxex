import path from "node:path";
import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

const rootDir = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(rootDir, "../..");
const protocolRoot = path.join(
  repositoryRoot,
  "crates/app-server-protocol/schema/typescript",
);

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@genex/protocol": protocolRoot,
    },
  },
  server: {
    fs: {
      allow: [repositoryRoot],
    },
  },
});
