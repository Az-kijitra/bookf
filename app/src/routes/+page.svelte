<script lang="ts">
  import "../app.css";
  import { onMount, tick } from "svelte";
  import { open as openDialog, save as saveDialog, ask } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import {
    applyCandidate,
    matchesFilter,
    matchesQuery,
    sameFilter,
    sideItems,
    sortBooks,
    SORT_LABELS,
    today,
    type Filter,
    type Sort,
    type SortKey,
  } from "$lib/library";
  import { emptyBook, MEDIA_TYPES, STATUSES, type Book, type LibraryInfo, type LookupResult } from "$lib/types";
  import DetailPane from "$lib/components/DetailPane.svelte";
  import LookupDialog from "$lib/components/LookupDialog.svelte";
  import CoverDialog from "$lib/components/CoverDialog.svelte";
  import MenuBar, { type MenuGroup } from "$lib/components/MenuBar.svelte";

  type Focus = "side" | "list" | "detail";

  let books = $state<Book[]>([]);
  let info = $state<LibraryInfo | null>(null);
  let filter = $state<Filter>({ kind: "all" });
  let query = $state("");
  let sort = $state<Sort>({ key: "completed_at", desc: true });
  let selectedId = $state<number | null>(null);
  let focus = $state<Focus>("list");
  let dialog = $state<null | { mode: "add" } | { mode: "update"; book: Book }>(null);
  let coverDialogBook = $state<Book | null>(null);
  let message = $state("");
  let covers = $state<Record<string, string>>({});

  let searchEl = $state<HTMLInputElement>();
  let listEl = $state<HTMLElement>();
  let detail = $state<ReturnType<typeof DetailPane>>();

  const mediaTypes = $derived([...new Set([...MEDIA_TYPES, ...books.map((b) => b.media_type)])]);
  const side = $derived(sideItems(books, STATUSES, MEDIA_TYPES));
  const filtered = $derived(sortBooks(books.filter((b) => matchesFilter(b, filter) && matchesQuery(b, query)), sort));
  const cursor = $derived(Math.max(0, filtered.findIndex((b) => b.id === selectedId)));
  const selected = $derived<Book | null>(filtered[cursor] ?? null);
  const readCount = $derived(books.filter((b) => b.status === "読了").length);
  const avgRating = $derived.by(() => {
    const r = books.filter((b) => b.rating > 0);
    return r.length ? (r.reduce((s, b) => s + b.rating, 0) / r.length).toFixed(1) : "-";
  });
  const coverSrc = $derived.by(() => {
    if (!selected) return null;
    if (selected.cover_local_path && covers[selected.cover_local_path]) return covers[selected.cover_local_path];
    return selected.cover_url;
  });

  function flash(msg: string) {
    message = msg;
    const m = msg;
    setTimeout(() => message === m && (message = ""), 6000);
  }

  async function reload() {
    try {
      [books, info] = await Promise.all([api.listBooks(), api.libraryInfo()]);
    } catch (e) {
      flash(`読み込みに失敗しました: ${e}`);
    }
  }

  onMount(async () => {
    await reload();
    selectedId = filtered[0]?.id ?? null;
  });

  // Load the cached cover of the selected book (data: URL from the back end).
  $effect(() => {
    const rel = selected?.cover_local_path;
    if (rel && !(rel in covers)) {
      api.coverData(rel).then((d) => d && (covers[rel] = d)).catch(() => {});
    }
  });

  function replaceBook(b: Book) {
    const i = books.findIndex((x) => x.id === b.id);
    if (i >= 0) books[i] = b;
    else books.push(b);
  }

  async function saveBook(b: Book) {
    try {
      const saved = await api.saveBook(b);
      replaceBook(saved);
      if (saved.cover_url && !saved.cover_local_path) cacheCover(saved.id!);
      return saved;
    } catch (e) {
      flash(`保存できません: ${e}`);
      return null;
    }
  }

  async function cacheCover(id: number) {
    try {
      replaceBook(await api.fetchCover(id));
    } catch {
      // The remote URL keeps being shown; caching is retried on the next save.
    }
  }

  async function onCoverUpdated(updated: Book) {
    replaceBook(updated);
    if (updated.cover_local_path) {
      try {
        const data = await api.coverData(updated.cover_local_path);
        if (data) covers[updated.cover_local_path] = data;
      } catch {
        // ignore
      }
    }
    flash(`「${updated.title}」の表紙イメージを更新しました。`);
  }

  async function select(i: number) {
    if (filtered.length === 0) return;
    const n = Math.max(0, Math.min(filtered.length - 1, i));
    selectedId = filtered[n].id;
    await tick();
    listEl?.querySelector(".row.cur")?.scrollIntoView({ block: "nearest" });
  }

  function setFilter(f: Filter) {
    filter = f;
    tick().then(() => select(0));
  }

  function setSort(key: SortKey) {
    sort = sort.key === key ? { key, desc: !sort.desc } : { key, desc: key === "completed_at" || key === "rating" };
  }

  function cycleSort() {
    const keys = Object.keys(SORT_LABELS) as SortKey[];
    setSort(keys[(keys.indexOf(sort.key) + 1) % keys.length]);
  }

  async function onLookup(r: LookupResult) {
    const d = dialog;
    dialog = null;
    if (!d) return;
    if (d.mode === "add") {
      let b: Book = { ...emptyBook(), title: r.title, status: r.status, media_type: r.media_type };
      if (r.status === "読了") b.completed_at = today();
      if (r.status === "読書中") b.started_at = today();
      if (r.candidate) b = applyCandidate(b, r.candidate);
      const saved = await saveBook(b);
      if (saved) {
        filter = { kind: "all" };
        query = "";
        await tick();
        selectedId = saved.id;
        await select(cursor);
        flash(`「${saved.title}」を追加しました。評価と感想を入力できます。`);
      }
    } else if (r.candidate) {
      const saved = await saveBook(applyCandidate(d.book, r.candidate));
      if (saved) flash(`「${saved.title}」の書誌情報を更新しました。`);
    }
    focus = "list";
  }

  async function setRating(v: number) {
    if (selected) await saveBook({ ...selected, rating: v });
  }

  async function removeSelected() {
    if (!selected) return;
    const ok = await ask(`「${selected.title}」を削除しますか？`, { title: "bookf", kind: "warning", okLabel: "削除", cancelLabel: "キャンセル" });
    if (!ok) return;
    const i = cursor;
    try {
      await api.deleteBook(selected.id!);
      books = books.filter((b) => b.id !== selected!.id);
      await tick();
      select(i);
    } catch (e) {
      flash(`削除できません: ${e}`);
    }
  }

  async function openLink() {
    if (selected?.link_url) await openUrl(selected.link_url).catch((e) => flash(String(e)));
  }

  async function importCsv() {
    const path = await openDialog({ title: "Notion からエクスポートした CSV", filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (typeof path !== "string") return;
    try {
      const r = await api.importNotionCsv(path);
      await reload();
      flash(`インポート: ${r.imported} 件を追加、${r.skipped} 件をスキップ（登録済み・タイトルなし）`);
    } catch (e) {
      flash(`インポートできません: ${e}`);
    }
  }

  async function moveDb() {
    const path = await saveDialog({
      title: "データベースの保存先（例: Google ドライブのフォルダー）",
      defaultPath: "books.db",
      filters: [{ name: "bookf データベース", extensions: ["db"] }],
    });
    if (!path) return;
    try {
      info = await api.setDbPath(path, true);
      covers = {};
      await reload();
      flash(`保存先を ${info.db_path} に変更しました。`);
    } catch (e) {
      flash(`保存先を変更できません: ${e}`);
    }
  }

  async function openDb() {
    const path = await openDialog({ title: "データベースを開く", filters: [{ name: "bookf データベース", extensions: ["db"] }] });
    if (typeof path !== "string") return;
    try {
      info = await api.setDbPath(path, false);
      covers = {};
      await reload();
      select(0);
      flash(`${info.db_path} を開きました。`);
    } catch (e) {
      flash(`開けません: ${e}`);
    }
  }

  function selectableSide() {
    return side.map((s, i) => ({ s, i })).filter(({ s }) => !s.heading);
  }

  function moveSide(d: number) {
    const items = selectableSide();
    const at = Math.max(0, items.findIndex(({ s }) => sameFilter(s.filter, filter)));
    const next = items[Math.max(0, Math.min(items.length - 1, at + d))];
    setFilter(next.s.filter);
  }

  function isTyping(t: EventTarget | null) {
    const el = t as HTMLElement | null;
    return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable);
  }

  function onkeydown(e: KeyboardEvent) {
    if (dialog || e.isComposing) return;
    const ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && e.key.toLowerCase() === "n") return e.preventDefault(), (dialog = { mode: "add" });
    if (ctrl && e.key.toLowerCase() === "i") return e.preventDefault(), importCsv();
    if (ctrl && e.key.toLowerCase() === "o") return e.preventDefault(), openDb();
    if (ctrl && e.key.toLowerCase() === "f") return e.preventDefault(), searchEl?.focus();

    if (isTyping(e.target)) {
      if (e.key === "Escape") {
        (e.target as HTMLElement).blur();
        focus = "list";
      } else if (e.target === searchEl && (e.key === "Enter" || e.key === "ArrowDown")) {
        e.preventDefault();
        searchEl?.blur();
        focus = "list";
        select(0);
      }
      return;
    }
    if (ctrl) return;

    if (e.key === "Tab") {
      e.preventDefault();
      focus = focus === "side" ? "list" : "side";
      return;
    }
    if (e.key === "/") return e.preventDefault(), searchEl?.focus();

    if (focus === "side") {
      if (e.key === "j" || e.key === "ArrowDown") return e.preventDefault(), moveSide(1);
      if (e.key === "k" || e.key === "ArrowUp") return e.preventDefault(), moveSide(-1);
      if (e.key === "Enter" || e.key === "l" || e.key === "ArrowRight") return e.preventDefault(), (focus = "list");
      return;
    }

    focus = "list";
    const page = Math.max(1, Math.floor((listEl?.clientHeight ?? 400) / 20) - 1);
    switch (e.key) {
      case "j":
      case "ArrowDown":
        e.preventDefault();
        select(cursor + 1);
        break;
      case "k":
      case "ArrowUp":
        e.preventDefault();
        select(cursor - 1);
        break;
      case "PageDown":
        e.preventDefault();
        select(cursor + page);
        break;
      case "PageUp":
        e.preventDefault();
        select(cursor - page);
        break;
      case "Home":
      case "g":
        e.preventDefault();
        select(0);
        break;
      case "End":
      case "G":
        e.preventDefault();
        select(filtered.length - 1);
        break;
      case "h":
      case "ArrowLeft":
        e.preventDefault();
        focus = "side";
        break;
      case "0":
      case "1":
      case "2":
      case "3":
      case "4":
      case "5":
        setRating(Number(e.key));
        break;
      case "e":
        e.preventDefault();
        detail?.focusNotes();
        break;
      case "o":
      case "Enter":
        openLink();
        break;
      case "u":
        if (selected) dialog = { mode: "update", book: selected };
        break;
      case "s":
        cycleSort();
        break;
      case "r":
        sort = { ...sort, desc: !sort.desc };
        break;
      case "Delete":
        removeSelected();
        break;
    }
  }

  function suggestQuery(b: Book): string {
    if (b.isbn) return b.isbn;
    if (b.title.startsWith("http://") || b.title.startsWith("https://")) return b.title;
    return b.title.trim();
  }

  function stars(n: number) {
    return "★".repeat(n) + "☆".repeat(5 - n);
  }

  function shortDate(d: string | null) {
    return d ? d.replaceAll("-", "/") : "";
  }

  const sortMark = (k: SortKey) => (sort.key === k ? (sort.desc ? " ▾" : " ▴") : "");

  const menus = $derived<MenuGroup[]>([
    {
      id: "file",
      name: "ファイル",
      accessKey: "F",
      items: [
        {
          id: "add_book",
          label: "本を追加",
          shortcut: "Ctrl+N",
          action: () => (dialog = { mode: "add" }),
        },
        {
          id: "import_csv",
          label: "Notion CSV 取り込み…",
          shortcut: "Ctrl+I",
          action: importCsv,
        },
        { id: "sep_file_1", label: "", type: "separator" },
        {
          id: "open_db",
          label: "データベースを開く…",
          shortcut: "Ctrl+O",
          action: openDb,
        },
        {
          id: "move_db",
          label: "保存先を移動…",
          action: moveDb,
        },
      ],
    },
    {
      id: "edit",
      name: "編集",
      accessKey: "E",
      items: [
        {
          id: "search_metadata",
          label: "書誌情報を検索…",
          shortcut: "u",
          disabled: !selected,
          action: () => selected && (dialog = { mode: "update", book: selected }),
        },
        {
          id: "fetch_from_link",
          label: "リンク先から書誌情報を取得",
          disabled: !selected || !selected.link_url,
          action: async () => {
            if (selected && selected.link_url) {
              await detail?.fetchFromLink();
            }
          },
        },
        {
          id: "change_cover",
          label: "表紙画像を変更…",
          disabled: !selected,
          action: () => selected && (coverDialogBook = selected),
        },
        {
          id: "open_link",
          label: "リンクを開く",
          shortcut: "o / Enter",
          disabled: !selected || !selected.link_url,
          action: openLink,
        },
        { id: "sep_edit_1", label: "", type: "separator" },
        {
          id: "delete_book",
          label: "本を削除",
          shortcut: "Delete",
          disabled: !selected,
          action: removeSelected,
        },
      ],
    },
    {
      id: "view",
      name: "表示",
      accessKey: "V",
      items: [
        {
          id: "sort_completed",
          label: "並び替え: 読了日",
          checked: sort.key === "completed_at",
          action: () => setSort("completed_at"),
        },
        {
          id: "sort_title",
          label: "並び替え: タイトル",
          checked: sort.key === "title",
          action: () => setSort("title"),
        },
        {
          id: "sort_author",
          label: "並び替え: 著者",
          checked: sort.key === "author",
          action: () => setSort("author"),
        },
        {
          id: "sort_rating",
          label: "並び替え: 評価",
          checked: sort.key === "rating",
          action: () => setSort("rating"),
        },
        { id: "sep_view_1", label: "", type: "separator" },
        {
          id: "toggle_sort_order",
          label: sort.desc ? "昇順に切り替え" : "降順に切り替え",
          shortcut: "r",
          action: () => (sort = { ...sort, desc: !sort.desc }),
        },
        { id: "sep_view_2", label: "", type: "separator" },
        {
          id: "clear_filter",
          label: "すべての絞り込みを解除",
          disabled: filter.kind === "all" && !query,
          action: () => {
            filter = { kind: "all" };
            query = "";
          },
        },
      ],
    },
  ]);
