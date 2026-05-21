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

  // Selection
  let selectedPaths: Set<string> = new Set();
  let lastSelectedIndex = -1;

  // Context menu
  let contextMenu = { visible: false, x: 0, y: 0 };

  // Inline rename
  let renamingPath = '';
  let renameValue = '';

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
      selectedPaths = new Set();
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
      navigateTo(pathHistory[historyIndex]);
    }
  }

  function goForward() {
    if (historyIndex < pathHistory.length - 1) {
      historyIndex++;
      navigateTo(pathHistory[historyIndex]);
    }
  }

  function openEntry(entry: FileEntry) {
    if (entry.is_dir) {
      navigateTo(entry.path);
    }
  }

  function selectEntry(entry: FileEntry, index: number, event: MouseEvent) {
    if (event.metaKey || event.ctrlKey) {
      const next = new Set(selectedPaths);
      if (next.has(entry.path)) {
        next.delete(entry.path);
      } else {
        next.add(entry.path);
      }
      selectedPaths = next;
    } else if (event.shiftKey && lastSelectedIndex >= 0) {
      const start = Math.min(lastSelectedIndex, index);
      const end = Math.max(lastSelectedIndex, index);
      const next = new Set(selectedPaths);
      for (let i = start; i <= end; i++) {
        next.add(entries[i].path);
      }
      selectedPaths = next;
    } else {
      selectedPaths = new Set([entry.path]);
    }
    lastSelectedIndex = index;
  }

  function selectAll() {
    selectedPaths = new Set(entries.map(e => e.path));
  }

  // Context menu
  function showContextMenu(event: MouseEvent, entry?: FileEntry) {
    event.preventDefault();
    if (entry && !selectedPaths.has(entry.path)) {
      selectedPaths = new Set([entry.path]);
    }
    contextMenu = { visible: true, x: event.clientX, y: event.clientY };
  }

  function hideContextMenu() {
    contextMenu = { visible: false, x: 0, y: 0 };
  }

  // File operations
  async function handleDelete() {
    hideContextMenu();
    if (selectedPaths.size === 0) return;
    try {
      await invoke('delete_files', { paths: [...selectedPaths], useTrash: true });
      await navigateTo(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  function startRename() {
    hideContextMenu();
    if (selectedPaths.size !== 1) return;
    const path = [...selectedPaths][0];
    const entry = entries.find(e => e.path === path);
    if (entry) {
      renamingPath = path;
      renameValue = entry.filename;
    }
  }

  async function commitRename() {
    if (!renamingPath || !renameValue) {
      renamingPath = '';
      return;
    }
    try {
      await invoke('rename_file', { path: renamingPath, newName: renameValue });
      renamingPath = '';
      await navigateTo(currentPath);
    } catch (e) {
      error = String(e);
      renamingPath = '';
    }
  }

  async function handleNewFolder() {
    hideContextMenu();
    const name = 'New Folder';
    const path = `${currentPath}/${name}`;
    try {
      await invoke('create_directory', { path });
      await navigateTo(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  async function handleNewFile() {
    hideContextMenu();
    const name = 'untitled';
    const path = `${currentPath}/${name}`;
    try {
      await invoke('create_file', { path });
      await navigateTo(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  async function handleUndo() {
    try {
      await invoke('undo');
      await navigateTo(currentPath);
    } catch (e) {
      error = String(e);
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
    if (renamingPath) return;
    if (e.key === 'Backspace' && !e.metaKey) {
      goUp();
    } else if (e.key === 'a' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      selectAll();
    } else if (e.key === 'z' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleUndo();
    } else if (e.key === 'Delete' || (e.key === 'Backspace' && e.metaKey)) {
      handleDelete();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} on:click={hideContextMenu} />

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

  <div class="file-list" on:contextmenu={(e) => showContextMenu(e)}>
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
      {#each entries as entry, i}
        <div
          class="list-row"
          class:is-dir={entry.is_dir}
          class:selected={selectedPaths.has(entry.path)}
          on:click={(e) => selectEntry(entry, i, e)}
          on:dblclick={() => openEntry(entry)}
          on:contextmenu|stopPropagation={(e) => showContextMenu(e, entry)}
          tabindex="0"
          on:keydown={(e) => { if (e.key === 'Enter') openEntry(entry); }}
        >
          <div class="col-icon">{getIcon(entry)}</div>
          <div class="col-name">
            {#if renamingPath === entry.path}
              <input
                class="rename-input"
                type="text"
                bind:value={renameValue}
                on:blur={commitRename}
                on:keydown={(e) => { if (e.key === 'Enter') commitRename(); if (e.key === 'Escape') { renamingPath = ''; } }}
                autofocus
              />
            {:else}
              {entry.filename}
            {/if}
          </div>
          <div class="col-size">{entry.is_dir ? '—' : formatSize(entry.size)}</div>
          <div class="col-modified">{formatDate(entry.modified)}</div>
        </div>
      {/each}
      {#if entries.length === 0}
        <div class="empty">Empty directory</div>
      {/if}
    {/if}
  </div>

  {#if contextMenu.visible}
    <div class="context-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px;">
      {#if selectedPaths.size > 0}
        <button on:click={startRename} disabled={selectedPaths.size !== 1}>Rename</button>
        <button on:click={handleDelete}>Move to Trash</button>
        <hr />
      {/if}
      <button on:click={handleNewFolder}>New Folder</button>
      <button on:click={handleNewFile}>New File</button>
    </div>
  {/if}

  <footer class="status-bar">
    <span>{selectedPaths.size > 0 ? `${selectedPaths.size} selected` : `${entries.length} items`}</span>
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
    outline: none;
  }

  .list-row.selected {
    background: #45475a;
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

  .rename-input {
    width: 100%;
    padding: 1px 4px;
    border: 1px solid #89b4fa;
    border-radius: 3px;
    background: #1e1e2e;
    color: #cdd6f4;
    font-size: 13px;
    outline: none;
  }

  .context-menu {
    position: fixed;
    background: #313244;
    border: 1px solid #45475a;
    border-radius: 6px;
    padding: 4px 0;
    min-width: 160px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    z-index: 1000;
  }

  .context-menu button {
    display: block;
    width: 100%;
    padding: 6px 14px;
    border: none;
    background: none;
    color: #cdd6f4;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .context-menu button:hover:not(:disabled) {
    background: #45475a;
  }

  .context-menu button:disabled {
    color: #6c7086;
    cursor: default;
  }

  .context-menu hr {
    border: none;
    border-top: 1px solid #45475a;
    margin: 4px 0;
  }
</style>
