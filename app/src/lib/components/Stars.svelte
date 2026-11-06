<script lang="ts">
  // Clickable 5-star rating. Clicking the current value again clears it (0 = 未評価).
  let { value, onchange }: { value: number; onchange?: (v: number) => void } = $props();
  let hover = $state(0);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="rating" aria-label="評価" onmouseleave={() => (hover = 0)}>
  {#each [1, 2, 3, 4, 5] as n}
    <button
      type="button"
      class="star"
      class:on={(hover || value) >= n}
      aria-pressed={value >= n}
      aria-label={`★${n}`}
      tabindex="-1"
      onmouseenter={() => (hover = n)}
      onclick={() => onchange?.(value === n ? 0 : n)}>{(hover || value) >= n ? "★" : "☆"}</button
    >
  {/each}
</span>

<style>
  .rating {
    display: inline-flex;
  }
  .star {
    border: none;
    padding: 0 1px;
    font-size: 16px;
    line-height: 20px;
    color: var(--ui-border);
  }
  .star.on {
    color: var(--ui-star);
  }
  .star:hover:not(:disabled) {
    border: none;
    color: var(--ui-star);
  }
</style>
