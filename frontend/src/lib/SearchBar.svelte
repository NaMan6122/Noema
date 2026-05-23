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
    if (e.key === 'Escape') { e.preventDefault(); query = ''; onClose(); }
  }
</script>

{#if visible}
  <div class="search-bar">
    <span class="material-symbols-outlined search-icon">filter_list</span>
    <input
      class="search-input"
      type="text"
      bind:value={query}
      bind:this={inputEl}
      on:keydown={handleKeydown}
      placeholder="Filter by name..."
    />
    <button class="search-close" on:click={() => { query = ''; onClose(); }}>
      <span class="material-symbols-outlined" style="font-size: 16px;">close</span>
    </button>
  </div>
{/if}

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    background: var(--bg-container-low);
    border-bottom: 1px solid var(--text-outline);
  }

  .search-icon {
    font-size: 16px;
    color: var(--text-muted);
  }

  .search-input {
    flex: 1;
    padding: 5px 8px;
    border: 1px solid var(--text-outline);
    border-radius: 4px;
    background: var(--bg-container-lowest);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-body);
    outline: none;
  }

  .search-input:focus {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px rgba(208, 188, 255, 0.15);
  }

  .search-close {
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px;
    border-radius: 4px;
    display: flex;
  }

  .search-close:hover {
    color: var(--text-primary);
    background: var(--bg-container-high);
  }
</style>
