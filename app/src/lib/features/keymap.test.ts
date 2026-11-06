import { describe, expect, it } from "vitest";
import { resolveKey, type KeyInput } from "./keymap";

const base: KeyInput = { key: "", ctrl: false, composing: false, dialogOpen: false, typing: false, inSearch: false, focus: "list" };
const press = (key: string, p: Partial<KeyInput> = {}) => resolveKey({ ...base, key, ...p });
const action = (key: string, p: Partial<KeyInput> = {}) => press(key, p)?.action ?? null;

describe("resolveKey", () => {
  it("ignores keys while a dialog is open or IME is composing", () => {
    expect(press("j", { dialogOpen: true })).toBeNull();
    expect(press("n", { ctrl: true, dialogOpen: true })).toBeNull();
    expect(press("j", { composing: true })).toBeNull();
  });

  it("handles Ctrl shortcuts everywhere, even while typing", () => {
    expect(action("n", { ctrl: true })).toEqual({ type: "addBook" });
    expect(action("I", { ctrl: true })).toEqual({ type: "importCsv" });
    expect(action("o", { ctrl: true, typing: true })).toEqual({ type: "openDb" });
    expect(press("f", { ctrl: true })).toEqual({ action: { type: "focusSearch" }, preventDefault: true });
    expect(press("a", { ctrl: true })).toBeNull();
  });

  it("leaves typed characters to the input", () => {
    expect(press("j", { typing: true })).toBeNull();
    expect(press("1", { typing: true })).toBeNull();
    expect(press("Delete", { typing: true })).toBeNull();
    expect(action("Escape", { typing: true })).toEqual({ type: "leaveInput" });
  });

  it("moves from the search box to the list with Enter or Down", () => {
    expect(action("Enter", { typing: true, inSearch: true })).toEqual({ type: "searchToList" });
    expect(action("ArrowDown", { typing: true, inSearch: true })).toEqual({ type: "searchToList" });
    expect(press("Enter", { typing: true, inSearch: false })).toBeNull();
  });

  it("switches panes and focuses the search", () => {
    expect(press("Tab")).toEqual({ action: { type: "toggleFocus" }, preventDefault: true });
    expect(action("/", { focus: "side" })).toEqual({ type: "focusSearch" });
  });

  it("navigates the sidebar", () => {
    const side = { focus: "side" as const };
    expect(action("j", side)).toEqual({ type: "sideMove", delta: 1 });
    expect(action("ArrowUp", side)).toEqual({ type: "sideMove", delta: -1 });
    expect(action("l", side)).toEqual({ type: "sideToList" });
    expect(action("Enter", side)).toEqual({ type: "sideToList" });
    expect(press("1", side)).toBeNull();
  });

  it("navigates the list", () => {
    expect(action("j")).toEqual({ type: "move", delta: 1 });
    expect(action("ArrowUp")).toEqual({ type: "move", delta: -1 });
    expect(action("PageDown")).toEqual({ type: "page", delta: 1 });
    expect(action("g")).toEqual({ type: "first" });
    expect(action("G")).toEqual({ type: "last" });
    expect(action("End")).toEqual({ type: "last" });
    expect(action("h")).toEqual({ type: "toSide" });
  });

  it("edits the selected book from the list or the detail pane", () => {
    for (const focus of ["list", "detail"] as const) {
      expect(action("0", { focus })).toEqual({ type: "rate", value: 0 });
      expect(action("5", { focus })).toEqual({ type: "rate", value: 5 });
      expect(action("e", { focus })).toEqual({ type: "editNotes" });
      expect(action("u", { focus })).toEqual({ type: "lookup" });
      expect(action("Delete", { focus })).toEqual({ type: "deleteBook" });
    }
    expect(press("6")).toBeNull();
    expect(action("o")).toEqual({ type: "openLink" });
    expect(action("Enter")).toEqual({ type: "openLink" });
    expect(action("s")).toEqual({ type: "cycleSort" });
    expect(action("r")).toEqual({ type: "reverseSort" });
  });

  it("suppresses the browser default only for movement keys", () => {
    expect(press("ArrowDown")?.preventDefault).toBe(true);
    expect(press("e")?.preventDefault).toBe(true);
    expect(press("3")?.preventDefault).toBe(false);
    expect(press("Enter")?.preventDefault).toBe(false);
  });
});
