// Menu bar contents (ファイル / 編集 / 表示). The menu is rebuilt from the current state,
// and each item calls back into +page.svelte through `MenuActions`.

import type { Sort, SortKey } from "$lib/library";

export interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  disabled?: boolean;
  checked?: boolean;
  type?: "separator";
  action?: () => void;
}

export interface MenuGroup {
  id: string;
  name: string;
  accessKey: string;
  items: MenuItem[];
}

export interface MenuState {
  hasSelection: boolean;
  hasLink: boolean;
  sort: Sort;
  /** A sidebar filter or a search word is active. */
  filtered: boolean;
}

export interface MenuActions {
  addBook(): void;
  importCsv(): void;
  openDb(): void;
  moveDb(): void;
  lookup(): void;
  fetchFromLink(): void;
  changeCover(): void;
  openLink(): void;
  deleteBook(): void;
  setSort(key: SortKey): void;
  reverseSort(): void;
  clearFilter(): void;
}

const sep = (id: string): MenuItem => ({ id, label: "", type: "separator" });

const MENU_SORT_KEYS: [SortKey, string][] = [
  ["completed_at", "読了日"],
  ["title", "タイトル"],
  ["author", "著者"],
  ["rating", "評価"],
];

export function buildMenus(s: MenuState, a: MenuActions): MenuGroup[] {
  const noBook = !s.hasSelection;
  const noLink = noBook || !s.hasLink;
  return [
    {
      id: "file",
      name: "ファイル",
      accessKey: "F",
      items: [
        { id: "add_book", label: "本を追加", shortcut: "Ctrl+N", action: a.addBook },
        { id: "import_csv", label: "Notion CSV 取り込み…", shortcut: "Ctrl+I", action: a.importCsv },
        sep("sep_file_1"),
        { id: "open_db", label: "データベースを開く…", shortcut: "Ctrl+O", action: a.openDb },
        { id: "move_db", label: "保存先を移動…", action: a.moveDb },
      ],
    },
    {
      id: "edit",
      name: "編集",
      accessKey: "E",
      items: [
        { id: "search_metadata", label: "書誌情報を検索…", shortcut: "u", disabled: noBook, action: a.lookup },
        { id: "fetch_from_link", label: "リンク先から書誌情報を取得", disabled: noLink, action: a.fetchFromLink },
        { id: "change_cover", label: "表紙画像を変更…", disabled: noBook, action: a.changeCover },
        { id: "open_link", label: "リンクを開く", shortcut: "o / Enter", disabled: noLink, action: a.openLink },
        sep("sep_edit_1"),
        { id: "delete_book", label: "本を削除", shortcut: "Delete", disabled: noBook, action: a.deleteBook },
      ],
    },
    {
      id: "view",
      name: "表示",
      accessKey: "V",
      items: [
        ...MENU_SORT_KEYS.map(([key, label]) => ({
          id: `sort_${key}`,
          label: `並び替え: ${label}`,
          checked: s.sort.key === key,
          action: () => a.setSort(key),
        })),
        sep("sep_view_1"),
        { id: "toggle_sort_order", label: s.sort.desc ? "昇順に切り替え" : "降順に切り替え", shortcut: "r", action: a.reverseSort },
        sep("sep_view_2"),
        { id: "clear_filter", label: "すべての絞り込みを解除", disabled: !s.filtered, action: a.clearFilter },
      ],
    },
  ];
}
