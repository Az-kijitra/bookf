<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api } from "$lib/api";
  import { MEDIA_TYPES, STATUSES, type Candidate, type LookupResult } from "$lib/types";

  let {
    mode,
    initialQuery = "",
    initialLinkUrl = "",
    mediaTypes = MEDIA_TYPES,
    onclose,
    onpick,
  }: {
    mode: "add" | "update";
    initialQuery?: string;
    initialLinkUrl?: string;
    mediaTypes?: string[];
    onclose: () => void;
    onpick: (r: LookupResult) => void;
  } = $props();

  let query = $state("");
  let searched = $state<string | null>(null);
  let results = $state<Candidate[]>([]);
  let cursor = $state(0);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let status = $state("読了");
  let mediaType = $state("書籍");
  let input: HTMLInputElement;
  let list = $state<HTMLElement>();

  onMount(() => {
    query = initialQuery;
    input.focus();
    input.select();
    if (initialQuery.trim()) {
      search();
    } else if (initialLinkUrl.trim()) {
      searchLink(initialLinkUrl);
    }
  });

  async function search() {
    const q = query.trim();
    if (!q || busy) return;
    busy = true;
    error = null;
    try {
      if (q.startsWith("http://") || q.startsWith("https://")) {
        const candidate = await api.fetchLinkMetadata(q);
        results = [candidate];
        cursor = 0;
        searched = q;
      } else {
        results = await api.searchBooks(q);
        cursor = 0;
        searched = q;
        if (results.length === 0) {
          if (initialLinkUrl.trim()) {
            // Try fetching from registered link if search yielded 0 results
            try {
              const fromLink = await api.fetchLinkMetadata(initialLinkUrl.trim());
              results = [fromLink];
              cursor = 0;
              return;
            } catch {
              // fallback to message below
            }
          }
          error = "見つかりませんでした。言葉を変えるか、ISBN や URL で検索してください。";
        }
      }
    } catch (e) {
      error = String(e);
      results = [];
    } finally {
      busy = false;
    }
  }

  async function searchLink(urlToFetch: string) {
    const u = urlToFetch.trim();
    if (!u || busy) return;
    busy = true;
    error = null;
    try {
      const candidate = await api.fetchLinkMetadata(u);
      results = [candidate];
      cursor = 0;
      searched = u;
    } catch (e) {
      error = `リンク先から取得できませんでした: ${e}`;
      results = [];
    } finally {
      busy = false;
    }
  }

  function pick(c: Candidate | null) {
    onpick({ candidate: c, title: c?.title || query.trim(), status, media_type: mediaType });
  }

  async function move(d: number) {
    if (results.length === 0) return;
    cursor = Math.max(0, Math.min(results.length - 1, cursor + d));
    await tick();
    list?.querySelector(".cand.cur")?.scrollIntoView({ block: "nearest" });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onclose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      move(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      move(-1);
    } else if (e.key === "Enter" && !e.isComposing && (e.target as HTMLElement).tagName !== "BUTTON") {
      e.preventDefault();
      // Enter searches when the query changed; otherwise it takes the highlighted result.
      if (query.trim() !== searched || results.length === 0) search();
      else pick(results[cursor]);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onkeydown={onkeydown} onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog pane active" role="dialog" aria-modal="true">
    <div class="pane-title">{mode === "add" ? " 本を追加 " : " 書誌情報を検索して更新 "}</div>
    <div class="body">
      <div class="search-row">
        <span>検索:</span>
        <input bind:this={input} bind:value={query} placeholder="タイトル・著者名・ISBN・URL" spellcheck="false" />
        <button onclick={search} disabled={busy || !query.trim()}>{busy ? "検索中…" : "検索"}</button>
        {#if initialLinkUrl.trim()}
          <button onclick={() => searchLink(initialLinkUrl)} disabled={busy} title={initialLinkUrl}>🔗 リンク先から取得</button>
        {/if}
      </div>
      <div class="hint muted">Enter: 検索 / 選択　↑↓: 候補の移動　Esc: 閉じる　（Webリンク先 / CiNii Books / openBD）</div>

      <div class="results" bind:this={list}>
        {#if error}
          <div class="error">{error}</div>
        {/if}
        {#each results as c, i}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="cand"
            class:cur={i === cursor}
            role="option"
            tabindex="-1"
            aria-selected={i === cursor}
            onclick={() => (cursor = i)}
            ondblclick={() => pick(c)}
          >
            <div class="thumb">
              {#if c.cover_url}<img src={c.cover_url} alt="" loading="lazy" referrerpolicy="no-referrer" />{:else}<span>no image</span>{/if}
            </div>
            <div class="info">
              <div class="t">{c.title || "（タイトル不明）"}{#if c.subtitle}<span class="muted">　{c.subtitle}</span>{/if}</div>
              <div class="muted">{[c.author, c.publisher, c.published_date].filter(Boolean).join(" / ")}</div>
              <div class="muted small">{c.isbn ? `ISBN ${c.isbn}` : "ISBN なし"}</div>
            </div>
          </div>
        {/each}
      </div>

      <div class="footer">
        {#if mode === "add"}
          <label>状態 <select bind:value={status}>{#each STATUSES as s}<option>{s}</option>{/each}</select></label>
          <label>種別 <select bind:value={mediaType}>{#each mediaTypes as m}<option>{m}</option>{/each}</select></label>
          <span class="spacer"></span>
          <button onclick={() => pick(null)} disabled={!query.trim()} title="検索結果を使わず、入力したタイトルだけで登録">手動で追加</button>
          <button class="primary" onclick={() => pick(results[cursor])} disabled={results.length === 0}>追加</button>
        {:else}
          <span class="spacer"></span>
          <button class="primary" onclick={() => pick(results[cursor])} disabled={results.length === 0}>この内容で更新</button>
        {/if}
        <button onclick={onclose}>キャンセル</button>
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--ui-backdrop);
  }
  .dialog {
    width: min(760px, calc(100vw - 40px));
    height: min(620px, calc(100vh - 40px));
    box-shadow: 0 8px 20px var(--ui-shadow);
  }
  .body {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    gap: 6px;
    padding: 6px 2ch 8px;
  }
  .search-row {
    display: flex;
    gap: 1ch;
    align-items: center;
  }
  .search-row input {
    flex: 1;
    height: 24px;
  }
  .search-row button {
    height: 24px;
  }
  .hint,
  .small {
    font-size: 11px;
  }
  .results {
    flex: 1;
    min-height: 0;
    overflow: auto;
    border: 1px solid var(--ui-border);
  }
  .error {
    padding: 8px 1ch;
    color: var(--ui-error);
  }
  .cand {
    display: flex;
    gap: 1.5ch;
    padding: 4px 1ch;
    cursor: pointer;
    border-bottom: 1px solid var(--ui-surface-2);
  }
  .cand.cur {
    background: var(--ui-cursor-bg);
    color: var(--ui-cursor-fg);
  }
  .cand.cur .muted {
    color: var(--ui-cursor-fg);
  }
  .thumb {
    flex: none;
    width: 42px;
    height: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--ui-surface-2);
    font-size: 9px;
    color: var(--ui-muted-2);
    overflow: hidden;
  }
  .thumb img {
    max-width: 100%;
    max-height: 100%;
  }
  .info {
    min-width: 0;
  }
  .t {
    font-weight: bold;
  }
  .footer {
    display: flex;
    gap: 1ch;
    align-items: center;
  }
  .footer button,
  .footer select {
    height: 24px;
  }
  .spacer {
    flex: 1;
  }
</style>
