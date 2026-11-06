// Native file pickers used by the ファイル menu. Each returns the chosen path, or null when cancelled.

import { open, save } from "@tauri-apps/plugin-dialog";

const DB_FILTER = [{ name: "bookf データベース", extensions: ["db"] }];

const single = (p: string | string[] | null): string | null => (typeof p === "string" ? p : null);

export async function pickNotionCsv(): Promise<string | null> {
  return single(await open({ title: "Notion からエクスポートした CSV", filters: [{ name: "CSV", extensions: ["csv"] }] }));
}

export async function pickDatabase(): Promise<string | null> {
  return single(await open({ title: "データベースを開く", filters: DB_FILTER }));
}

export async function pickDatabaseTarget(): Promise<string | null> {
  return save({ title: "データベースの保存先（例: Google ドライブのフォルダー）", defaultPath: "books.db", filters: DB_FILTER });
}
