export interface Book {
  id: number | null;
  title: string;
  subtitle: string | null;
  author: string | null;
  publisher: string | null;
  published_date: string | null;
  isbn: string | null;
  status: string;
  media_type: string;
  rating: number;
  started_at: string | null;
  completed_at: string | null;
  link_url: string | null;
  cover_url: string | null;
  cover_local_path: string | null;
  notes: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface Candidate {
  title: string;
  subtitle: string | null;
  author: string | null;
  publisher: string | null;
  published_date: string | null;
  isbn: string | null;
  cover_url: string | null;
  link_url: string | null;
  description: string | null;
}

/** What the lookup dialog returns: the chosen candidate (null = manual entry with the typed title). */
export interface LookupResult {
  candidate: Candidate | null;
  title: string;
  status: string;
  media_type: string;
}

export interface LibraryInfo {
  db_path: string;
  count: number;
}

export interface ImportResult {
  imported: number;
  skipped: number;
}

export const STATUSES = ["読了", "読書中", "未着手", "積読", "中断"];
export const MEDIA_TYPES = ["書籍", "書籍（図書館）", "Kindle", "Kindle Unlimited", "オーディブル", "学術誌", "記事"];

export function emptyBook(): Book {
  return {
    id: null,
    title: "",
    subtitle: null,
    author: null,
    publisher: null,
    published_date: null,
    isbn: null,
    status: "読了",
    media_type: "書籍",
    rating: 0,
    started_at: null,
    completed_at: null,
    link_url: null,
    cover_url: null,
    cover_local_path: null,
    notes: null,
    created_at: null,
    updated_at: null,
  };
}
