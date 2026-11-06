import { describe, expect, it, vi } from "vitest";
import { buildMenus, type MenuActions, type MenuItem, type MenuState } from "./menu";

const actions = (): MenuActions => ({
  addBook: vi.fn(),
  importCsv: vi.fn(),
  openDb: vi.fn(),
  moveDb: vi.fn(),
  lookup: vi.fn(),
  fetchFromLink: vi.fn(),
  changeCover: vi.fn(),
  openLink: vi.fn(),
  deleteBook: vi.fn(),
  setSort: vi.fn(),
  reverseSort: vi.fn(),
  clearFilter: vi.fn(),
});

const state: MenuState = { hasSelection: true, hasLink: true, sort: { key: "completed_at", desc: true }, filtered: false };

function item(menus: ReturnType<typeof buildMenus>, id: string): MenuItem {
  const found = menus.flatMap((m) => m.items).find((i) => i.id === id);
  if (!found) throw new Error(`no menu item ${id}`);
  return found;
}

describe("buildMenus", () => {
  it("has the three menus with access keys", () => {
    expect(buildMenus(state, actions()).map((m) => [m.name, m.accessKey])).toEqual([
      ["ファイル", "F"],
      ["編集", "E"],
      ["表示", "V"],
    ]);
  });

  it("disables book items without a selection and link items without a link", () => {
    const none = buildMenus({ ...state, hasSelection: false, hasLink: false }, actions());
    for (const id of ["search_metadata", "change_cover", "delete_book", "open_link", "fetch_from_link"]) {
      expect(item(none, id).disabled).toBe(true);
    }
    const noLink = buildMenus({ ...state, hasLink: false }, actions());
    expect(item(noLink, "delete_book").disabled).toBe(false);
    expect(item(noLink, "open_link").disabled).toBe(true);
    expect(item(noLink, "fetch_from_link").disabled).toBe(true);
  });

  it("checks the current sort and offers the opposite direction", () => {
    const m = buildMenus({ ...state, sort: { key: "title", desc: false } }, actions());
    expect(item(m, "sort_title").checked).toBe(true);
    expect(item(m, "sort_completed_at").checked).toBe(false);
    expect(item(m, "toggle_sort_order").label).toBe("降順に切り替え");
  });

  it("enables clearing filters only while filtered", () => {
    expect(item(buildMenus(state, actions()), "clear_filter").disabled).toBe(true);
    expect(item(buildMenus({ ...state, filtered: true }, actions()), "clear_filter").disabled).toBe(false);
  });

  it("wires items to their actions", () => {
    const a = actions();
    const m = buildMenus(state, a);
    item(m, "sort_rating").action!();
    item(m, "move_db").action!();
    expect(a.setSort).toHaveBeenCalledWith("rating");
    expect(a.moveDb).toHaveBeenCalledOnce();
  });

  it("uses unique ids", () => {
    const ids = buildMenus(state, actions()).flatMap((m) => m.items.map((i) => i.id));
    expect(new Set(ids).size).toBe(ids.length);
  });
});
