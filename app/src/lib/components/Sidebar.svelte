<script lang="ts">
  import { sameFilter, type Filter, type SideItem } from "$lib/library";

  let {
    items,
    filter,
    active,
    onpick,
  }: {
    items: SideItem[];
    filter: Filter;
    active: boolean;
    onpick: (f: Filter) => void;
  } = $props();
</script>

<section class="pane side" class:active>
  <div class="pane-title"> 絞り込み </div>
  <div class="pane-body">
    {#each items as item}
      {#if item.heading}
        <div class="side-head">{item.label}</div>
      {:else}
        <button class="side-item" class:cur={sameFilter(item.filter, filter)} class:focused={active} onclick={() => onpick(item.filter)}>
          <span class:stars={item.filter.kind === "rating" && item.filter.value > 0}>{item.label}</span>
          <span class="count">{item.count}</span>
        </button>
      {/if}
    {/each}
  </div>
</section>

<style>
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
</style>
