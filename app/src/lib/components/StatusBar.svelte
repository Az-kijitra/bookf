<script lang="ts">
  import type { LibraryStats } from "$lib/library";

  let {
    position,
    shown,
    total,
    stats,
    message,
    dbPath,
  }: {
    /** 0-based cursor in the shown list. */
    position: number;
    shown: number;
    total: number;
    stats: LibraryStats;
    message: string;
    dbPath: string;
  } = $props();
</script>

<footer class="statusbar">
  <span>{shown ? `${position + 1}/${shown}` : "0/0"}</span>
  <span>全 {total} 冊 / 読了 {stats.read} 冊</span>
  <span>平均 ★{stats.avgRating}</span>
  <span class="msg">{message}</span>
  <span class="path" title={dbPath}>{dbPath}</span>
</footer>

<style>
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
