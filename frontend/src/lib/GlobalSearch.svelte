<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { tick } from 'svelte';

  interface SearchResult {
    path: string;
    filename: string;
    is_dir: boolean;
    extension: string | null;
  }

  export let visible = false;
  export let currentPath = '';
  export let onNavigate: (path: string) => void;
  export let onClose: () => void;

  let query = '';
  let results: SearchResult[] = [];
  let selectedIndex = 0;
  let loading = false;
  let inputEl: HTMLInputElement;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $: if (visible) {
    query = '';
    results = [];
    selectedIndex = 0;
    tick().then(() => inputEl?.focus());
  }

  function handleInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    if (!query.trim()) { results = []; return; }
    debounceTimer = setTimeout(doSearch, 200);
  }

  async function doSearch() {
    if (!query.trim()) return;
    loading = true;
    try {
      results = await invoke<SearchResult[]>('search_files', {
        root: currentPath,
        query: query.trim(),
        limit: 50,
      });
      selectedIndex = 0;
    } catch (_) {
      results = [];
    }
    loading = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(results.length - 1, selectedIndex + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(0, selectedIndex - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      selectResult(selectedIndex);
    }
  }

  function selectResult(idx: number) {
    const r = results[idx];
    if (!r) return;
    if (r.is_dir) {
      onNavigate(r.path);
    } else {
      const parent = r.path.split('/').slice(0, -1).join('/') || '/';
      onNavigate(parent);
    }
    onClose();
  }

  function getIcon(r: SearchResult): string {
    if (r.is_dir) return '📁';
    const ext = r.extension?.toLowerCase();
    if (!ext) return '📄';
    if (['jpg', 'jpeg', 'png', 'gif', 'webp'].includes(ext)) return '🖼️';
    if (['rs', 'py', 'js', 'ts'].includes(ext)) return '⚙️';
    if (['pdf'].includes(ext)) return '📕';
    return '📄';
  }
</script>

{#if visible}
  <div class="global-search-overlay" on:click|self={onClose}>
    <div class="global-search" on:keydown={handleKeydown}>
      <input
        class="global-search-input"
        type="text"
        bind:value={query}
        bind:this={inputEl}
        on:input={handleInput}
        placeholder="Search files..."
      />
      <div class="global-search-results">
        {#if loading}
          <div class="search-status">Searching...</div>
        {:else if query && results.length === 0}
          <div class="search-status">No results</div>
        {:else}
          {#each results as result, i}
            <button
              class="search-result"
              class:selected={i === selectedIndex}
              on:click={() => selectResult(i)}
              on:mouseenter={() => selectedIndex = i}
            >
              <span class="result-icon">{getIcon(result)}</span>
              <div class="result-info">
                <span class="result-name">{result.filename}</span>
                <span class="result-path">{result.path}</span>
              </div>
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .global-search-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    display: flex;
    justify-content: center;
    padding-top: 15vh;
    z-index: 2000;
  }

  .global-search {
    width: 550px;
    max-height: 450px;
    background: var(--bg-base);
    border: 1px solid var(--bg-surface1);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 8px 32px var(--shadow);
    display: flex;
    flex-direction: column;
    align-self: flex-start;
  }

  .global-search-input {
    padding: 14px 18px;
    border: none;
    border-bottom: 1px solid var(--bg-surface0);
    background: transparent;
    color: var(--text-primary);
    font-size: 15px;
    outline: none;
  }

  .global-search-input::placeholder {
    color: var(--text-muted);
  }

  .global-search-results {
    overflow-y: auto;
    max-height: 370px;
    padding: 4px 0;
  }

  .search-status {
    padding: 16px 18px;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }

  .search-result {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 18px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .search-result:hover,
  .search-result.selected {
    background: var(--bg-surface0);
  }

  .result-icon {
    font-size: 16px;
    flex-shrink: 0;
  }

  .result-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .result-name {
    font-weight: 500;
  }

  .result-path {
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
