import { invoke } from "@tauri-apps/api/core";
import type { Book, Candidate, ImportResult, LibraryInfo } from "./types";

export const api = {
  libraryInfo: () => invoke<LibraryInfo>("library_info"),
  listBooks: () => invoke<Book[]>("list_books"),
  saveBook: (book: Book) => invoke<Book>("save_book", { book }),
  deleteBook: (id: number) => invoke<void>("delete_book", { id }),
  importNotionCsv: (path: string) => invoke<ImportResult>("import_notion_csv", { path }),
  searchBooks: (query: string) => invoke<Candidate[]>("search_books", { query }),
  fetchLinkMetadata: (url: string) => invoke<Candidate>("fetch_link_metadata", { url }),
  fetchCover: (id: number) => invoke<Book>("fetch_cover", { id }),
  setCoverFromFile: (id: number, filePath: string) => invoke<Book>("set_cover_from_file", { id, filePath }),
  setCoverFromUrl: (id: number, url: string) => invoke<Book>("set_cover_from_url", { id, url }),
  removeCover: (id: number) => invoke<Book>("remove_cover", { id }),
  coverData: (rel: string) => invoke<string | null>("cover_data", { rel }),
  setDbPath: (path: string, copyCurrent: boolean) => invoke<LibraryInfo>("set_db_path", { path, copyCurrent }),
};
