<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { tick } from 'svelte';

  interface FileSearchResult {
    path: string;
    filename: string;
    is_dir: boolean;
    extension: string | null;
    kind: 'file';
  }

  interface ContentSearchResult {
    file_path: string;
    filename: string;
    score: number;
    snippet: { text: string; highlights: [number, number][] } | null;
    match_type: string;
    kind: 'content';
  }

  type SearchResult = FileSearchResult | ContentSearchResult;

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
  let searchMode: 'files' | 'content' = 'files';

  $: if (visible) {
    query = ''; results = []; selectedIndex = 0; searchMode = 'files';
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
      if (searchMode === 'content') {
        const resp = await invoke<{ results: Omit<ContentSearchResult, 'kind'>[] }>('content_search', { query: query.trim(), limit: 30 });
        results = resp.results.map(r => ({ ...r, kind: 'content' as const }));
      } else {
        const fileResults = await invoke<Omit<FileSearchResult, 'kind'>[]>('search_files', { root: currentPath, query: query.trim(), limit: 50 });
        results = fileResults.map(r => ({ ...r, kind: 'file' as const }));
      }
      selectedIndex = 0;
    } catch (_) { results = []; }
    loading = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.preventDefault(); onClose(); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); selectedIndex = Math.min(results.length - 1, selectedIndex + 1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); selectedIndex = Math.max(0, selectedIndex - 1); }
    else if (e.key === 'Enter') { e.preventDefault(); selectResult(selectedIndex); }
    else if (e.key === 'Tab') { e.preventDefault(); toggleMode(); }
  }

  function toggleMode() {
    searchMode = searchMode === 'files' ? 'content' : 'files';
    if (query.trim()) doSearch();
  }

  function selectResult(idx: number) {
    const r = results[idx];
    if (!r) return;
    if (r.kind === 'file') {
      if (r.is_dir) { onNavigate(r.path); }
      else { const parent = r.path.split('/').slice(0, -1).join('/') || '/'; onNavigate(parent); }
    } else {
      const parent = r.file_path.split('/').slice(0, -1).join('/') || '/';
      onNavigate(parent);
    }
    onClose();
  }

  function getResultPath(r: SearchResult): string {
    return r.kind === 'file' ? r.path : r.file_path;
  }

  function getResultName(r: SearchResult): string {
    return r.filename;
  }

  function getIcon(r: SearchResult): string {
    if (r.kind === 'file' && r.is_dir) return 'folder';
    const ext = r.kind === 'file' ? r.extension?.toLowerCase() : r.filename.split('.').pop()?.toLowerCase();
    if (!ext) return 'description';
    if (['jpg', 'jpeg', 'png', 'gif', 'webp'].includes(ext)) return 'image';
    if (['rs', 'py', 'js', 'ts'].includes(ext)) return 'code';
    if (['pdf'].includes(ext)) return 'picture_as_pdf';
    if (['md', 'markdown'].includes(ext)) return 'article';
    return 'description';
  }
</script>

