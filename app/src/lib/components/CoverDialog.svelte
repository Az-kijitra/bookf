<script lang="ts">
  import { untrack } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { api } from "$lib/api";
  import type { Book } from "$lib/types";

  let {
    book,
    coverSrc,
    onclose,
    onupdated,
  }: {
    book: Book;
    coverSrc: string | null;
    onclose: () => void;
    onupdated: (b: Book) => void;
  } = $props();

  let urlInput = $state(untrack(() => book.cover_url ?? ""));
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function chooseLocalFile() {
    if (busy) return;
    try {
      const selected = await openDialog({
        title: "表紙画像ファイルを選択",
        multiple: false,
        directory: false,
        filters: [{ name: "画像ファイル", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
      });
      if (!selected || typeof selected !== "string") return;

      busy = true;
      error = null;
      const updated = await api.setCoverFromFile(book.id!, selected);
      onupdated(updated);
      onclose();
    } catch (e) {
      error = `ファイルを設定できませんでした: ${e}`;
    } finally {
      busy = false;
    }
  }

  async function submitUrl() {
    const u = urlInput.trim();
    if (!u || busy) return;
    busy = true;
    error = null;
    try {
      const updated = await api.setCoverFromUrl(book.id!, u);
      onupdated(updated);
      onclose();
    } catch (e) {
      error = `URLから画像を取得できませんでした: ${e}`;
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (busy) return;
    busy = true;
    error = null;
    try {
      const updated = await api.removeCover(book.id!);
      onupdated(updated);
      onclose();
    } catch (e) {
      error = `表紙を削除できませんでした: ${e}`;
    } finally {
      busy = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onclose();
    } else if (e.key === "Enter" && !e.isComposing && (e.target as HTMLElement).tagName !== "BUTTON") {
      e.preventDefault();
      if (urlInput.trim()) submitUrl();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onkeydown={onkeydown} onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog pane active" role="dialog" aria-modal="true">
    <div class="pane-title"> 表紙イメージの変更 </div>
    <div class="body">
      <div class="preview-row">
        <div class="preview-box">
          {#if coverSrc}
            <img src={coverSrc} alt="表紙プレビュー" referrerpolicy="no-referrer" />
          {:else}
            <div class="nopreview muted">表紙なし</div>
          {/if}
        </div>
        <div class="info">
          <div class="title">{book.title}</div>
          <div class="muted">{book.author ?? "著者不明"}</div>
        </div>
      </div>

      {#if error}
        <div class="error">{error}</div>
      {/if}

      <div class="section">
        <div class="sec-title">1. PC 内の画像ファイルから読み込む</div>
        <button class="action-btn" onclick={chooseLocalFile} disabled={busy}>
          📁 画像ファイルを選択…
        </button>
        <div class="hint muted">（PNG / JPG / WebP / GIF、10 MB まで）</div>
      </div>

      <div class="section">
        <div class="sec-title">2. 任意の画像 URL から読み込む</div>
        <div class="url-row">
          <input
            bind:value={urlInput}
            placeholder="https://example.com/cover.jpg"
            spellcheck="false"
            disabled={busy}
          />
          <button onclick={submitUrl} disabled={busy || !urlInput.trim()}>
            {busy ? "取得中…" : "URLから設定"}
          </button>
        </div>
      </div>

      {#if book.cover_local_path || book.cover_url}
        <div class="section">
          <button class="remove-btn" onclick={remove} disabled={busy}>
            ✕ 現在の表紙を削除する
          </button>
        </div>
      {/if}

      <div class="footer">
        <button onclick={onclose} disabled={busy}>閉じる (Esc)</button>
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 500;
  }
  .dialog {
    width: 440px;
    max-width: 95vw;
    background: var(--ui-bg);
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.6);
  }
  .body {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .preview-row {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .preview-box {
    width: 60px;
    height: 85px;
    border: 1px solid var(--ui-border);
    background: var(--ui-surface);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    overflow: hidden;
  }
  .preview-box img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .nopreview {
    font-size: 10px;
    text-align: center;
  }
  .info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .title {
    font-weight: bold;
    font-size: 13px;
    line-height: 18px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 5px;
    border-top: 1px solid var(--ui-border);
    padding-top: 8px;
  }
  .sec-title {
    font-size: 11px;
    color: var(--ui-accent);
    font-weight: bold;
  }
  .action-btn {
    height: 28px;
    text-align: left;
    padding: 0 1ch;
  }
  .url-row {
    display: flex;
    gap: 6px;
  }
  .url-row input {
    flex: 1;
    height: 26px;
  }
  .url-row button {
    height: 26px;
    padding: 0 1ch;
  }
  .remove-btn {
    height: 26px;
    color: var(--nord11, #bf616a);
    border-color: var(--ui-border);
  }
  .remove-btn:hover:not(:disabled) {
    background: rgba(191, 97, 106, 0.15);
  }
  .error {
    color: var(--nord11, #bf616a);
    font-size: 11px;
    background: rgba(191, 97, 106, 0.1);
    padding: 6px;
    border: 1px solid var(--nord11, #bf616a);
  }
  .hint {
    font-size: 10px;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 4px;
  }
</style>
