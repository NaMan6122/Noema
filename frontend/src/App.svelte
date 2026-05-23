<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { resolvedTheme, initTheme } from './lib/themeStore';
  import ProgressToast from './lib/ProgressToast.svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import FileList from './lib/FileList.svelte';
  import TabBar from './lib/TabBar.svelte';
  import CommandPalette from './lib/CommandPalette.svelte';
  import PreviewPane from './lib/PreviewPane.svelte';
  import InfoPanel from './lib/InfoPanel.svelte';
  import SearchBar from './lib/SearchBar.svelte';
  import GlobalSearch from './lib/GlobalSearch.svelte';

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
    viewMode: 'list' | 'grid';
  }

  let tabs: TabState[] = [];
  let activeTabId = '';
  let showHidden = false;
  let loading = false;
  let error = '';

  let renamingPath = '';
  let renameValue = '';
  let dragOverPath = '';
  let contextMenu = { visible: false, x: 0, y: 0 };
  let previewVisible = false;
  let infoVisible = false;
  let searchVisible = false;
  let searchQuery = '';
  let globalSearchVisible = false;
  let diskInfo: { total: number; available: number; used: number } | null = null;

  function formatSize(bytes: number): string {
    if (bytes >= 1e12) return (bytes / 1e12).toFixed(1) + ' TB';
    if (bytes >= 1e9) return (bytes / 1e9).toFixed(1) + ' GB';
    if (bytes >= 1e6) return (bytes / 1e6).toFixed(1) + ' MB';
    if (bytes >= 1e3) return (bytes / 1e3).toFixed(1) + ' KB';
    return bytes + ' B';
  }

  let paletteVisible = false;

  $: paletteCommands = [
    { id: 'home', label: 'Go to Home', action: () => { dirs('home_dir'); } },
    { id: 'desktop', label: 'Go to Desktop', action: () => { dirs('desktop_dir'); } },
    { id: 'documents', label: 'Go to Documents', action: () => { dirs('document_dir'); } },
    { id: 'downloads', label: 'Go to Downloads', action: () => { dirs('download_dir'); } },
    { id: 'new-tab', label: 'New Tab', shortcut: '⌘T', action: newTab },
    { id: 'close-tab', label: 'Close Tab', shortcut: '⌘W', action: () => closeTab(activeTabId) },
    { id: 'toggle-hidden', label: 'Toggle Hidden Files', action: () => { showHidden = !showHidden; loadDirectory(currentPath); } },
    { id: 'new-folder', label: 'New Folder', action: handleNewFolder },
    { id: 'new-file', label: 'New File', action: handleNewFile },
    { id: 'undo', label: 'Undo', shortcut: '⌘Z', action: handleUndo },
    { id: 'redo', label: 'Redo', shortcut: '⌘⇧Z', action: handleRedo },
    { id: 'terminal', label: 'Open in Terminal', action: () => invoke('open_in_terminal', { path: currentPath }) },
  ];

  async function dirs(name: string) {
    try {
      const favorites = await invoke<Array<{ name: string; path: string; kind: string }>>('get_favorites');
      const map: Record<string, string> = {
        home_dir: 'Home', desktop_dir: 'Desktop', document_dir: 'Documents', download_dir: 'Downloads',
      };
      const fav = favorites.find(f => f.name === map[name]);
      if (fav) navigateTo(fav.path);
    } catch (_) {}
  }

  let editingPath = false;
  let pathInputValue = '';

  $: activeTab = tabs.find(t => t.id === activeTabId);
  $: currentPath = activeTab?.path ?? '';
  $: rawEntries = activeTab?.entries ?? [];
  $: entries = searchQuery
    ? rawEntries.filter(e => e.filename.toLowerCase().includes(searchQuery.toLowerCase()))
    : rawEntries;
  $: selectedPaths = activeTab?.selectedPaths ?? new Set<string>();
  $: sortField = activeTab?.sortField ?? 'name';
  $: sortDirection = activeTab?.sortDirection ?? 'asc';
  $: viewMode = activeTab?.viewMode ?? 'list';
  $: selectedEntry = selectedPaths.size === 1 ? entries.find(e => selectedPaths.has(e.path)) ?? null : null;
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
    const unsub = resolvedTheme.subscribe(t => {
      document.documentElement.setAttribute('data-theme', t);
    });
    await initTheme();
    let restoredTabs: Array<{ path: string; sortField: string; sortDirection: string }> | null = null;
    try {
      const json = await invoke<string | null>('load_workspace', { name: 'last_session' });
      if (json) restoredTabs = JSON.parse(json);
    } catch (_) {}

    if (restoredTabs && restoredTabs.length > 0) {
      tabs = restoredTabs.map(t => ({
        id: genId(),
        path: t.path,
        title: titleFromPath(t.path),
        entries: [],
        selectedPaths: new Set<string>(),
        pathHistory: [],
        historyIndex: -1,
        sortField: t.sortField || 'name',
        sortDirection: t.sortDirection || 'asc',
        viewMode: (t as any).viewMode || 'list',
      }));
      activeTabId = tabs[0].id;
      await loadDirectory(tabs[0].path);
    } else {
      const home = await invoke<string>('get_home_dir');
      const id = genId();
      tabs = [{
        id, path: home, title: titleFromPath(home), entries: [],
        selectedPaths: new Set(), pathHistory: [], historyIndex: -1,
        sortField: 'name', sortDirection: 'asc', viewMode: 'list',
      }];
      activeTabId = id;
      await loadDirectory(home);
    }

    listen('fs:changed', () => { loadDirectory(currentPath); });
    window.addEventListener('beforeunload', saveWorkspace);
    return () => window.removeEventListener('beforeunload', saveWorkspace);
  });

  function saveWorkspace() {
    const state = tabs.map(t => ({
      path: t.path, sortField: t.sortField, sortDirection: t.sortDirection, viewMode: t.viewMode,
    }));
    invoke('save_workspace', { name: 'last_session', stateJson: JSON.stringify(state) }).catch(() => {});
  }

  async function loadDirectory(path: string) {
    if (!activeTab) return;
    loading = true;
    error = '';
    editingPath = false;
    try {
      const newEntries = await invoke<FileEntry[]>('list_directory', {
        path, sortField: activeTab.sortField, sortDirection: activeTab.sortDirection, showHidden,
      });
      let newHistory = activeTab.pathHistory;
      let newHistoryIndex = activeTab.historyIndex;
      if (activeTab.path !== path) {
        newHistory = [...activeTab.pathHistory.slice(0, activeTab.historyIndex + 1), path];
        newHistoryIndex = newHistory.length - 1;
      }
      updateActiveTab({
        path, title: titleFromPath(path), entries: newEntries, selectedPaths: new Set(),
        pathHistory: newHistory, historyIndex: newHistoryIndex,
      });
    } catch (e) {
      error = String(e);
    }
    loading = false;
    invoke<{ total: number; available: number; used: number }>('get_disk_space', { path }).then(info => { diskInfo = info; }).catch(() => {});
  }

  function navigateTo(path: string) { loadDirectory(path); }
  function goUp() { const parent = currentPath.split('/').slice(0, -1).join('/') || '/'; navigateTo(parent); }

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
    loading = true; error = ''; editingPath = false;
    try {
      const newEntries = await invoke<FileEntry[]>('list_directory', {
        path, sortField: activeTab.sortField, sortDirection: activeTab.sortDirection, showHidden,
      });
      updateActiveTab({ path, title: titleFromPath(path), entries: newEntries, selectedPaths: new Set() });
    } catch (e) { error = String(e); }
    loading = false;
  }

  function openEntry(entry: FileEntry) {
    if (entry.is_dir) { navigateTo(entry.path); }
    else { invoke('log_file_open', { path: entry.path }).catch(() => {}); }
  }

  function selectEntry(entry: FileEntry, index: number, event: MouseEvent) {
    if (!activeTab) return;
    const prev = activeTab.selectedPaths;
    let next: Set<string>;
    if (event.metaKey || event.ctrlKey) {
      next = new Set(prev);
      if (next.has(entry.path)) next.delete(entry.path); else next.add(entry.path);
    } else if (event.shiftKey) {
      next = new Set(prev);
      const lastIdx = entries.findIndex(e => prev.has(e.path));
      if (lastIdx >= 0) {
        const start = Math.min(lastIdx, index);
        const end = Math.max(lastIdx, index);
        for (let i = start; i <= end; i++) next.add(entries[i].path);
      } else { next.add(entry.path); }
    } else { next = new Set([entry.path]); }
    updateActiveTab({ selectedPaths: next });
  }

  function selectAll() { updateActiveTab({ selectedPaths: new Set(entries.map(e => e.path)) }); }

  function newTab() {
    const id = genId();
    const path = currentPath || '/';
    tabs = [...tabs, {
      id, path, title: titleFromPath(path), entries: [], selectedPaths: new Set(),
      pathHistory: [], historyIndex: -1, sortField: 'name', sortDirection: 'asc', viewMode: 'list',
    }];
    activeTabId = id;
    loadDirectory(path);
  }

  function closeTab(id: string) {
    if (tabs.length <= 1) return;
    const idx = tabs.findIndex(t => t.id === id);
    tabs = tabs.filter(t => t.id !== id);
    if (activeTabId === id) { activeTabId = tabs[Math.min(idx, tabs.length - 1)].id; }
  }

  function selectTab(id: string) { activeTabId = id; error = ''; }

  $: pathSegments = buildBreadcrumb(currentPath);
  function buildBreadcrumb(path: string): { name: string; path: string }[] {
    if (!path) return [];
    const parts = path.split('/').filter(Boolean);
    const segments = [{ name: '/', path: '/' }];
    let accumulated = '';
    for (const part of parts) { accumulated += '/' + part; segments.push({ name: part, path: accumulated }); }
    return segments;
  }

  function startEditingPath() { editingPath = true; pathInputValue = currentPath; }
  function commitPathEdit(e: KeyboardEvent) {
    if (e.key === 'Enter') { editingPath = false; navigateTo(pathInputValue); }
    else if (e.key === 'Escape') { editingPath = false; }
  }

  function showContextMenu(event: MouseEvent, entry?: FileEntry) {
    event.preventDefault();
    if (entry && !selectedPaths.has(entry.path)) { updateActiveTab({ selectedPaths: new Set([entry.path]) }); }
    contextMenu = { visible: true, x: event.clientX, y: event.clientY };
  }
  function hideContextMenu() { contextMenu = { visible: false, x: 0, y: 0 }; }

  async function handleDelete() {
    hideContextMenu();
    if (selectedPaths.size === 0) return;
    try { await invoke('delete_files', { paths: [...selectedPaths], useTrash: true }); await loadDirectory(currentPath); }
    catch (e) { error = String(e); }
  }

  function startRename() {
    hideContextMenu();
    if (selectedPaths.size !== 1) return;
    const path = [...selectedPaths][0];
    const entry = entries.find(e => e.path === path);
    if (entry) { renamingPath = path; renameValue = entry.filename; }
  }

  async function commitRename() {
    if (!renamingPath || !renameValue) { renamingPath = ''; return; }
    try { await invoke('rename_file', { path: renamingPath, newName: renameValue }); renamingPath = ''; await loadDirectory(currentPath); }
    catch (e) { error = String(e); renamingPath = ''; }
  }
  function cancelRename() { renamingPath = ''; }

  async function handleNewFolder() {
    hideContextMenu();
    try { await invoke('create_directory', { path: `${currentPath}/New Folder` }); await loadDirectory(currentPath); }
    catch (e) { error = String(e); }
  }

  async function handleNewFile() {
    hideContextMenu();
    try { await invoke('create_file', { path: `${currentPath}/untitled` }); await loadDirectory(currentPath); }
    catch (e) { error = String(e); }
  }

  async function handleUndo() {
    try { await invoke('undo'); await loadDirectory(currentPath); } catch (e) { error = String(e); }
  }
  async function handleRedo() {
    try { await invoke('redo'); await loadDirectory(currentPath); } catch (e) { error = String(e); }
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
    if (event.dataTransfer) { event.dataTransfer.dropEffect = event.altKey ? 'copy' : 'move'; }
    dragOverPath = entry.path;
  }
  function handleDragLeave() { dragOverPath = ''; }

  async function handleDrop(event: DragEvent, entry: FileEntry) {
    event.preventDefault(); dragOverPath = '';
    if (!entry.is_dir || !event.dataTransfer) return;
    const raw = event.dataTransfer.getData('application/json');
    if (!raw) return;
    const sources: string[] = JSON.parse(raw);
    try {
      if (event.altKey) { await invoke('copy_files', { sources, dest: entry.path }); }
      else { await invoke('move_files', { sources, dest: entry.path }); }
    } catch (e) { error = String(e); }
  }

  function toggleSort(field: string) {
    if (!activeTab) return;
    let newDirection = 'asc';
    if (activeTab.sortField === field) { newDirection = activeTab.sortDirection === 'asc' ? 'desc' : 'asc'; }
    updateActiveTab({ sortField: field, sortDirection: newDirection });
    loadDirectory(currentPath);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (renamingPath || editingPath) return;
    if ((e.metaKey || e.ctrlKey) && e.key === 'p') { e.preventDefault(); globalSearchVisible = !globalSearchVisible; }
    else if ((e.metaKey || e.ctrlKey) && e.key === 'k') { e.preventDefault(); paletteVisible = !paletteVisible; }
    else if ((e.metaKey || e.ctrlKey) && e.key === 't') { e.preventDefault(); newTab(); }
    else if ((e.metaKey || e.ctrlKey) && e.key === 'w') { e.preventDefault(); closeTab(activeTabId); }
    else if ((e.metaKey || e.ctrlKey) && e.key === '1') { e.preventDefault(); updateActiveTab({ viewMode: 'list' }); }
    else if ((e.metaKey || e.ctrlKey) && e.key === '2') { e.preventDefault(); updateActiveTab({ viewMode: 'grid' }); }
    else if ((e.metaKey || e.ctrlKey) && e.key >= '3' && e.key <= '9') { e.preventDefault(); const idx = parseInt(e.key) - 1; if (idx < tabs.length) activeTabId = tabs[idx].id; }
    else if (e.key === 'Backspace' && !e.metaKey) { goUp(); }
    else if (e.key === 'a' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); selectAll(); }
    else if (e.key === 'z' && (e.metaKey || e.ctrlKey) && e.shiftKey) { e.preventDefault(); handleRedo(); }
    else if (e.key === 'z' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); handleUndo(); }
    else if ((e.metaKey || e.ctrlKey) && e.key === 'i') { e.preventDefault(); infoVisible = !infoVisible; }
    else if ((e.metaKey || e.ctrlKey) && e.key === 'f') { e.preventDefault(); searchVisible = !searchVisible; if (!searchVisible) searchQuery = ''; }
    else if (e.key === 'Delete' || (e.key === 'Backspace' && e.metaKey)) { handleDelete(); }
    else if (e.key === ' ' && !e.metaKey && !e.ctrlKey) { e.preventDefault(); previewVisible = !previewVisible; }
  }
