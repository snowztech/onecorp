import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

// Paths are from this file, not the working directory, so tests can run from the repo root.
const here = path => fileURLToPath(new URL(path, import.meta.url));

// Tauri loads dist/ in release builds and the dev server at :1420 in `cargo tauri dev`.
export default defineConfig({
  root: here("ui"),
  base: "./",
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  // The DiceBear face sets are one 516 kB chunk, loaded only when someone picks them.
  build: { outDir: here("dist"), emptyOutDir: true, target: "safari15", chunkSizeWarningLimit: 600 },
});
