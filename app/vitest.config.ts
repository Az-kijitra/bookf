import { defineConfig } from "vitest/config";

// Pure-logic tests only (src/lib/**/*.test.ts); kept apart from the SvelteKit build config.
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
  },
});
