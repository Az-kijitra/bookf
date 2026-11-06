import { describe, expect, it } from "vitest";
import {
  applyCandidate,
  bookFromLookup,
  clampIndex,
  cycleSort,
  libraryStats,
  matchesFilter,
  matchesQuery,
  nextSort,
  shortDate,
  sideItems,
  sortBooks,
  stepFilter,
  suggestQuery,
  today,
} from "./library";
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

  it("keeps the title and cached cover when the candidate has neither", () => {
    const b = applyCandidate(book({ title: "元の題", cover_url: "https://c", cover_local_path: "covers/1.jpg" }), {
      title: "", subtitle: null, author: null, publisher: null, published_date: null,
      isbn: null, cover_url: "https://c", link_url: null, description: null,
    });
    expect(b).toMatchObject({ title: "元の題", cover_local_path: "covers/1.jpg" });
  });

  it("filters by status, media, rating and year", () => {
    expect(books.filter((b) => matchesFilter(b, { kind: "media", value: "オーディブル" })).map((b) => b.id)).toEqual([1]);
    expect(books.filter((b) => matchesFilter(b, { kind: "status", value: "読書中" })).map((b) => b.id)).toEqual([3]);
    expect(books.filter((b) => matchesFilter(b, { kind: "rating", value: 0 })).map((b) => b.id)).toEqual([3]);
    expect(books.filter((b) => matchesFilter(b, { kind: "year", value: "2024" })).map((b) => b.id)).toEqual([1, 2]);
  });

  it("sorts by text columns in Japanese order and breaks ties by id", () => {
    const kana = [book({ id: 1, title: "うさぎ" }), book({ id: 2, title: "アヒル" }), book({ id: 3, title: "いぬ" })];
    expect(sortBooks(kana, { key: "title", desc: false }).map((b) => b.id)).toEqual([2, 3, 1]);
    const same = [book({ id: 9, title: "同" }), book({ id: 4, title: "同" })];
    expect(sortBooks(same, { key: "title", desc: false }).map((b) => b.id)).toEqual([4, 9]);
  });

  it("flips the direction on the same column and picks a default on a new one", () => {
    expect(nextSort({ key: "title", desc: false }, "title")).toEqual({ key: "title", desc: true });
    expect(nextSort({ key: "title", desc: false }, "rating")).toEqual({ key: "rating", desc: true });
    expect(nextSort({ key: "rating", desc: true }, "author")).toEqual({ key: "author", desc: false });
    expect(cycleSort({ key: "completed_at", desc: true })).toEqual({ key: "title", desc: false });
    expect(cycleSort({ key: "status", desc: false }).key).toBe("completed_at");
  });

  it("clamps indexes", () => {
    expect(clampIndex(-3, 5)).toBe(0);
    expect(clampIndex(9, 5)).toBe(4);
    expect(clampIndex(2, 0)).toBe(0);
  });

  it("steps through sidebar filters skipping headings", () => {
    const items = sideItems(books, ["読了", "読書中"], ["書籍", "オーディブル"]);
    expect(stepFilter(items, { kind: "all" }, 1)).toEqual({ kind: "status", value: "読了" });
    expect(stepFilter(items, { kind: "status", value: "読了" }, -1)).toEqual({ kind: "all" });
    expect(stepFilter(items, { kind: "all" }, -1)).toEqual({ kind: "all" });
    expect(stepFilter(items, { kind: "year", value: "2024" }, 1)).toEqual({ kind: "year", value: "2024" });
    expect(stepFilter([], { kind: "all" }, 1)).toEqual({ kind: "all" });
  });

  it("computes the status bar numbers", () => {
    expect(libraryStats(books)).toEqual({ read: 2, avgRating: "2.5" });
    expect(libraryStats([])).toEqual({ read: 0, avgRating: "-" });
  });

  it("creates a book from a lookup result with today's date by status", () => {
    const read = bookFromLookup({ candidate: null, title: "手入力", status: "読了", media_type: "Kindle" }, "2026-11-06");
    expect(read).toMatchObject({ id: null, title: "手入力", media_type: "Kindle", completed_at: "2026-11-06", started_at: null });
    const reading = bookFromLookup({ candidate: null, title: "x", status: "読書中", media_type: "書籍" }, "2026-11-06");
    expect(reading).toMatchObject({ started_at: "2026-11-06", completed_at: null });
    const todo = bookFromLookup(
      { candidate: { title: "正式", subtitle: null, author: "著", publisher: null, published_date: null, isbn: null, cover_url: null, link_url: "https://a", description: null }, title: "q", status: "未着手", media_type: "書籍" },
      "2026-11-06",
    );
    expect(todo).toMatchObject({ title: "正式", author: "著", link_url: "https://a", completed_at: null, started_at: null });
  });

  it("formats helpers", () => {
    expect(today(new Date(2026, 0, 5))).toBe("2026-01-05");
    expect(shortDate("2024-06-25")).toBe("2024/06/25");
    expect(shortDate(null)).toBe("");
    expect(suggestQuery(book({ title: " 題 ", isbn: "9784121022721" }))).toBe("9784121022721");
    expect(suggestQuery(book({ title: " 題 " }))).toBe("題");
  });
});
