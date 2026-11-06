<script lang="ts">
  import { tick } from "svelte";
  import { shortDate, type Sort, type SortKey } from "$lib/library";
  import type { Book } from "$lib/types";

  let {
    books,
    total,
    cursor,
    active,
    sort,
    query,
    onselect,
    onopen,
    onsort,
  }: {
    /** The filtered and sorted books shown. */
    books: Book[];
    /** Number of books in the whole library (for the empty message). */
    total: number;
    cursor: number;
    active: boolean;
    sort: Sort;
    query: string;
    onselect: (id: number | null) => void;
    onopen: () => void;
    onsort: (key: SortKey) => void;
  } = $props();

  const ROW_HEIGHT = 20;
  const COLUMNS: [SortKey, string][] = [
    ["rating", "評価"],
    ["title", "タイトル"],
    ["author", "著者"],
    ["media_type", "種別"],
    ["status", "状態"],
    ["completed_at", "読了日"],
  ];

  let body = $state<HTMLElement>();

  /** Rows that fit in the visible list, minus one for context (PageUp / PageDown). */
  export function pageSize(): number {
    return Math.max(1, Math.floor((body?.clientHeight ?? 400) / ROW_HEIGHT) - 1);
  }

  export async function scrollToCursor() {
    await tick();
    body?.querySelector(".row.cur")?.scrollIntoView({ block: "nearest" });
  }

  const mark = (k: SortKey) => (sort.key === k ? (sort.desc ? " ▾" : " ▴") : "");
</script>

<section class="pane list" class:active>
  <div class="pane-title"> 本 ({books.length} 冊){query ? ` [検索: ${query}]` : ""} </div>
  <div class="table-head">
    {#each COLUMNS as [key, label]}
      <button onclick={() => onsort(key)}>{label}{mark(key)}</button>
    {/each}
  </div>
  <div class="pane-body" bind:this={body}>
    {#if books.length === 0}
      <div class="empty muted">
        {#if total === 0}
          本がまだありません。Ctrl+N で追加するか、Ctrl+I で Notion の CSV を取り込んでください。
        {:else}
          該当する本がありません。
        {/if}
      </div>
    {/if}
    {#each books as b, i (b.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="row" class:cur={i === cursor} class:focused={active} role="row" tabindex="-1" onclick={() => onselect(b.id)} ondblclick={onopen}>
        <span class="stars"><span>{"★".repeat(b.rating)}</span><span class="off">{"☆".repeat(5 - b.rating)}</span></span>
        <span class="title">{b.title}{#if b.notes}<span class="note-mark" title="感想あり"> ✎</span>{/if}</span>
        <span class="author">{b.author ?? ""}</span>
        <span class="media">{b.media_type}</span>
        <span class="status" class:reading={b.status === "読書中"} class:todo={b.status !== "読了" && b.status !== "読書中"}>{b.status}</span>
        <span class="date">{shortDate(b.completed_at)}</span>
      </div>
    {/each}
  </div>
</section>

<style>
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
</style>