{#if visible}
  <div class="overlay" on:click|self={onClose}>
    <div class="search-palette" on:keydown={handleKeydown}>
      <div class="search-header">
        <span class="material-symbols-outlined header-icon">search</span>
        <input
          class="search-input"
          type="text"
          bind:value={query}
          bind:this={inputEl}
          on:input={handleInput}
          placeholder={searchMode === 'files' ? 'Search files by name...' : 'Search file contents...'}
        />
        <button class="mode-toggle" on:click={toggleMode} title="Tab to toggle">
          <span class="material-symbols-outlined" style="font-size: 16px;">{searchMode === 'files' ? 'insert_drive_file' : 'text_snippet'}</span>
          <span class="mode-label">{searchMode === 'files' ? 'Files' : 'Content'}</span>
        </button>
        <div class="esc-badge"><span>ESC</span></div>
      </div>
      <div class="search-results">
        {#if loading}
          <div class="search-status">Searching...</div>
        {:else if query && results.length === 0}
          <div class="search-status">No results</div>
        {:else}
          {#each results as result, i}
            <button
              class="result-item"
              class:selected={i === selectedIndex}
              on:click={() => selectResult(i)}
              on:mouseenter={() => selectedIndex = i}
            >
              <span class="material-symbols-outlined result-icon">{getIcon(result)}</span>
              <div class="result-info">
                <span class="result-name">{getResultName(result)}</span>
                {#if result.kind === 'content' && result.snippet}
                  <span class="result-snippet">{result.snippet.text}</span>
                {/if}
                <span class="result-path">{getResultPath(result)}</span>
              </div>
              {#if result.kind === 'content'}
                <span class="result-score">{result.score.toFixed(1)}</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
      <div class="search-footer">
        <div class="footer-hints">
          <div class="hint-group">
            <span class="hint-key">Tab</span>
            <span class="hint-label">switch mode</span>
          </div>
          <div class="hint-group">
            <span class="hint-key"><span class="material-symbols-outlined" style="font-size: 14px;">keyboard_arrow_up</span></span>
            <span class="hint-key"><span class="material-symbols-outlined" style="font-size: 14px;">keyboard_arrow_down</span></span>
            <span class="hint-label">navigate</span>
          </div>
          <div class="hint-group">
            <span class="hint-key">Enter</span>
            <span class="hint-label">open</span>
          </div>
        </div>
        <div class="footer-brand">
          <span class="hint-label">Search powered by</span>
          <span class="brand-text">Noema Intelligence</span>
        </div>
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
    padding-top: 12vh;
    z-index: 2000;
  }

  .search-palette {
    width: 680px;
    max-height: 560px;
    background: var(--glass-bg);
    backdrop-filter: blur(20px);
    border: 1px solid var(--text-outline);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 8px 32px var(--shadow), inset 0 0 15px rgba(208, 188, 255, 0.05), 0 0 20px rgba(208, 188, 255, 0.1);
    display: flex;
    flex-direction: column;
    align-self: flex-start;
  }

  .search-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--text-outline);
  }

  .header-icon {
    color: var(--accent-primary);
    font-size: 20px;
  }

  .search-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 500;
    outline: none;
    letter-spacing: -0.01em;
  }

  .search-input::placeholder {
    color: var(--text-outline);
  }

  .esc-badge {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    background: var(--bg-container-highest);
    border-radius: 6px;
    border: 1px solid var(--text-outline);
  }

  .esc-badge span {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .mode-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    background: var(--bg-container-highest);
    border-radius: 6px;
    border: 1px solid var(--text-outline);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }

  .mode-toggle:hover {
    background: var(--bg-container-high);
    color: var(--text-primary);
  }

  .mode-label {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .result-snippet {
    font-size: 12px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 500px;
  }

  .result-score {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .search-results {
    overflow-y: auto;
    max-height: 400px;
    padding: 4px;
  }

  .search-status {
    padding: 24px 20px;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }

  .result-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px 16px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: none;
    color: var(--text-primary);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s;
  }

  .result-item:hover,
  .result-item.selected {
    background: var(--bg-container-high);
    border-color: var(--text-outline);
  }

  .result-icon {
    color: var(--text-secondary);
    font-size: 20px;
    flex-shrink: 0;
  }

  .result-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .result-name {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 500;
  }

  .result-path {
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
  }

  .search-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 20px;
    border-top: 1px solid var(--text-outline);
    background: var(--bg-container-lowest, rgba(14, 14, 14, 0.5));
  }

  .footer-hints {
    display: flex;
    gap: 16px;
  }

  .hint-group {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .hint-key {
    display: flex;
    align-items: center;
    padding: 2px 6px;
    background: var(--bg-container-highest);
    border-radius: 4px;
    border: 1px solid var(--text-outline);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .hint-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .footer-brand {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .brand-text {
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 700;
    color: var(--accent-primary);
    letter-spacing: -0.02em;
  }
</style>
