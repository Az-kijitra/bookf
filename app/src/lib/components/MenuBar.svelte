<script lang="ts">
  import { onMount } from "svelte";
  import type { MenuGroup, MenuItem } from "$lib/features/menu";

  let {
    menus,
  }: {
    menus: MenuGroup[];
  } = $props();

  let openMenu = $state<string | null>(null);
  let headingEls = $state<(HTMLButtonElement | undefined)[]>([]);

  function toggleMenu(id: string) {
    openMenu = openMenu === id ? null : id;
  }

  function onHeadingMouseEnter(id: string) {
    if (openMenu !== null) {
      openMenu = id;
    }
  }

  function handleItemClick(item: MenuItem) {
    if (item.disabled || item.type === "separator") return;
    openMenu = null;
    item.action?.();
  }

  function closeMenu() {
    openMenu = null;
  }

  function onWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest("[data-menubar]")) {
      closeMenu();
    }
  }

  function onWindowKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape" && openMenu !== null) {
      e.preventDefault();
      closeMenu();
    } else if (e.altKey && !e.ctrlKey && !e.metaKey) {
      const key = e.key.toUpperCase();
      const match = menus.find((m) => m.accessKey.toUpperCase() === key);
      if (match) {
        e.preventDefault();
        openMenu = openMenu === match.id ? null : match.id;
      }
    }
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKeyDown} />

<div class="menubar" role="menubar" aria-label="メニューバー" data-menubar>
  <span class="brand">bookf</span>
  <div class="menus">
    {#each menus as menu, idx (menu.id)}
      <div class="menu-slot">
        <button
          type="button"
          class="heading"
          class:open={openMenu === menu.id}
          bind:this={headingEls[idx]}
          onclick={() => toggleMenu(menu.id)}
          onmouseenter={() => onHeadingMouseEnter(menu.id)}
          aria-haspopup="menu"
          aria-expanded={openMenu === menu.id}
        >
          {menu.name}(<span class="accel">{menu.accessKey}</span>)
        </button>

        {#if openMenu === menu.id}
          <div class="dropdown" role="menu">
            {#each menu.items as item (item.id)}
              {#if item.type === "separator"}
                <div class="sep" role="separator"></div>
              {:else}
                <button
                  type="button"
                  class="item"
                  class:checked={item.checked}
                  disabled={item.disabled}
                  onclick={() => handleItemClick(item)}
                >
                  <span class="check">{item.checked ? "✓" : ""}</span>
                  <span class="label">{item.label}</span>
                  {#if item.shortcut}
                    <span class="shortcut">{item.shortcut}</span>
                  {/if}
                </button>
              {/if}
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .menubar {
    display: flex;
    align-items: center;
    height: var(--row);
    padding: 0 1ch;
    background: var(--ui-status-bg);
    color: var(--ui-status-fg);
    user-select: none;
    position: relative;
    z-index: 100;
  }
  .brand {
    font-weight: bold;
    color: var(--ui-border-active);
    margin-right: 2ch;
  }
  .menus {
    display: flex;
    align-items: stretch;
    height: 100%;
  }
  .menu-slot {
    position: relative;
    display: flex;
    align-items: stretch;
  }
  .heading {
    display: flex;
    align-items: center;
    border: none;
    background: transparent;
    color: inherit;
    padding: 0 1ch;
    height: 100%;
    cursor: default;
    font-family: inherit;
    font-size: var(--ui-font-size, 12px);
    line-height: inherit;
  }
  .heading:hover,
  .heading.open {
    background: var(--ui-menu-active-bg, #434c5e);
    color: var(--ui-menu-active-fg, #eceff4);
  }
  .accel {
    text-decoration: underline;
  }
  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    min-width: 220px;
    background: var(--ui-surface, #2e3440);
    border: 1px solid var(--ui-border-active, #88c0d0);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.45);
    padding: 3px 0;
    display: flex;
    flex-direction: column;
    z-index: 1000;
  }
  .sep {
    height: 1px;
    background: var(--ui-border, #434c5e);
    margin: 3px 0;
  }
  .item {
    display: flex;
    align-items: center;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--ui-fg, #eceff4);
    padding: 4px 1ch;
    text-align: left;
    font-family: inherit;
    font-size: var(--ui-font-size, 12px);
    cursor: default;
    box-sizing: border-box;
  }
  .item:hover:not(:disabled) {
    background: var(--ui-menu-active-bg, #434c5e);
    color: var(--ui-menu-active-fg, #eceff4);
  }
  .item:disabled {
    color: var(--ui-muted, #616e88);
    cursor: not-allowed;
  }
  .check {
    width: 2ch;
    text-align: center;
    color: var(--ui-accent, #88c0d0);
    margin-right: 0.5ch;
  }
  .label {
    flex: 1;
    white-space: nowrap;
  }
  .shortcut {
    margin-left: 3ch;
    color: var(--ui-muted, #7b88a1);
    font-size: 11px;
    white-space: nowrap;
  }
</style>
