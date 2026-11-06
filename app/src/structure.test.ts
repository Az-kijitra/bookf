// Structure guards (same idea as guif): the page stays a thin composition layer, and the
// back end never impersonates a browser when talking to external services.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const SRC = join(__dirname);
const RUST = join(__dirname, "..", "src-tauri", "src");
const PAGE_LIMIT = 400;

const lines = (file: string) => readFileSync(file, "utf8").split(/\r?\n/).length;

describe("structure", () => {
  it(`routes/+page.svelte is at most ${PAGE_LIMIT} lines`, () => {
    expect(lines(join(SRC, "routes", "+page.svelte"))).toBeLessThanOrEqual(PAGE_LIMIT);
  });

  it("the back end does not fake a browser or shell out to curl", () => {
    for (const f of readdirSync(RUST).filter((f) => f.endsWith(".rs"))) {
      const code = readFileSync(join(RUST, f), "utf8");
      expect(code, f).not.toMatch(/Mozilla\/5\.0|sec-ch-ua|curl\.exe/);
    }
  });
});
