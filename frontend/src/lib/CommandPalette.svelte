<script lang="ts">
  import { tick } from 'svelte';

  interface Command {
    id: string;
    label: string;
    shortcut?: string;
    action: () => void;
  }

  export let visible = false;
  export let commands: Command[] = [];
  export let onClose: () => void;

  let query = '';
  let selectedIndex = 0;
  let inputEl: HTMLInputElement;

  $: filtered = query
    ? commands.filter(c => c.label.toLowerCase().includes(query.toLowerCase()))
    : commands;

  $: if (visible) { query = ''; selectedIndex = 0; tick().then(() => inputEl?.focus()); }
  $: selectedIndex = Math.min(selectedIndex, Math.max(0, filtered.length - 1));

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.preventDefault(); onClose(); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); selectedIndex = Math.min(filtered.length - 1, selectedIndex + 1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); selectedIndex = Math.max(0, selectedIndex - 1); }
    else if (e.key === 'Enter') { e.preventDefault(); if (filtered[selectedIndex]) { filtered[selectedIndex].action(); onClose(); } }
  }

  function execute(cmd: Command) { cmd.action(); onClose(); }
</script>

{#if visible}
  <div class="overlay" on:click|self={onClose}>
    <div class="palette" on:keydown={handleKeydown}>
      <div class="palette-header">
        <span class="material-symbols-outlined" style="color: var(--accent-primary);">terminal</span>
        <input
          class="palette-input"
          type="text"
          bind:value={query}
          bind:this={inputEl}
          placeholder="Type a command..."
        />
      </div>
      <div class="palette-list">
        {#each filtered as cmd, i (cmd.id)}
          <button
            class="palette-item"
            class:selected={i === selectedIndex}
            on:click={() => execute(cmd)}
            on:mouseenter={() => selectedIndex = i}
          >
            <span>{cmd.label}</span>
            {#if cmd.shortcut}
              <span class="shortcut">{cmd.shortcut}</span>
            {/if}
          </button>
        {/each}
        {#if filtered.length === 0}
          <div class="no-results">No matching commands</div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    padding-top: 20vh;
    z-index: 2000;
  }

  .palette {
    width: 560px;
    max-height: 420px;
    background: var(--glass-bg);
    backdrop-filter: blur(20px);
    border: 1px solid var(--text-outline);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 8px 32px var(--shadow), inset 0 0 15px rgba(208, 188, 255, 0.05);
    display: flex;
    flex-direction: column;
    align-self: flex-start;
  }

  .palette-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--text-outline);
  }

  .palette-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 15px;
    outline: none;
    font-family: var(--font-body);
  }

  .palette-input::placeholder {
    color: var(--text-muted);
  }

  .palette-list {
    overflow-y: auto;
    max-height: 340px;
    padding: 4px;
  }

  .palette-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 10px 14px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--text-primary);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: background 0.1s;
  }

  .palette-item:hover,
  .palette-item.selected {
    background: var(--bg-container-high);
  }

  .shortcut {
    color: var(--text-muted);
    font-size: 11px;
    font-family: var(--font-mono);
    padding: 2px 6px;
    background: var(--bg-container-highest);
    border-radius: 4px;
    border: 1px solid var(--text-outline);
  }

  .no-results {
    padding: 24px 18px;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }
</style>
