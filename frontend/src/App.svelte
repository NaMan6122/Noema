<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import ProgressToast from './lib/ProgressToast.svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import FileList from './lib/FileList.svelte';
  import TabBar from './lib/TabBar.svelte';

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

  interface TabState {
    id: string;
    path: string;
    title: string;
    entries: FileEntry[];
    selectedPaths: Set<string>;
    pathHistory: string[];
    historyIndex: number;
    sortField: string;
    sortDirection: string;
  }

  let tabs: TabState[] = [];
  let activeTabId = '';
  let showHidden = false;
  let loading = false;
  let error = '';

  // Inline rename
  let renamingPath = '';
  let renameValue = '';

  // Drag-drop
  let dragOverPath = '';

  // Context menu
  let contextMenu = { visible: false, x: 0, y: 0 };

  // Breadcrumb
  let editingPath = false;
  let pathInputValue = '';

  $: activeTab = tabs.find(t => t.id === activeTabId);
  $: currentPath = activeTab?.path ?? '';
  $: entries = activeTab?.entries ?? [];
  $: selectedPaths = activeTab?.selectedPaths ?? new Set<string>();
  $: sortField = activeTab?.sortField ?? 'name';
  $: sortDirection = activeTab?.sortDirection ?? 'asc';
  $: tabBarData = tabs.map(t => ({ id: t.id, path: t.path, title: t.title }));

  function genId(): string {
    return Math.random().toString(36).slice(2, 10);
  }

  function titleFromPath(path: string): string {
    const parts = path.split('/').filter(Boolean);
    return parts.length > 0 ? parts[parts.length - 1] : '/';
  }

  function updateActiveTab(update: Partial<TabState>) {
    tabs = tabs.map(t => t.id === activeTabId ? { ...t, ...update } as TabState : t);
  }

  onMount(async () => {
    const home = await invoke<string>('get_home_dir');
    const id = genId();
    tabs = [{
      id,
      path: home,
      title: titleFromPath(home),
      entries: [],
      selectedPaths: new Set(),
      pathHistory: [],
      historyIndex: -1,
      sortField: 'name',
      sortDirection: 'asc',
    }];
    activeTabId = id;
    await loadDirectory(home);

    listen('fs:changed', () => {
      loadDirectory(currentPath);
    });
  });

  async function loadDirectory(path: string) {
    if (!activeTab) return;
    loading = true;
    error = '';
    editingPath = false;
    try {
      const newEntries = await invoke<FileEntry[]>('list_directory', {
        path,
        sortField: activeTab.sortField,
        sortDirection: activeTab.sortDirection,
        showHidden,
      });

      let newHistory = activeTab.pathHistory;
      let newHistoryIndex = activeTab.historyIndex;
      if (activeTab.path !== path) {
        newHistory = [...activeTab.pathHistory.slice(0, activeTab.historyIndex + 1), path];
        newHistoryIndex = newHistory.length - 1;
      }

      updateActiveTab({
        path,
        title: titleFromPath(path),
        entries: newEntries,
        selectedPaths: new Set(),
        pathHistory: newHistory,
        historyIndex: newHistoryIndex,
      });
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  function navigateTo(path: string) {
    loadDirectory(path);
  }

  function goUp() {
    const parent = currentPath.split('/').slice(0, -1).join('/') || '/';
    navigateTo(parent);
  }

  function goBack() {
    if (!activeTab || activeTab.historyIndex <= 0) return;
    const newIdx = activeTab.historyIndex - 1;
    updateActiveTab({ historyIndex: newIdx });
    loadDirectoryNoHistory(activeTab.pathHistory[newIdx]);
  }

  function goForward() {
    if (!activeTab || activeTab.historyIndex >= activeTab.pathHistory.length - 1) return;
    const newIdx = activeTab.historyIndex + 1;
    updateActiveTab({ historyIndex: newIdx });
    loadDirectoryNoHistory(activeTab.pathHistory[newIdx]);
  }

  async function loadDirectoryNoHistory(path: string) {
    if (!activeTab) return;
    loading = true;
    error = '';
    editingPath = false;
    try {
      const newEntries = await invoke<FileEntry[]>('list_directory', {
        path,
        sortField: activeTab.sortField,
        sortDirection: activeTab.sortDirection,
        showHidden,
      });
      updateActiveTab({
        path,
        title: titleFromPath(path),
        entries: newEntries,
        selectedPaths: new Set(),
      });
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  function openEntry(entry: FileEntry) {
    if (entry.is_dir) {
      navigateTo(entry.path);
    }
  }

  function selectEntry(entry: FileEntry, index: number, event: MouseEvent) {
    if (!activeTab) return;
    const prev = activeTab.selectedPaths;
    let next: Set<string>;

    if (event.metaKey || event.ctrlKey) {
      next = new Set(prev);
      if (next.has(entry.path)) next.delete(entry.path);
      else next.add(entry.path);
    } else if (event.shiftKey) {
      next = new Set(prev);
      // Simple range select from last click
      const lastIdx = entries.findIndex(e => prev.has(e.path));
      if (lastIdx >= 0) {
        const start = Math.min(lastIdx, index);
        const end = Math.max(lastIdx, index);
        for (let i = start; i <= end; i++) next.add(entries[i].path);
      } else {
        next.add(entry.path);
      }
    } else {
      next = new Set([entry.path]);
    }
    updateActiveTab({ selectedPaths: next });
  }

  function selectAll() {
    updateActiveTab({ selectedPaths: new Set(entries.map(e => e.path)) });
  }

  // Tabs
  function newTab() {
    const id = genId();
    const path = currentPath || '/';
    tabs = [...tabs, {
      id,
      path,
      title: titleFromPath(path),
      entries: [],
      selectedPaths: new Set(),
      pathHistory: [],
      historyIndex: -1,
      sortField: 'name',
      sortDirection: 'asc',
    }];
    activeTabId = id;
    loadDirectory(path);
  }

  function closeTab(id: string) {
    if (tabs.length <= 1) return;
    const idx = tabs.findIndex(t => t.id === id);
    tabs = tabs.filter(t => t.id !== id);
    if (activeTabId === id) {
      activeTabId = tabs[Math.min(idx, tabs.length - 1)].id;
    }
  }

  function selectTab(id: string) {
    activeTabId = id;
    error = '';
  }

  // Breadcrumb
  $: pathSegments = buildBreadcrumb(currentPath);

  function buildBreadcrumb(path: string): { name: string; path: string }[] {
    if (!path) return [];
    const parts = path.split('/').filter(Boolean);
    const segments = [{ name: '/', path: '/' }];
    let accumulated = '';
    for (const part of parts) {
      accumulated += '/' + part;
      segments.push({ name: part, path: accumulated });
    }
    return segments;
  }

  function startEditingPath() {
    editingPath = true;
    pathInputValue = currentPath;
  }

  function commitPathEdit(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      editingPath = false;
      navigateTo(pathInputValue);
    } else if (e.key === 'Escape') {
      editingPath = false;
    }
  }

  // Context menu
  function showContextMenu(event: MouseEvent, entry?: FileEntry) {
    event.preventDefault();
    if (entry && !selectedPaths.has(entry.path)) {
      updateActiveTab({ selectedPaths: new Set([entry.path]) });
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
      await loadDirectory(currentPath);
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
      await loadDirectory(currentPath);
    } catch (e) {
      error = String(e);
      renamingPath = '';
    }
  }

  function cancelRename() {
    renamingPath = '';
  }

  async function handleNewFolder() {
    hideContextMenu();
    const path = `${currentPath}/New Folder`;
    try {
      await invoke('create_directory', { path });
      await loadDirectory(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  async function handleNewFile() {
    hideContextMenu();
    const path = `${currentPath}/untitled`;
    try {
      await invoke('create_file', { path });
      await loadDirectory(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  async function handleUndo() {
    try {
      await invoke('undo');
      await loadDirectory(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  async function handleRedo() {
    try {
      await invoke('redo');
      await loadDirectory(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  function handleDragStart(event: DragEvent, entry: FileEntry) {
    if (!event.dataTransfer) return;
    const paths = selectedPaths.has(entry.path) ? [...selectedPaths] : [entry.path];
    event.dataTransfer.setData('application/json', JSON.stringify(paths));
    event.dataTransfer.effectAllowed = 'copyMove';
  }

  function handleDragOver(event: DragEvent, entry: FileEntry) {
    if (!entry.is_dir) return;
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = event.altKey ? 'copy' : 'move';
    }
    dragOverPath = entry.path;
  }

  function handleDragLeave() {
    dragOverPath = '';
  }

  async function handleDrop(event: DragEvent, entry: FileEntry) {
    event.preventDefault();
    dragOverPath = '';
    if (!entry.is_dir || !event.dataTransfer) return;

    const raw = event.dataTransfer.getData('application/json');
    if (!raw) return;
    const sources: string[] = JSON.parse(raw);

    try {
      if (event.altKey) {
        await invoke('copy_files', { sources, dest: entry.path });
      } else {
        await invoke('move_files', { sources, dest: entry.path });
      }
    } catch (e) {
      error = String(e);
    }
  }

  function toggleSort(field: string) {
    if (!activeTab) return;
    let newDirection = 'asc';
    if (activeTab.sortField === field) {
      newDirection = activeTab.sortDirection === 'asc' ? 'desc' : 'asc';
    }
    updateActiveTab({ sortField: field, sortDirection: newDirection });
    loadDirectory(currentPath);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (renamingPath || editingPath) return;

    if ((e.metaKey || e.ctrlKey) && e.key === 't') {
      e.preventDefault();
      newTab();
    } else if ((e.metaKey || e.ctrlKey) && e.key === 'w') {
      e.preventDefault();
      closeTab(activeTabId);
    } else if ((e.metaKey || e.ctrlKey) && e.key >= '1' && e.key <= '9') {
      e.preventDefault();
      const idx = parseInt(e.key) - 1;
      if (idx < tabs.length) activeTabId = tabs[idx].id;
    } else if (e.key === 'Backspace' && !e.metaKey) {
      goUp();
    } else if (e.key === 'a' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      selectAll();
    } else if (e.key === 'z' && (e.metaKey || e.ctrlKey) && e.shiftKey) {
      e.preventDefault();
      handleRedo();
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
  <TabBar
    tabs={tabBarData}
    {activeTabId}
    onSelect={selectTab}
    onClose={closeTab}
    onNew={newTab}
  />

  <header class="toolbar">
    <div class="nav-buttons">
      <button on:click={goBack} disabled={!activeTab || activeTab.historyIndex <= 0} title="Back">←</button>
      <button on:click={goForward} disabled={!activeTab || activeTab.historyIndex >= activeTab.pathHistory.length - 1} title="Forward">→</button>
      <button on:click={goUp} title="Go up">↑</button>
    </div>
    <div class="breadcrumb">
      {#if editingPath}
        <input
          class="path-input"
          type="text"
          bind:value={pathInputValue}
          on:keydown={commitPathEdit}
          on:blur={() => editingPath = false}
          autofocus
        />
      {:else}
        <div class="breadcrumb-segments" on:dblclick={startEditingPath}>
          {#each pathSegments as seg, i}
            {#if i > 0}
              <span class="breadcrumb-sep">/</span>
            {/if}
            {#if i === pathSegments.length - 1}
              <span class="breadcrumb-current">{seg.name}</span>
            {:else}
              <button class="breadcrumb-link" on:click={() => navigateTo(seg.path)}>{seg.name}</button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
    <div class="actions">
      <label>
        <input type="checkbox" bind:checked={showHidden} on:change={() => loadDirectory(currentPath)} />
        Hidden
      </label>
    </div>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="main-content">
    <Sidebar {currentPath} onNavigate={navigateTo} />

    <div class="content-area">
      {#if loading}
        <div class="loading">Loading...</div>
      {:else}
        <FileList
          {entries}
          {selectedPaths}
          {sortField}
          {sortDirection}
          {renamingPath}
          bind:renameValue
          {dragOverPath}
          onSelect={selectEntry}
          onOpen={openEntry}
          onContextMenu={showContextMenu}
          onDragStart={handleDragStart}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          onToggleSort={toggleSort}
          onCommitRename={commitRename}
          onCancelRename={cancelRename}
        />
      {/if}
    </div>
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

  <ProgressToast />
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
    min-width: 0;
  }

  .path-input {
    width: 100%;
    padding: 5px 10px;
    border: 1px solid #45475a;
    border-radius: 4px;
    background: #1e1e2e;
    color: #cdd6f4;
    font-size: 12px;
    font-family: 'SF Mono', Monaco, monospace;
    box-sizing: border-box;
  }

  .path-input:focus {
    outline: none;
    border-color: #89b4fa;
  }

  .breadcrumb-segments {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: 4px;
    background: #1e1e2e;
    border: 1px solid transparent;
    min-height: 28px;
    overflow: hidden;
    cursor: text;
  }

  .breadcrumb-segments:hover {
    border-color: #45475a;
  }

  .breadcrumb-link {
    border: none;
    background: none;
    color: #89b4fa;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 12px;
    white-space: nowrap;
  }

  .breadcrumb-link:hover {
    background: #313244;
    text-decoration: underline;
  }

  .breadcrumb-sep {
    color: #6c7086;
    margin: 0 1px;
    font-size: 12px;
  }

  .breadcrumb-current {
    color: #cdd6f4;
    font-weight: 500;
    font-size: 12px;
    padding: 2px 4px;
    white-space: nowrap;
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

  .main-content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .loading {
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
