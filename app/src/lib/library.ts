import { emptyBook, type Book, type Candidate, type LookupResult } from "./types";

export type Filter =
  | { kind: "all" }
  | { kind: "status"; value: string }
  | { kind: "media"; value: string }
  | { kind: "rating"; value: number }
  | { kind: "year"; value: string };

export type SortKey = "completed_at" | "title" | "author" | "rating" | "media_type" | "status";
export interface Sort {
  key: SortKey;
  desc: boolean;
}

export const SORT_LABELS: Record<SortKey, string> = {
  completed_at: "読了日",
  title: "タイトル",
  author: "著者",
  rating: "評価",
  media_type: "種別",
  status: "状態",
};

export interface SideItem {
  label: string;
  filter: Filter;
  count: number;
  /** Section heading rows are not selectable. */
  heading?: boolean;
}

export function yearOf(b: Book): string | null {
  return b.completed_at ? b.completed_at.slice(0, 4) : null;
}

export function matchesFilter(b: Book, f: Filter): boolean {
  switch (f.kind) {
    case "all":
      return true;
    case "status":
      return b.status === f.value;
    case "media":
      return b.media_type === f.value;
    case "rating":
      return b.rating === f.value;
    case "year":
      return yearOf(b) === f.value;
  }
}

/** Space-separated words, all must appear in title / subtitle / author / publisher / notes / ISBN (case and width insensitive). */
export function matchesQuery(b: Book, q: string): boolean {
  const words = normalize(q).split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const hay = normalize([b.title, b.subtitle, b.author, b.publisher, b.notes, b.isbn].filter(Boolean).join(" "));
  return words.every((w) => hay.includes(w));
}

export function normalize(s: string): string {
  return s.normalize("NFKC").toLowerCase();
}

function cmpStr(a: string | null, b: string | null): number {
  // Empty values always go last regardless of direction is handled by the caller.
  return (a ?? "").localeCompare(b ?? "", "ja");
}

export function sortBooks(books: Book[], sort: Sort): Book[] {
  const dir = sort.desc ? -1 : 1;
  const out = [...books];
  out.sort((a, b) => {
    let c = 0;
    switch (sort.key) {
      case "rating":
        c = a.rating - b.rating;
        break;
      case "completed_at": {
        const av = a.completed_at ?? a.started_at;
        const bv = b.completed_at ?? b.started_at;
        if (!av && bv) return 1;
        if (av && !bv) return -1;
        c = cmpStr(av, bv);
        break;
      }
      default:
        c = cmpStr(a[sort.key], b[sort.key]);
    }
    if (c === 0) c = (a.id ?? 0) - (b.id ?? 0);
    return c * dir;
  });
  return out;
}

function countBy<T>(books: Book[], key: (b: Book) => T | null): Map<T, number> {
  const m = new Map<T, number>();
  for (const b of books) {
    const k = key(b);
    if (k === null || k === "") continue;
    m.set(k, (m.get(k) ?? 0) + 1);
  }
  return m;
}

export function sideItems(books: Book[], statusOrder: string[], mediaOrder: string[]): SideItem[] {
  const items: SideItem[] = [{ label: "すべて", filter: { kind: "all" }, count: books.length }];
  const ordered = <T extends string>(m: Map<T, number>, order: string[]) =>
    [...m.keys()].sort((a, b) => {
      const ia = order.indexOf(a), ib = order.indexOf(b);
      return (ia < 0 ? 999 : ia) - (ib < 0 ? 999 : ib) || a.localeCompare(b, "ja");
    });

  const st = countBy(books, (b) => b.status);
  items.push({ label: "ステータス", filter: { kind: "all" }, count: 0, heading: true });
  for (const s of ordered(st, statusOrder)) items.push({ label: s, filter: { kind: "status", value: s }, count: st.get(s)! });

  const md = countBy(books, (b) => b.media_type);
  items.push({ label: "種別", filter: { kind: "all" }, count: 0, heading: true });
  for (const s of ordered(md, mediaOrder)) items.push({ label: s, filter: { kind: "media", value: s }, count: md.get(s)! });

  const rt = countBy(books, (b) => b.rating);
  items.push({ label: "評価", filter: { kind: "all" }, count: 0, heading: true });
  for (let r = 5; r >= 0; r--) {
    if (rt.has(r)) items.push({ label: r === 0 ? "未評価" : "★".repeat(r), filter: { kind: "rating", value: r }, count: rt.get(r)! });
  }

  const yr = countBy(books, yearOf);
  items.push({ label: "読了年", filter: { kind: "all" }, count: 0, heading: true });
  for (const y of [...yr.keys()].sort().reverse()) items.push({ label: `${y}年`, filter: { kind: "year", value: y }, count: yr.get(y)! });
  return items;
}

