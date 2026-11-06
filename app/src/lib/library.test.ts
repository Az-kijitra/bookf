import { describe, expect, it } from "vitest";
import { applyCandidate, matchesQuery, sideItems, sortBooks } from "./library";
import { emptyBook, type Book } from "./types";

const book = (p: Partial<Book>): Book => ({ ...emptyBook(), ...p });

describe("library", () => {
  const books = [
    book({ id: 1, title: "具体と抽象", author: "細谷 功", rating: 3, completed_at: "2024-06-25", media_type: "オーディブル" }),
    book({ id: 2, title: "言語沼", author: "水野 太貴", rating: 2, completed_at: "2024-07-14" }),
    book({ id: 3, title: "世界哲学史１", status: "読書中", rating: 0 }),
  ];

  it("searches width- and case-insensitively", () => {
    expect(matchesQuery(books[2], "世界哲学史1")).toBe(true);
    expect(matchesQuery(books[0], "細谷 抽象")).toBe(true);
    expect(matchesQuery(books[0], "沼")).toBe(false);
  });

  it("sorts by completion date with undated books last", () => {
    expect(sortBooks(books, { key: "completed_at", desc: true }).map((b) => b.id)).toEqual([2, 1, 3]);
    expect(sortBooks(books, { key: "rating", desc: true }).map((b) => b.id)).toEqual([1, 2, 3]);
  });

  it("builds sidebar groups", () => {
    const items = sideItems(books, ["読了", "読書中"], ["書籍"]);
    expect(items[0]).toMatchObject({ label: "すべて", count: 3 });
    expect(items.find((i) => i.label === "読書中")?.count).toBe(1);
    expect(items.find((i) => i.label === "2024年")?.count).toBe(2);
  });

  it("keeps an existing link when applying a candidate", () => {
    const b = applyCandidate(book({ title: "x", link_url: "https://audible" }), {
      title: "正式", subtitle: null, author: "著者", publisher: "P", published_date: null,
      isbn: "9784000000000", cover_url: "https://c", link_url: "https://amazon", description: null,
    });
    expect(b).toMatchObject({ title: "正式", link_url: "https://audible", isbn: "9784000000000", cover_local_path: null });
  });
});
