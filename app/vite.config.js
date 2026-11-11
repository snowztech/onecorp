import { defineConfig } from "vite";

// Tauri loads dist/ in release builds and the dev server at :1420 in `cargo tauri dev`.
export default defineConfig({
  root: "ui",
  base: "./",
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  // The DiceBear face sets are one 516 kB chunk, loaded only when someone picks them.
  build: { outDir: "../dist", emptyOutDir: true, target: "safari15", chunkSizeWarningLimit: 600 },
});
