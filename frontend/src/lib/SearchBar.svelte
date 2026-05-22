<script lang="ts">
  import { tick } from 'svelte';

  export let visible = false;
  export let query = '';
  export let onClose: () => void;

  let inputEl: HTMLInputElement;

  $: if (visible) {
    tick().then(() => inputEl?.focus());
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      query = '';
      onClose();
    }
  }
</script>

{#if visible}
  <div class="search-bar">
    <span class="search-icon">🔍</span>
    <input
      class="search-input"
      type="text"
      bind:value={query}
      bind:this={inputEl}
      on:keydown={handleKeydown}
      placeholder="Filter by name..."
    />
    <button class="search-close" on:click={() => { query = ''; onClose(); }}>×</button>
  </div>
{/if}

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: var(--bg-mantle);
    border-bottom: 1px solid var(--bg-surface0);
  }

  .search-icon {
    font-size: 14px;
  }

  .search-input {
    flex: 1;
    padding: 5px 8px;
    border: 1px solid var(--bg-surface1);
    border-radius: 4px;
    background: var(--bg-base);
    color: var(--text-primary);
    font-size: 13px;
    outline: none;
  }

  .search-input:focus {
    border-color: var(--accent-blue);
  }

  .search-close {
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    padding: 0 4px;
  }

  .search-close:hover {
    color: var(--text-primary);
  }
</style>
