import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri expects a fixed dev port and a plain `dist` output directory.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2021",
    sourcemap: false,
    outDir: "dist",
    // Le icone sono molte: separarle evita un unico chunk oltre i 500 kB.
    rollupOptions: {
      output: {
        manualChunks: {
          icons: ["lucide"],
          prosemirror: [
            "prosemirror-state",
            "prosemirror-view",
            "prosemirror-model",
            "prosemirror-schema-list",
            "prosemirror-tables",
            "prosemirror-history",
            "prosemirror-keymap",
            "prosemirror-commands",
          ],
        },
      },
    },
  },
});