</script>

<svelte:window {onkeydown} />

<div class="app">
  <MenuBar {menus} />

  <div class="searchbar">
    <span class="label">検索</span>
    <input bind:this={searchEl} bind:value={query} placeholder="/ でタイトル・著者・感想を絞り込み" spellcheck="false" />
    <span class="muted">並び: {SORT_LABELS[sort.key]} {sort.desc ? "降順" : "昇順"}（s で切替 / r で反転）</span>
  </div>

  <main class="panes">
    <section class="pane side" class:active={focus === "side"}>
      <div class="pane-title"> 絞り込み </div>
      <div class="pane-body">
        {#each side as item}
          {#if item.heading}
            <div class="side-head">{item.label}</div>
          {:else}
            <button class="side-item" class:cur={sameFilter(item.filter, filter)} class:focused={focus === "side"} onclick={() => { setFilter(item.filter); focus = "side"; }}>
              <span class:stars={item.filter.kind === "rating" && item.filter.value > 0}>{item.label}</span>
              <span class="count">{item.count}</span>
            </button>
          {/if}
        {/each}
      </div>
    </section>

    <section class="pane list" class:active={focus === "list"}>
      <div class="pane-title"> 本 ({filtered.length} 冊){query ? ` [検索: ${query}]` : ""} </div>
      <div class="table-head">
        <button onclick={() => setSort("rating")}>評価{sortMark("rating")}</button>
        <button onclick={() => setSort("title")}>タイトル{sortMark("title")}</button>
        <button onclick={() => setSort("author")}>著者{sortMark("author")}</button>
        <button onclick={() => setSort("media_type")}>種別{sortMark("media_type")}</button>
        <button onclick={() => setSort("status")}>状態{sortMark("status")}</button>
        <button onclick={() => setSort("completed_at")}>読了日{sortMark("completed_at")}</button>
      </div>
      <div class="pane-body" bind:this={listEl}>
        {#if filtered.length === 0}
          <div class="empty muted">
            {#if books.length === 0}
              本がまだありません。Ctrl+N で追加するか、Ctrl+I で Notion の CSV を取り込んでください。
            {:else}
              該当する本がありません。
            {/if}
          </div>
        {/if}
        {#each filtered as b, i (b.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="row"
            class:cur={i === cursor}
            class:focused={focus === "list"}
            role="row"
            tabindex="-1"
            onclick={() => { selectedId = b.id; focus = "list"; }}
            ondblclick={openLink}
          >
            <span class="stars"><span>{stars(b.rating).slice(0, b.rating)}</span><span class="off">{stars(b.rating).slice(b.rating)}</span></span>
            <span class="title">{b.title}{#if b.notes}<span class="note-mark" title="感想あり"> ✎</span>{/if}</span>
            <span class="author">{b.author ?? ""}</span>
            <span class="media">{b.media_type}</span>
            <span class="status" class:reading={b.status === "読書中"} class:todo={b.status !== "読了" && b.status !== "読書中"}>{b.status}</span>
            <span class="date">{shortDate(b.completed_at)}</span>
          </div>
        {/each}
      </div>
    </section>

    <section class="pane detail" class:active={focus === "detail"}>
      <div class="pane-title"> 詳細 </div>
      <div class="pane-body">
        {#if selected}
          <DetailPane
            bind:this={detail}
            book={selected}
            {coverSrc}
            {mediaTypes}
            onsave={saveBook}
            onlookup={() => selected && (dialog = { mode: "update", book: selected })}
            onopen={(u) => openUrl(u)}
            onfocus={() => (focus = "detail")}
            onchangecover={() => (coverDialogBook = selected)}
          />
        {:else}
          <div class="empty muted">本を選ぶと詳細が表示されます。</div>
        {/if}
      </div>
    </section>
  </main>

  <footer class="statusbar">
    <span>{filtered.length ? `${cursor + 1}/${filtered.length}` : "0/0"}</span>
    <span>全 {books.length} 冊 / 読了 {readCount} 冊</span>
    <span>平均 ★{avgRating}</span>
    <span class="msg">{message}</span>
    <span class="path" title={info?.db_path}>{info?.db_path ?? ""}</span>
  </footer>
</div>

{#if dialog}
  <LookupDialog
    mode={dialog.mode}
    initialQuery={dialog.mode === "update" ? suggestQuery(dialog.book) : ""}
    initialLinkUrl={dialog.mode === "update" ? (dialog.book.link_url ?? "") : ""}
    {mediaTypes}
    onclose={() => (dialog = null)}
    onpick={onLookup}
  />
{/if}

{#if coverDialogBook}
  <CoverDialog
    book={coverDialogBook}
    {coverSrc}
    onclose={() => (coverDialogBook = null)}
    onupdated={onCoverUpdated}
  />
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 2px;
    gap: 3px;
    box-sizing: border-box;
  }
  .statusbar {
    display: flex;
    align-items: center;
    gap: 2ch;
    height: var(--row);
    padding: 1px 1ch;
    background: var(--ui-status-bg);
    color: var(--ui-status-fg);
    white-space: nowrap;
    overflow: hidden;
  }
  .searchbar {
    display: flex;
    align-items: center;
    gap: 1ch;
    padding: 0 1ch;
  }
  .searchbar .label {
    color: var(--ui-border-active);
  }
  .searchbar input {
    flex: 0 1 50ch;
    height: 22px;
  }
  .panes {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(12ch, 13.5%) 1fr minmax(44ch, 30%);
    gap: 3px;
  }

  /* Sidebar */
  .side-head {
    margin-top: 5px;
    padding: 0 0.5ch 0 1ch;
    color: var(--ui-accent);
    font-weight: bold;
    font-size: 11px;
  }
  .side-item {
    display: flex;
    width: 100%;
    justify-content: space-between;
    align-items: center;
    border: none;
    padding: 0 0.5ch 0 1ch;
    height: var(--row);
    text-align: left;
    font-size: 11px;
  }
  .side-item:hover:not(:disabled) {
    border: none;
    color: inherit;
    background: var(--ui-surface-2);
  }
  .side-item.cur {
    background: var(--ui-inactive-cursor-bg);
  }
  .side-item.cur.focused {
    background: var(--ui-cursor-bg);
    color: var(--ui-cursor-fg);
    font-weight: bold;
  }
  .side-item.cur.focused .stars {
    color: inherit;
  }
  .count {
    color: var(--ui-muted);
  }
  .side-item.cur.focused .count {
    color: inherit;
  }

  /* Book table */
  .list {
    --cols: 11ch 1fr 18ch 15ch 7ch 11ch;
  }
  .table-head,
  .row {
    display: grid;
    grid-template-columns: var(--cols);
    gap: 1.5ch;
    padding: 0 1ch;
    align-items: center;
  }
  .table-head {
    margin: 0 1px;
    border-bottom: 1px solid var(--ui-border);
  }
  .table-head button {
    border: none;
    padding: 0;
    text-align: left;
    color: var(--ui-accent);
  }
  .row {
    height: 20px;
    cursor: default;
  }
  .row > span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .row:hover {
    background: var(--ui-surface-2);
  }
  .row.cur {
    background: var(--ui-inactive-cursor-bg);
  }
  .row.cur.focused {
    background: var(--ui-cursor-bg);
    color: var(--ui-cursor-fg);
  }
  .row.cur.focused .title {
    font-weight: bold;
  }
  .row.cur.focused > span,
  .row.cur.focused .stars .off,
  .row.cur.focused .note-mark {
    color: var(--ui-cursor-fg);
  }
  .stars .off {
    color: var(--ui-border);
  }
  .author,
  .date {
    color: var(--ui-muted);
  }
  .media {
    color: var(--ui-accent);
  }
  .status.reading {
    color: var(--ui-green);
  }
  .status.todo {
    color: var(--ui-purple);
  }
  .note-mark {
    color: var(--ui-muted-2);
  }
  .empty {
    padding: 1em 2ch;
  }

  /* Status bar */
  .msg {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ui-star);
  }
  .path {
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    color: var(--ui-muted);
  }
</style>
