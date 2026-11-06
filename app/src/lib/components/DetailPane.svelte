<script lang="ts">
  import { untrack } from "svelte";
  import Stars from "./Stars.svelte";
  import { STATUSES, type Book } from "$lib/types";
  import { today, applyCandidate } from "$lib/library";
  import { api } from "$lib/api";

  let {
    book,
    coverSrc,
    mediaTypes,
    active = false,
    onsave,
    onlookup,
    onopen,
    onfocus,
    onchangecover,
  }: {
    book: Book;
    coverSrc: string | null;
    mediaTypes: string[];
    active?: boolean;
    onsave: (b: Book) => void;
    onlookup: () => void;
    onopen: (url: string) => void;
    onfocus: () => void;
    onchangecover?: () => void;
  } = $props();

  // Local copy edited by the inputs; reset whenever the shown book (or its saved state) changes.
  let draft = $state<Book>(untrack(() => ({ ...book })));
  $effect(() => {
    draft = { ...book };
  });

  let notesEl = $state<HTMLTextAreaElement>();
  let fetchingLink = $state(false);
  let linkError = $state<string | null>(null);

  export function focusNotes() {
    notesEl?.focus();
  }

  export async function fetchFromLink() {
    const url = (draft.link_url || book.link_url || "").trim();
    if (!url || fetchingLink) return;
    fetchingLink = true;
    linkError = null;
    try {
      const candidate = await api.fetchLinkMetadata(url);
      const merged = applyCandidate(draft, candidate);
      draft = merged;
      onsave(merged);
    } catch (e) {
      linkError = String(e);
      setTimeout(() => linkError = null, 5000);
    } finally {
      fetchingLink = false;
    }
  }

  function commit(patch: Partial<Book> = {}) {
    const next = { ...draft, ...patch };
    // Marking a book as read fills today's date if none was set.
    if (patch.status === "読了" && !next.completed_at) next.completed_at = today();
    if (patch.status === "読書中" && !next.started_at) next.started_at = today();
    const changed = (Object.keys(next) as (keyof Book)[]).some((k) => (next[k] ?? null) !== (book[k] ?? null));
    if (changed && next.title.trim()) onsave(next);
  }

  function onNotesKey(e: KeyboardEvent) {
    if (e.key === "s" && e.ctrlKey) {
      e.preventDefault();
      commit();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="detail" onfocusin={onfocus}>
  <div class="top">
    <button
      type="button"
      class="cover-btn"
      onclick={onchangecover}
      title="クリックして表紙イメージを変更（URLまたは画像ファイル）"
    >
      <div class="cover">
        {#if coverSrc}
          <img src={coverSrc} alt="表紙" referrerpolicy="no-referrer" />
        {:else}
          <div class="nocover"><span>{book.title.slice(0, 24)}</span></div>
        {/if}
        <div class="cover-overlay">変更</div>
      </div>
    </button>
    <div class="headline">
      <textarea class="title-input" rows="2" bind:value={draft.title} onchange={() => commit()} spellcheck="false" aria-label="タイトル"></textarea>
      {#if draft.subtitle}<div class="muted sub">{draft.subtitle}</div>{/if}
      <div class="rating-row">
        <Stars value={draft.rating} onchange={(v) => commit({ rating: v })} />
      </div>
      {#if fetchingLink}
        <div class="link-status muted">🔗 リンク先から書誌情報を取得中…</div>
      {/if}
      {#if linkError}
        <div class="link-error">{linkError}</div>
      {/if}
    </div>
  </div>

  <dl class="fields">
    <dt>著者</dt>
    <dd><input bind:value={draft.author} onchange={() => commit()} spellcheck="false" /></dd>
    <dt>状態</dt>
    <dd>
      <select bind:value={draft.status} onchange={() => commit({ status: draft.status })}>
        {#each STATUSES.includes(draft.status) ? STATUSES : [...STATUSES, draft.status] as s}<option>{s}</option>{/each}
      </select>
    </dd>
    <dt>種別</dt>
    <dd>
      <select bind:value={draft.media_type} onchange={() => commit()}>
        {#each mediaTypes.includes(draft.media_type) ? mediaTypes : [...mediaTypes, draft.media_type] as m}<option>{m}</option>{/each}
      </select>
    </dd>
    <dt>開始日</dt>
    <dd><input type="date" bind:value={draft.started_at} onchange={() => commit()} /></dd>
    <dt>読了日</dt>
    <dd><input type="date" bind:value={draft.completed_at} onchange={() => commit()} /></dd>
    <dt>出版社</dt>
    <dd><input bind:value={draft.publisher} onchange={() => commit()} spellcheck="false" /></dd>
    <dt>出版日</dt>
    <dd><input bind:value={draft.published_date} onchange={() => commit()} spellcheck="false" /></dd>
    <dt>ISBN</dt>
    <dd><input bind:value={draft.isbn} onchange={() => commit()} spellcheck="false" /></dd>
    <dt>リンク</dt>
    <dd class="link-dd">
      <input bind:value={draft.link_url} onchange={() => commit()} spellcheck="false" />
      {#if draft.link_url}
        <button type="button" class="open-link-btn" onclick={() => onopen(draft.link_url!)} title="ブラウザで開く (o / Enter)">↗</button>
      {/if}
    </dd>
  </dl>

  <div class="notes-label">感想・メモ <span class="muted">（e で編集、Ctrl+S / フォーカスを外すと保存）</span></div>
  <textarea
    class="notes"
    bind:this={notesEl}
    bind:value={draft.notes}
    onchange={() => commit()}
    onkeydown={onNotesKey}
    placeholder="数行の感想やメモ"
  ></textarea>
  <div class="meta muted">登録 {book.created_at ?? "-"} / 更新 {book.updated_at ?? "-"}</div>
</div>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100%;
    box-sizing: border-box;
    padding: 6px 1.5ch 6px;
  }
  .top {
    display: flex;
    gap: 1.5ch;
  }
  .cover-btn {
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
    display: block;
    flex: none;
  }
  .cover-btn:focus-visible {
    outline: 1px solid var(--ui-border-active);
  }
  .cover {
    position: relative;
    width: 96px;
    height: 136px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--ui-surface-2);
    overflow: hidden;
  }
  .cover-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    color: var(--ui-fg);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }
  .cover-btn:hover .cover-overlay {
    opacity: 1;
  }
  .cover img {
    max-width: 100%;
    max-height: 100%;
  }
  .nocover {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 8px;
    border: 1px solid var(--ui-border);
    color: var(--ui-muted-2);
    font-size: 10px;
    line-height: 13px;
    overflow: hidden;
  }
  .headline {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }
  .title-input {
    width: 100%;
    resize: none;
    font-weight: bold;
    font-size: 13px;
    line-height: 18px;
    border-color: transparent;
  }
  .title-input:hover {
    border-color: var(--ui-border);
  }
  .title-input:focus {
    border-color: var(--ui-border-active);
  }
  .sub {
    padding-left: 5px;
  }
  .link-status {
    font-size: 11px;
    padding-left: 2px;
  }
  .link-error {
    color: var(--nord11, #bf616a);
    font-size: 11px;
    padding-left: 2px;
  }
  .link-dd {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .open-link-btn {
    flex: 0 0 auto;
    height: 22px;
    padding: 0 6px;
    font-size: 11px;
  }
  .fields {
    display: grid;
    grid-template-columns: 7ch 1fr;
    gap: 3px 1ch;
    margin: 0;
    align-items: center;
  }
  .fields dt {
    color: var(--ui-muted);
  }
  .fields dd {
    margin: 0;
    min-width: 0;
  }
  .fields input,
  .fields select {
    width: 100%;
    height: 22px;
  }
  .notes-label {
    margin-top: 2px;
  }
  .notes {
    flex: 1;
    min-height: 90px;
    width: 100%;
    resize: none;
    line-height: 18px;
  }
  .meta {
    font-size: 10px;
  }
</style>
