// Release scripts and the documents that name the release (SPEC-0600-0001, SPEC-0500-0001).

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { checkVersions, readVersions } from "../scripts/release/versions.mjs";
import { expectedInstallerName } from "../scripts/release/installer-name.mjs";
import { formatChecksumLine } from "../scripts/release/sha256.mjs";

const APP = join(__dirname, "..");
const ROOT = join(APP, "..");
const conf = JSON.parse(readFileSync(join(APP, "src-tauri", "tauri.conf.json"), "utf8"));
const read = (rel: string) => readFileSync(join(ROOT, rel), "utf8");

describe("release scripts", () => {
  it("all five version fields agree", () => {
    const result = checkVersions(readVersions(APP));
    expect(result.errors).toEqual([]);
  });

  it("reports a mismatch with the file name", () => {
    const v = { "tauri.conf.json": "0.1.0", "package.json": "0.1.0", "package-lock.json": "0.1.0", "Cargo.toml": "0.2.0", "Cargo.lock": "0.1" };
    const r = checkVersions(v);
    expect(r.ok).toBe(false);
    expect(r.errors.join("\n")).toMatch(/Cargo\.toml.*differs/);
    expect(r.errors.join("\n")).toMatch(/Cargo\.lock.*MAJOR\.MINOR\.PATCH/);
  });

  it("names the installer and checksum line like Tauri and sha256sum", () => {
    expect(expectedInstallerName({ productName: "bookf", version: "1.2.3" })).toBe("bookf_1.2.3_x64-setup.exe");
    expect(formatChecksumLine("ab12", "bookf_1.2.3_x64-setup.exe")).toBe("ab12 *bookf_1.2.3_x64-setup.exe");
  });
});

describe("documents follow the version", () => {
  const installer = expectedInstallerName(conf);

  it("CHANGELOG has a section for the current version", () => {
    expect(read("CHANGELOG.md")).toContain(`## ${conf.version}`);
  });

  it("README and the user manual name the current installer", () => {
    expect(read("README.md")).toContain(installer);
    expect(read("docs/user_manual.md")).toContain(installer);
  });

  it("the user manual title carries the version", () => {
    expect(read("docs/user_manual.md").split("\n")[0]).toContain(`(${conf.version})`);
  });
});
