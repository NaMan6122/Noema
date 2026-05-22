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

  $: if (visible) {
    query = '';
    selectedIndex = 0;
    tick().then(() => inputEl?.focus());
  }

  $: selectedIndex = Math.min(selectedIndex, Math.max(0, filtered.length - 1));

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(filtered.length - 1, selectedIndex + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(0, selectedIndex - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filtered[selectedIndex]) {
        filtered[selectedIndex].action();
        onClose();
      }
    }
  }

  function execute(cmd: Command) {
    cmd.action();
    onClose();
  }
</script>

{#if visible}
  <div class="palette-overlay" on:click|self={onClose}>
    <div class="palette" on:keydown={handleKeydown}>
      <input
        class="palette-input"
        type="text"
        bind:value={query}
        bind:this={inputEl}
        placeholder="Type a command..."
      />
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
  .palette-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    display: flex;
    justify-content: center;
    padding-top: 20vh;
    z-index: 2000;
  }

  .palette {
    width: 500px;
    max-height: 400px;
    background: var(--bg-base);
    border: 1px solid var(--bg-surface1);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 8px 32px var(--shadow);
    display: flex;
    flex-direction: column;
    align-self: flex-start;
  }

  .palette-input {
    padding: 14px 18px;
    border: none;
    border-bottom: 1px solid var(--bg-surface0);
    background: transparent;
    color: var(--text-primary);
    font-size: 15px;
    outline: none;
  }

  .palette-input::placeholder {
    color: var(--text-muted);
  }

  .palette-list {
    overflow-y: auto;
    max-height: 320px;
    padding: 4px 0;
  }

  .palette-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 8px 18px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .palette-item:hover,
  .palette-item.selected {
    background: var(--bg-surface0);
  }

  .shortcut {
    color: var(--text-muted);
    font-size: 11px;
    font-family: 'SF Mono', Monaco, monospace;
  }

  .no-results {
    padding: 16px 18px;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }
</style>
