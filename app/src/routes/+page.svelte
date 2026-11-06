<script lang="ts">
  import "../app.css";
  import { onMount, tick } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import {
    applyCandidate,
    bookFromLookup,
    clampIndex,
    cycleSort,
    libraryStats,
    matchesFilter,
    matchesQuery,
    nextSort,
    sideItems,
    sortBooks,
    stepFilter,
    suggestQuery,
    SORT_LABELS,
    today,
    type Filter,
    type Sort,
    type SortKey,
  } from "$lib/library";
  import { MEDIA_TYPES, STATUSES, type Book, type LibraryInfo, type LookupResult } from "$lib/types";
  import { isTypingTarget, resolveKey, type Focus, type KeyAction } from "$lib/features/keymap";
  import { buildMenus } from "$lib/features/menu";
  import { pickDatabase, pickDatabaseTarget, pickNotionCsv } from "$lib/features/file-dialogs";
  import BookList from "$lib/components/BookList.svelte";
  import CoverDialog from "$lib/components/CoverDialog.svelte";
  import DetailPane from "$lib/components/DetailPane.svelte";
  import LookupDialog from "$lib/components/LookupDialog.svelte";
  import MenuBar from "$lib/components/MenuBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";

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
  let list = $state<ReturnType<typeof BookList>>();
  let detail = $state<ReturnType<typeof DetailPane>>();

  const mediaTypes = $derived([...new Set([...MEDIA_TYPES, ...books.map((b) => b.media_type)])]);
  const side = $derived(sideItems(books, STATUSES, MEDIA_TYPES));
  const filtered = $derived(sortBooks(books.filter((b) => matchesFilter(b, filter) && matchesQuery(b, query)), sort));
  const cursor = $derived(Math.max(0, filtered.findIndex((b) => b.id === selectedId)));
  const selected = $derived<Book | null>(filtered[cursor] ?? null);
  const stats = $derived(libraryStats(books));
  const coverSrc = $derived.by(() => {
    if (!selected) return null;
    if (selected.cover_local_path && covers[selected.cover_local_path]) return covers[selected.cover_local_path];
    return selected.cover_url;
  });

  function flash(msg: string) {
    message = msg;
    setTimeout(() => message === msg && (message = ""), 6000);
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
    if (rel && !(rel in covers)) loadCover(rel);
  });

  async function loadCover(rel: string) {
    try {
      const d = await api.coverData(rel);
      if (d) covers[rel] = d;
    } catch {
      // The remote cover_url is shown instead.
    }
  }

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
    if (updated.cover_local_path) await loadCover(updated.cover_local_path);
    flash(`「${updated.title}」の表紙イメージを更新しました。`);
  }

  async function select(i: number) {
    if (filtered.length === 0) return;
    selectedId = filtered[clampIndex(i, filtered.length)].id;
    await list?.scrollToCursor();
  }

  function setFilter(f: Filter) {
    filter = f;
    tick().then(() => select(0));
  }

  function clearFilter() {
    filter = { kind: "all" };
    query = "";
  }

  const setSort = (key: SortKey) => (sort = nextSort(sort, key));
  const openLookup = () => selected && (dialog = { mode: "update", book: selected });

  async function onLookup(r: LookupResult) {
    const d = dialog;
    dialog = null;
    if (!d) return;
    if (d.mode === "add") {
      const saved = await saveBook(bookFromLookup(r, today()));
      if (saved) {
        clearFilter();
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
    const target = selected;
    const ok = await ask(`「${target.title}」を削除しますか？`, { title: "bookf", kind: "warning", okLabel: "削除", cancelLabel: "キャンセル" });
    if (!ok) return;
    const i = cursor;
    try {
      await api.deleteBook(target.id!);
      books = books.filter((b) => b.id !== target.id);
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
    const path = await pickNotionCsv();
    if (!path) return;
    try {
      const r = await api.importNotionCsv(path);
      await reload();
      flash(`インポート: ${r.imported} 件を追加、${r.skipped} 件をスキップ（登録済み・タイトルなし）`);
    } catch (e) {
      flash(`インポートできません: ${e}`);
    }
  }

  /** 保存先を移動 (`copy`: the current library is copied there first) or データベースを開く. */
  async function switchDb(copy: boolean) {
    const path = await (copy ? pickDatabaseTarget() : pickDatabase());
    if (!path) return;
    try {
      info = await api.setDbPath(path, copy);
      covers = {};
      await reload();
      if (!copy) select(0);
      flash(copy ? `保存先を ${info.db_path} に変更しました。` : `${info.db_path} を開きました。`);
    } catch (e) {
      flash(copy ? `保存先を変更できません: ${e}` : `開けません: ${e}`);
    }
  }
  const moveDb = () => switchDb(true);
  const openDb = () => switchDb(false);

  function run(a: KeyAction) {
    switch (a.type) {
      case "addBook": return void (dialog = { mode: "add" });
      case "importCsv": return void importCsv();
      case "openDb": return void openDb();
      case "focusSearch": return searchEl?.focus();
      case "leaveInput":
        (document.activeElement as HTMLElement | null)?.blur();
        focus = "list";
        return;
      case "searchToList":
        searchEl?.blur();
        focus = "list";
        return void select(0);
      case "toggleFocus": return void (focus = focus === "side" ? "list" : "side");
      case "sideMove": return setFilter(stepFilter(side, filter, a.delta));
      case "sideToList": return void (focus = "list");
    }
    focus = "list";
    switch (a.type) {
      case "move": return void select(cursor + a.delta);
      case "page": return void select(cursor + a.delta * (list?.pageSize() ?? 20));
      case "first": return void select(0);
      case "last": return void select(filtered.length - 1);
      case "toSide": return void (focus = "side");
      case "rate": return void setRating(a.value);
      case "editNotes": return detail?.focusNotes();
      case "openLink": return void openLink();
      case "lookup": return void openLookup();
      case "cycleSort": return void (sort = cycleSort(sort));
      case "reverseSort": return void (sort = { ...sort, desc: !sort.desc });
      case "deleteBook": return void removeSelected();
    }
  }

  function onkeydown(e: KeyboardEvent) {
    const r = resolveKey({
      key: e.key,
      ctrl: e.ctrlKey || e.metaKey,
      composing: e.isComposing,
      dialogOpen: !!dialog || !!coverDialogBook,
      typing: isTypingTarget(e.target),
      inSearch: e.target === searchEl,
      focus,
    });
    if (!r) return;
    if (r.preventDefault) e.preventDefault();
    run(r.action);
  }

  const menus = $derived(
    buildMenus(
      { hasSelection: !!selected, hasLink: !!selected?.link_url, sort, filtered: filter.kind !== "all" || !!query },
      {
        addBook: () => run({ type: "addBook" }),
        importCsv,
        openDb,
        moveDb,
        lookup: openLookup,
        fetchFromLink: () => selected?.link_url && detail?.fetchFromLink(),
        changeCover: () => selected && (coverDialogBook = selected),
        openLink,
        deleteBook: removeSelected,
        setSort,
        reverseSort: () => run({ type: "reverseSort" }),
        clearFilter,
      },
    ),
  );
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
    <Sidebar items={side} {filter} active={focus === "side"} onpick={(f) => { setFilter(f); focus = "side"; }} />

    <BookList
      bind:this={list}
      books={filtered}
      total={books.length}
      {cursor}
      active={focus === "list"}
      {sort}
      {query}
      onselect={(id) => { selectedId = id; focus = "list"; }}
      onopen={openLink}
      onsort={setSort}
    />

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
            onlookup={openLookup}
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

  <StatusBar position={cursor} shown={filtered.length} total={books.length} {stats} {message} dbPath={info?.db_path ?? ""} />
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
  <CoverDialog book={coverDialogBook} {coverSrc} onclose={() => (coverDialogBook = null)} onupdated={onCoverUpdated} />
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
  .empty {
    padding: 1em 2ch;
  }
</style>
