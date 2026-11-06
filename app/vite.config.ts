import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

// Unit tests use the standalone vitest.config.ts.
export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 5174,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
