<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface FileEntry {
    path: string;
    filename: string;
    extension: string | null;
    size: number;
    created: string;
    modified: string;
    is_dir: boolean;
    is_hidden: boolean;
    is_symlink: boolean;
  }

  let entries: FileEntry[] = [];
  let currentPath = '';
  let pathHistory: string[] = [];
  let historyIndex = -1;
  let sortField = 'name';
  let sortDirection = 'asc';
  let showHidden = false;
  let loading = false;
  let error = '';

  onMount(async () => {
    const home = await invoke<string>('get_home_dir');
    await navigateTo(home);
  });

  async function navigateTo(path: string) {
    loading = true;
    error = '';
    try {
      entries = await invoke<FileEntry[]>('list_directory', {
        path,
        sortField,
        sortDirection,
        showHidden,
      });
      if (currentPath !== path) {
        pathHistory = [...pathHistory.slice(0, historyIndex + 1), path];
        historyIndex = pathHistory.length - 1;
      }
      currentPath = path;
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  function goUp() {
    const parent = currentPath.split('/').slice(0, -1).join('/') || '/';
    navigateTo(parent);
  }

  function goBack() {
    if (historyIndex > 0) {
      historyIndex--;
      currentPath = pathHistory[historyIndex];
      navigateTo(currentPath);
    }
  }

  function goForward() {
    if (historyIndex < pathHistory.length - 1) {
      historyIndex++;
      currentPath = pathHistory[historyIndex];
      navigateTo(currentPath);
    }
  }

  function openEntry(entry: FileEntry) {
    if (entry.is_dir) {
      navigateTo(entry.path);
    }
  }

  function toggleSort(field: string) {
    if (sortField === field) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortField = field;
      sortDirection = 'asc';
    }
    navigateTo(currentPath);
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return '—';
    const units = ['B', 'KB', 'MB', 'GB'];
    let i = 0;
    let size = bytes;
    while (size >= 1024 && i < units.length - 1) {
      size /= 1024;
      i++;
    }
    return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function formatDate(iso: string): string {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }

  function getIcon(entry: FileEntry): string {
    if (entry.is_dir) return '📁';
    const ext = entry.extension?.toLowerCase();
    if (!ext) return '📄';
    if (['pdf'].includes(ext)) return '📕';
    if (['doc', 'docx'].includes(ext)) return '📘';
    if (['xls', 'xlsx', 'csv'].includes(ext)) return '📊';
    if (['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp'].includes(ext)) return '🖼️';
    if (['mp3', 'wav', 'flac', 'aac'].includes(ext)) return '🎵';
    if (['mp4', 'mov', 'avi', 'mkv'].includes(ext)) return '🎬';
    if (['zip', 'tar', 'gz', 'rar', '7z'].includes(ext)) return '📦';
    if (['rs', 'py', 'js', 'ts', 'go', 'c', 'cpp', 'java'].includes(ext)) return '⚙️';
    if (['md', 'txt', 'rtf'].includes(ext)) return '📝';
    return '📄';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Backspace' && !e.metaKey) {
      goUp();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="app">
  <header class="toolbar">
    <div class="nav-buttons">
      <button on:click={goBack} disabled={historyIndex <= 0} title="Back">←</button>
      <button on:click={goForward} disabled={historyIndex >= pathHistory.length - 1} title="Forward">→</button>
      <button on:click={goUp} title="Go up">↑</button>
    </div>
    <div class="breadcrumb">
      <input
        type="text"
        value={currentPath}
        on:keydown={(e) => { if (e.key === 'Enter') navigateTo(e.currentTarget.value); }}
      />
    </div>
    <div class="actions">
      <label>
        <input type="checkbox" bind:checked={showHidden} on:change={() => navigateTo(currentPath)} />
        Hidden
      </label>
    </div>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="file-list">
    <div class="list-header">
      <div class="col-icon"></div>
      <div class="col-name" on:click={() => toggleSort('name')}>
        Name {sortField === 'name' ? (sortDirection === 'asc' ? '▲' : '▼') : ''}
      </div>
      <div class="col-size" on:click={() => toggleSort('size')}>
        Size {sortField === 'size' ? (sortDirection === 'asc' ? '▲' : '▼') : ''}
      </div>
      <div class="col-modified" on:click={() => toggleSort('modified')}>
        Modified {sortField === 'modified' ? (sortDirection === 'asc' ? '▲' : '▼') : ''}
      </div>
    </div>

    {#if loading}
      <div class="loading">Loading...</div>
    {:else}
      {#each entries as entry}
        <div
          class="list-row"
          class:is-dir={entry.is_dir}
          on:dblclick={() => openEntry(entry)}
          tabindex="0"
          on:keydown={(e) => { if (e.key === 'Enter') openEntry(entry); }}
        >
          <div class="col-icon">{getIcon(entry)}</div>
          <div class="col-name">{entry.filename}</div>
          <div class="col-size">{entry.is_dir ? '—' : formatSize(entry.size)}</div>
          <div class="col-modified">{formatDate(entry.modified)}</div>
        </div>
      {/each}
      {#if entries.length === 0}
        <div class="empty">Empty directory</div>
      {/if}
    {/if}
  </div>

  <footer class="status-bar">
    <span>{entries.length} items</span>
    <span>{currentPath}</span>
  </footer>
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    font-size: 13px;
    color: #e0e0e0;
    background: #1e1e2e;
    overflow: hidden;
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: #181825;
    border-bottom: 1px solid #313244;
    -webkit-app-region: drag;
  }

  .nav-buttons {
    display: flex;
    gap: 4px;
    -webkit-app-region: no-drag;
  }

  .nav-buttons button {
    padding: 4px 10px;
    border: 1px solid #45475a;
    border-radius: 4px;
    background: #313244;
    color: #cdd6f4;
    cursor: pointer;
    font-size: 14px;
  }

  .nav-buttons button:hover:not(:disabled) {
    background: #45475a;
  }

  .nav-buttons button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .breadcrumb {
    flex: 1;
    -webkit-app-region: no-drag;
  }

  .breadcrumb input {
    width: 100%;
    padding: 5px 10px;
    border: 1px solid #45475a;
    border-radius: 4px;
    background: #1e1e2e;
    color: #cdd6f4;
    font-size: 12px;
    font-family: 'SF Mono', Monaco, monospace;
  }

  .breadcrumb input:focus {
    outline: none;
    border-color: #89b4fa;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: no-drag;
    color: #a6adc8;
    font-size: 12px;
  }

  .error {
    padding: 8px 12px;
    background: #f38ba820;
    color: #f38ba8;
    border-bottom: 1px solid #f38ba840;
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .list-header {
    display: grid;
    grid-template-columns: 32px 1fr 90px 120px;
    padding: 6px 12px;
    background: #181825;
    border-bottom: 1px solid #313244;
    font-weight: 600;
    color: #a6adc8;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    position: sticky;
    top: 0;
    cursor: pointer;
    user-select: none;
  }

  .list-row {
    display: grid;
    grid-template-columns: 32px 1fr 90px 120px;
    padding: 4px 12px;
    border-bottom: 1px solid #31324420;
    cursor: default;
    align-items: center;
  }

  .list-row:hover {
    background: #313244;
  }

  .list-row:focus {
    background: #45475a;
    outline: none;
  }

  .list-row.is-dir .col-name {
    font-weight: 500;
  }

  .col-icon {
    font-size: 16px;
    text-align: center;
  }

  .col-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding-right: 8px;
  }

  .col-size, .col-modified {
    color: #a6adc8;
    font-size: 12px;
    text-align: right;
  }

  .loading, .empty {
    padding: 40px;
    text-align: center;
    color: #6c7086;
  }

  .status-bar {
    display: flex;
    justify-content: space-between;
    padding: 4px 12px;
    background: #181825;
    border-top: 1px solid #313244;
    color: #6c7086;
    font-size: 11px;
  }
</style>