</script>

<svelte:window on:keydown={handleKeydown} on:click={hideContextMenu} />

<div class="app">
  <Sidebar {currentPath} onNavigate={navigateTo} />

  <div class="main-area">
    <TabBar
      tabs={tabBarData}
      {activeTabId}
      onSelect={selectTab}
      onClose={closeTab}
      onNew={newTab}
    />

    <header class="toolbar">
      <div class="nav-buttons">
        <button on:click={goBack} disabled={!activeTab || activeTab.historyIndex <= 0} title="Back">
          <span class="material-symbols-outlined">arrow_back</span>
        </button>
        <button on:click={goForward} disabled={!activeTab || activeTab.historyIndex >= activeTab.pathHistory.length - 1} title="Forward">
          <span class="material-symbols-outlined">arrow_forward</span>
        </button>
        <button on:click={goUp} title="Go up">
          <span class="material-symbols-outlined">arrow_upward</span>
        </button>
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

      <div class="toolbar-actions">
        <div class="search-field">
          <input
            class="search-input"
            type="text"
            placeholder="Search semantically..."
            on:focus={() => { globalSearchVisible = true; }}
            readonly
          />
          <span class="material-symbols-outlined search-field-icon">search</span>
        </div>
        <div class="toolbar-icons">
          <button class="icon-btn" title="Notifications">
            <span class="material-symbols-outlined">notifications</span>
          </button>
          <button class="icon-btn" title="History">
            <span class="material-symbols-outlined">history</span>
          </button>
          <button class="icon-btn" on:click={() => invoke('open_in_terminal', { path: currentPath })} title="Open in Terminal">
            <span class="material-symbols-outlined">bolt</span>
          </button>
        </div>
      </div>
    </header>

    <div class="sub-toolbar">
      <div class="view-toggles">
        <button
          class="view-btn"
          class:active={viewMode === 'list'}
          on:click={() => updateActiveTab({ viewMode: 'list' })}
        >
          <span class="material-symbols-outlined" style="font-size: 16px;">list</span>
          <span>List</span>
        </button>
        <button
          class="view-btn"
          class:active={viewMode === 'grid'}
          on:click={() => updateActiveTab({ viewMode: 'grid' })}
        >
          <span class="material-symbols-outlined" style="font-size: 16px;">grid_view</span>
          <span>Grid</span>
        </button>
      </div>
      <div class="sub-toolbar-right">
        <span class="item-count">{entries.length} Items</span>
        <div class="toolbar-divider"></div>
        <button class="filter-btn" on:click={() => { searchVisible = !searchVisible; if (!searchVisible) searchQuery = ''; }}>
          <span class="material-symbols-outlined" style="font-size: 16px;">filter_list</span>
          <span>Filter</span>
        </button>
        <button class="filter-btn" title="Sort options">
          <span class="material-symbols-outlined" style="font-size: 16px;">sort</span>
          <span>Sort</span>
        </button>
        <label class="hidden-toggle">
          <input type="checkbox" bind:checked={showHidden} on:change={() => loadDirectory(currentPath)} />
          <span>Hidden</span>
        </label>
      </div>
    </div>

    {#if error}
      <div class="error-bar">{error}</div>
    {/if}

    <div class="content-row">
      <div class="content-area">
        <SearchBar
          visible={searchVisible}
          bind:query={searchQuery}
          onClose={() => { searchVisible = false; searchQuery = ''; }}
        />
        {#if loading}
          <div class="loading">Loading...</div>
        {:else}
          <FileList
            {entries}
            {selectedPaths}
            {sortField}
            {sortDirection}
            {viewMode}
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

      <PreviewPane entry={selectedEntry} visible={previewVisible} />
    </div>

    <footer class="status-bar">
      <span>{selectedPaths.size > 0 ? `${selectedPaths.size} selected` : `${entries.length} items`}</span>
      <span class="status-disk">{diskInfo ? `${formatSize(diskInfo.available)} free of ${formatSize(diskInfo.total)}` : ''}</span>
      <span class="status-path">{currentPath}</span>
    </footer>
  </div>

  <!-- Collapsed Intelligence Strip -->
  <div class="intel-strip">
    <span class="material-symbols-outlined intel-icon" style="font-variation-settings: 'FILL' 1;">auto_awesome</span>
    <div class="intel-shimmer intelligence-shimmer"></div>
    <span class="material-symbols-outlined intel-expand">chevron_left</span>
  </div>

  {#if contextMenu.visible}
    <div class="context-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px;">
      {#if selectedPaths.size > 0}
        <button on:click={startRename} disabled={selectedPaths.size !== 1}>
          <span class="material-symbols-outlined" style="font-size: 16px;">edit</span>
          Rename
        </button>
        <button on:click={handleDelete}>
          <span class="material-symbols-outlined" style="font-size: 16px;">delete</span>
          Move to Trash
        </button>
        <hr />
      {/if}
      <button on:click={handleNewFolder}>
        <span class="material-symbols-outlined" style="font-size: 16px;">create_new_folder</span>
        New Folder
      </button>
      <button on:click={handleNewFile}>
        <span class="material-symbols-outlined" style="font-size: 16px;">note_add</span>
        New File
      </button>
      <hr />
      <button on:click={() => { invoke('open_in_terminal', { path: currentPath }); hideContextMenu(); }}>
        <span class="material-symbols-outlined" style="font-size: 16px;">terminal</span>
        Open in Terminal
      </button>
    </div>
  {/if}

  <InfoPanel path={selectedPaths.size === 1 ? [...selectedPaths][0] : null} bind:visible={infoVisible} />
  <ProgressToast />
  <CommandPalette visible={paletteVisible} commands={paletteCommands} onClose={() => paletteVisible = false} />
  <GlobalSearch visible={globalSearchVisible} {currentPath} onNavigate={navigateTo} onClose={() => globalSearchVisible = false} />

  <!-- Ambient Glows -->
  <div class="ambient-glow glow-bottom-right"></div>
  <div class="ambient-glow glow-top-left"></div>
</div>

<style>
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .main-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  /* Toolbar */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 16px;
    height: 48px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--text-outline);
    flex-shrink: 0;
  }

  .nav-buttons {
    display: flex;
    gap: 2px;
  }

  .nav-buttons button {
    padding: 6px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .nav-buttons button:hover:not(:disabled) {
    background: var(--bg-container-high);
    color: var(--text-primary);
  }

  .nav-buttons button:disabled {
    opacity: 0.3;
    cursor: default;
  }

  /* Breadcrumb */
  .breadcrumb {
    flex: 1;
    min-width: 0;
  }

  .path-input {
    width: 100%;
    padding: 6px 10px;
    border: 1px solid var(--text-outline);
    border-radius: 4px;
    background: var(--bg-container-lowest);
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
    box-sizing: border-box;
    outline: none;
  }

  .path-input:focus {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px rgba(208, 188, 255, 0.15);
  }

  .breadcrumb-segments {
    display: flex;
    align-items: center;
    padding: 4px 0;
    min-height: 28px;
    overflow: hidden;
    cursor: text;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .breadcrumb-link {
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 12px;
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  .breadcrumb-link:hover {
    color: var(--text-primary);
    background: var(--bg-container-high);
  }

  .breadcrumb-sep {
    color: var(--text-outline);
    margin: 0 2px;
  }

  .breadcrumb-current {
    color: var(--accent-primary);
    font-weight: 500;
    padding: 2px 4px;
    white-space: nowrap;
  }

  /* Toolbar actions */
  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .search-field {
    position: relative;
  }

  .search-input {
    width: 260px;
    padding: 6px 32px 6px 12px;
    background: var(--bg-container-low);
    border: 1px solid var(--text-outline);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-body);
    outline: none;
    cursor: pointer;
    transition: border-color 0.15s;
  }

  .search-input::placeholder {
    color: var(--text-secondary);
  }

  .search-input:focus {
    border-color: var(--accent-primary);
  }

  .search-field-icon {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 18px;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .toolbar-icons {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .icon-btn {
    padding: 6px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .icon-btn:hover {
    color: var(--accent-primary);
    background: var(--bg-container-high);
  }

  /* Sub toolbar */
  .sub-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding: 0 16px;
    border-bottom: 1px solid var(--text-outline);
    background: rgba(14, 14, 14, 0.5);
    backdrop-filter: blur(4px);
    flex-shrink: 0;
    font-size: 12px;
  }

  .view-toggles {
    display: flex;
    gap: 4px;
  }

  .view-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
  }

  .view-btn:hover {
    background: var(--bg-container-high);
  }

  .view-btn.active {
    background: rgba(71, 71, 70, 0.5);
    border-color: rgba(73, 68, 84, 0.3);
    color: var(--accent-on-secondary-container, var(--text-primary));
  }

  .sub-toolbar-right {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--text-secondary);
  }

  .item-count {
    font-weight: 500;
  }

  .toolbar-divider {
    width: 1px;
    height: 16px;
    background: var(--text-outline);
  }

  .filter-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
  }

  .filter-btn:hover {
    color: var(--text-primary);
    background: var(--bg-container-high);
  }

  .hidden-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    font-weight: 500;
  }

  .hidden-toggle input {
    accent-color: var(--accent-primary);
  }

  /* Error bar */
  .error-bar {
    padding: 8px 16px;
    background: rgba(255, 180, 171, 0.08);
    color: var(--accent-error);
    border-bottom: 1px solid rgba(255, 180, 171, 0.2);
    font-size: 13px;
    flex-shrink: 0;
  }

  /* Content */
  .content-row {
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
    color: var(--text-muted);
  }

  /* Status bar */
  .status-bar {
    display: flex;
    justify-content: space-between;
    padding: 4px 16px;
    background: var(--bg-container-low);
    border-top: 1px solid var(--text-outline);
    color: var(--text-muted);
    font-size: 11px;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .status-path {
    opacity: 0.7;
  }

  /* Intelligence strip */
  .intel-strip {
    width: 32px;
    min-width: 32px;
    background: var(--bg-container-low);
    border-left: 1px solid var(--text-outline);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 16px 0;
  }

  .intel-icon {
    color: var(--accent-primary);
    margin-bottom: 24px;
  }

  .intel-shimmer {
    flex: 1;
    width: 1px;
  }

  .intel-expand {
    color: var(--text-secondary);
    margin-top: 24px;
    transform: rotate(180deg);
    font-size: 16px;
  }

  /* Context menu */
  .context-menu {
    position: fixed;
    background: var(--glass-bg);
    backdrop-filter: blur(20px);
    border: 1px solid var(--text-outline);
    border-radius: 8px;
    padding: 4px 0;
    min-width: 200px;
    box-shadow: 0 8px 32px var(--shadow);
    z-index: 1000;
  }

  .context-menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 14px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .context-menu button:hover:not(:disabled) {
    background: var(--bg-container-high);
  }

  .context-menu button:disabled {
    color: var(--text-muted);
    cursor: default;
  }

  .context-menu button .material-symbols-outlined {
    color: var(--text-secondary);
  }

  .context-menu hr {
    border: none;
    border-top: 1px solid var(--text-outline);
    margin: 4px 0;
  }

  /* Ambient glows */
  .ambient-glow {
    position: fixed;
    pointer-events: none;
    z-index: -1;
  }

  .glow-bottom-right {
    bottom: 0;
    right: 0;
    width: 500px;
    height: 500px;
    background: rgba(208, 188, 255, 0.05);
    filter: blur(150px);
  }

  .glow-top-left {
    top: 0;
    left: 0;
    width: 300px;
    height: 300px;
    background: rgba(200, 198, 197, 0.05);
    filter: blur(100px);
  }
</style>