export function sameFilter(a: Filter, b: Filter): boolean {
  return a.kind === b.kind && (a as { value?: unknown }).value === (b as { value?: unknown }).value;
}

/** Fills a book with a looked-up candidate. Existing links are kept (they often point at the exact edition, e.g. Audible). */
export function applyCandidate(book: Book, c: Candidate): Book {
  return {
    ...book,
    title: c.title || book.title,
    subtitle: c.subtitle ?? book.subtitle,
    author: c.author ?? book.author,
    publisher: c.publisher ?? book.publisher,
    published_date: c.published_date ?? book.published_date,
    isbn: c.isbn ?? book.isbn,
    cover_url: c.cover_url ?? book.cover_url,
    cover_local_path: c.cover_url && c.cover_url !== book.cover_url ? null : book.cover_local_path,
    link_url: book.link_url || c.link_url,
  };
}

export function today(d: Date = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** Clicking the current sort column flips the direction; a new column starts newest / best first for dates and ratings. */
export function nextSort(sort: Sort, key: SortKey): Sort {
  return sort.key === key ? { key, desc: !sort.desc } : { key, desc: key === "completed_at" || key === "rating" };
}

/** `s` key: the next column in SORT_LABELS order. */
export function cycleSort(sort: Sort): Sort {
  const keys = Object.keys(SORT_LABELS) as SortKey[];
  return nextSort(sort, keys[(keys.indexOf(sort.key) + 1) % keys.length]);
}

/** `i` clamped to a list of `len` items (0 for an empty list). */
export function clampIndex(i: number, len: number): number {
  return Math.max(0, Math.min(len - 1, i));
}

/** The sidebar filter `delta` selectable rows away from `current` (headings are skipped). */
export function stepFilter(items: SideItem[], current: Filter, delta: number): Filter {
  const selectable = items.filter((s) => !s.heading);
  if (selectable.length === 0) return current;
  const at = Math.max(0, selectable.findIndex((s) => sameFilter(s.filter, current)));
  return selectable[clampIndex(at + delta, selectable.length)].filter;
}

export interface LibraryStats {
  read: number;
  /** Average of the rated books, one decimal, or "-" when none is rated. */
  avgRating: string;
}

export function libraryStats(books: Book[]): LibraryStats {
  const rated = books.filter((b) => b.rating > 0);
  return {
    read: books.filter((b) => b.status === "読了").length,
    avgRating: rated.length ? (rated.reduce((s, b) => s + b.rating, 0) / rated.length).toFixed(1) : "-",
  };
}

/** The new book for a lookup dialog result; 読了 / 読書中 get today's date. */
export function bookFromLookup(r: LookupResult, day: string): Book {
  const b: Book = { ...emptyBook(), title: r.title, status: r.status, media_type: r.media_type };
  if (r.status === "読了") b.completed_at = day;
  if (r.status === "読書中") b.started_at = day;
  return r.candidate ? applyCandidate(b, r.candidate) : b;
}

/** Initial search words when looking up an existing book: the ISBN, else the title (which may be a URL). */
export function suggestQuery(b: Book): string {
  return b.isbn || b.title.trim();
}

export function shortDate(d: string | null): string {
  return d ? d.replaceAll("-", "/") : "";
}
