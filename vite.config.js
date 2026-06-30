import { defineConfig } from "vite";

// Tauri expects a fixed dev port and ignores src-tauri during HMR.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  // Produce a relative-path build that Tauri can load from disk.
  base: "./",
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});
