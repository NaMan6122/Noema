<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, tick } from 'svelte';

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

  export let entries: FileEntry[];
  export let selectedPaths: Set<string>;
  export let sortField: string;
  export let sortDirection: string;
  export let viewMode: 'list' | 'grid' = 'list';
  export let renamingPath: string;
  export let renameValue: string;
  export let dragOverPath: string;

  export let onSelect: (entry: FileEntry, index: number, event: MouseEvent) => void;
  export let onOpen: (entry: FileEntry) => void;
  export let onContextMenu: (event: MouseEvent, entry?: FileEntry) => void;
  export let onDragStart: (event: DragEvent, entry: FileEntry) => void;
  export let onDragOver: (event: DragEvent, entry: FileEntry) => void;
  export let onDragLeave: () => void;
  export let onDrop: (event: DragEvent, entry: FileEntry) => void;
  export let onToggleSort: (field: string) => void;
  export let onCommitRename: () => void;
  export let onCancelRename: () => void;

  const ROW_HEIGHT = 28;
  const BUFFER = 20;

  let container: HTMLDivElement;
  let scrollTop = 0;
  let containerHeight = 0;
  let focusIndex = -1;
  let typeAheadBuffer = '';
  let typeAheadTimer: ReturnType<typeof setTimeout> | null = null;
  let thumbnails: Record<string, string> = {};
  const IMAGE_EXTS = ['jpg', 'jpeg', 'png', 'gif', 'webp'];

  function isImage(entry: FileEntry): boolean {
    return !entry.is_dir && !!entry.extension && IMAGE_EXTS.includes(entry.extension.toLowerCase());
  }

  async function loadThumbnail(entry: FileEntry) {
    if (thumbnails[entry.path] !== undefined) return;
    thumbnails[entry.path] = '';
    try {
      const dataUri = await invoke<string>('get_thumbnail', { path: entry.path });
      thumbnails = { ...thumbnails, [entry.path]: dataUri };
    } catch (_) {}
  }

  $: if (viewMode === 'grid') { entries.filter(isImage).forEach(loadThumbnail); }

  $: totalHeight = entries.length * ROW_HEIGHT;
  $: startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER);
  $: endIndex = Math.min(entries.length, Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + BUFFER);
  $: visibleEntries = entries.slice(startIndex, endIndex);
  $: offsetY = startIndex * ROW_HEIGHT;
  $: if (entries) focusIndex = entries.length > 0 ? 0 : -1;

  function handleScroll() { scrollTop = container.scrollTop; }

  onMount(() => {
    if (container) {
      containerHeight = container.clientHeight;
      const ro = new ResizeObserver(() => { containerHeight = container.clientHeight; });
      ro.observe(container);
      return () => ro.disconnect();
    }
  });

  function scrollToIndex(idx: number) {
    if (!container) return;
    const rowTop = idx * ROW_HEIGHT;
    const rowBottom = rowTop + ROW_HEIGHT;
    const viewTop = container.scrollTop;
    const viewBottom = viewTop + containerHeight;
    if (rowTop < viewTop) { container.scrollTop = rowTop; }
    else if (rowBottom > viewBottom) { container.scrollTop = rowBottom - containerHeight; }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (renamingPath || entries.length === 0) return;
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault(); focusIndex = Math.min(entries.length - 1, focusIndex + 1); scrollToIndex(focusIndex);
        if (!e.shiftKey) onSelect(entries[focusIndex], focusIndex, e as unknown as MouseEvent);
        break;
      case 'ArrowUp':
        e.preventDefault(); focusIndex = Math.max(0, focusIndex - 1); scrollToIndex(focusIndex);
        if (!e.shiftKey) onSelect(entries[focusIndex], focusIndex, e as unknown as MouseEvent);
        break;
      case 'Enter':
        e.preventDefault(); if (focusIndex >= 0 && focusIndex < entries.length) onOpen(entries[focusIndex]);
        break;
      case 'Home':
        e.preventDefault(); focusIndex = 0; scrollToIndex(focusIndex);
        onSelect(entries[focusIndex], focusIndex, e as unknown as MouseEvent);
        break;
      case 'End':
        e.preventDefault(); focusIndex = entries.length - 1; scrollToIndex(focusIndex);
        onSelect(entries[focusIndex], focusIndex, e as unknown as MouseEvent);
        break;
      default:
        if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.altKey) { e.preventDefault(); handleTypeAhead(e.key); }
        break;
    }
  }

  function handleTypeAhead(char: string) {
    typeAheadBuffer += char.toLowerCase();
    if (typeAheadTimer) clearTimeout(typeAheadTimer);
    typeAheadTimer = setTimeout(() => { typeAheadBuffer = ''; }, 500);
    const match = entries.findIndex(e => e.filename.toLowerCase().startsWith(typeAheadBuffer));
    if (match >= 0) {
      focusIndex = match; scrollToIndex(focusIndex);
      onSelect(entries[focusIndex], focusIndex, { metaKey: false, ctrlKey: false, shiftKey: false } as MouseEvent);
    }
  }

  function sortIndicator(field: string): string {
    if (sortField !== field) return '';
    return sortDirection === 'asc' ? ' ▲' : ' ▼';
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return '—';
    const units = ['B', 'KB', 'MB', 'GB'];
    let i = 0; let size = bytes;
    while (size >= 1024 && i < units.length - 1) { size /= 1024; i++; }
    return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function formatDate(iso: string): string {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }

  function getIcon(entry: FileEntry): string {
    if (entry.is_dir) return 'folder';
    const ext = entry.extension?.toLowerCase();
    if (!ext) return 'description';
    if (['pdf'].includes(ext)) return 'picture_as_pdf';
    if (['doc', 'docx'].includes(ext)) return 'article';
    if (['xls', 'xlsx', 'csv'].includes(ext)) return 'table_chart';
    if (['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp'].includes(ext)) return 'image';
    if (['mp3', 'wav', 'flac', 'aac'].includes(ext)) return 'music_note';
    if (['mp4', 'mov', 'avi', 'mkv'].includes(ext)) return 'movie';
    if (['zip', 'tar', 'gz', 'rar', '7z'].includes(ext)) return 'inventory_2';
    if (['rs', 'py', 'js', 'ts', 'go', 'c', 'cpp', 'java'].includes(ext)) return 'code';
    if (['md', 'txt', 'rtf'].includes(ext)) return 'edit_note';
    return 'description';
  }

  function getIconColor(entry: FileEntry): string {
    if (entry.is_dir) return 'var(--accent-secondary)';
    const ext = entry.extension?.toLowerCase();
    if (!ext) return 'var(--text-secondary)';
    if (['pdf'].includes(ext)) return 'var(--accent-error)';
    if (['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp'].includes(ext)) return 'var(--accent-on-primary-container, #340080)';
    if (['rs', 'py', 'js', 'ts', 'go', 'c', 'cpp', 'java'].includes(ext)) return 'var(--accent-tertiary)';
    if (['md', 'txt', 'rtf'].includes(ext)) return 'var(--accent-tertiary)';
    if (['xls', 'xlsx', 'csv'].includes(ext)) return 'var(--accent-secondary)';
    return 'var(--text-secondary)';
  }
</script>

<div
  class="file-list"
  bind:this={container}
  on:scroll={handleScroll}
  on:contextmenu={(e) => onContextMenu(e)}
  on:keydown={handleKeydown}
  tabindex="-1"
>
  {#if viewMode === 'list'}
    <div class="list-header">
      <div class="col-check"></div>
      <div class="col-icon"></div>
      <div class="col-name" on:click={() => onToggleSort('name')}>Name{sortIndicator('name')}</div>
      <div class="col-modified" on:click={() => onToggleSort('modified')}>Date Modified{sortIndicator('modified')}</div>
      <div class="col-size" on:click={() => onToggleSort('size')}>Size{sortIndicator('size')}</div>
      <div class="col-insight">AI Insight</div>
    </div>

    <div class="virtual-scroller semantic-glow" style="height: {totalHeight}px; position: relative;">
      <div style="position: absolute; top: {offsetY}px; left: 0; right: 0;">
        {#each visibleEntries as entry, vi}
          {@const i = startIndex + vi}
          <div
            class="list-row"
            class:is-dir={entry.is_dir}
            class:selected={selectedPaths.has(entry.path)}
            class:focused={focusIndex === i}
            class:drag-over={dragOverPath === entry.path}
            style="height: {ROW_HEIGHT}px;"
            draggable="true"
            on:dragstart={(e) => onDragStart(e, entry)}
            on:dragover={(e) => onDragOver(e, entry)}
            on:dragleave={onDragLeave}
            on:drop={(e) => onDrop(e, entry)}
            on:click={(e) => { focusIndex = i; onSelect(entry, i, e); }}
            on:dblclick={() => onOpen(entry)}
            on:contextmenu|stopPropagation={(e) => onContextMenu(e, entry)}
          >
            <div class="col-check">
              <input type="checkbox" checked={selectedPaths.has(entry.path)} on:click|stopPropagation />
            </div>
            <div class="col-icon">
              <span class="material-symbols-outlined" style="color: {getIconColor(entry)}">{getIcon(entry)}</span>
            </div>
            <div class="col-name">
              {#if renamingPath === entry.path}
                <input
                  class="rename-input"
                  type="text"
                  bind:value={renameValue}
                  on:blur={onCommitRename}
                  on:keydown={(e) => { if (e.key === 'Enter') onCommitRename(); if (e.key === 'Escape') onCancelRename(); }}
                  autofocus
                />
              {:else}
                <span class="filename">{entry.filename}</span>
              {/if}
            </div>
            <div class="col-modified">{formatDate(entry.modified)}</div>
            <div class="col-size">{entry.is_dir ? '—' : formatSize(entry.size)}</div>
            <div class="col-insight">
              <span class="insight-placeholder">—</span>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="grid-view">
      {#each entries as entry, i}
        <div
          class="grid-cell"
          class:selected={selectedPaths.has(entry.path)}
          class:focused={focusIndex === i}
          class:drag-over={dragOverPath === entry.path}
          draggable="true"
          on:dragstart={(e) => onDragStart(e, entry)}
          on:dragover={(e) => onDragOver(e, entry)}
          on:dragleave={onDragLeave}
          on:drop={(e) => onDrop(e, entry)}
          on:click={(e) => { focusIndex = i; onSelect(entry, i, e); }}
          on:dblclick={() => onOpen(entry)}
          on:contextmenu|stopPropagation={(e) => onContextMenu(e, entry)}
        >
          <div class="grid-icon">
            {#if isImage(entry) && thumbnails[entry.path]}
              <img class="grid-thumb" src={thumbnails[entry.path]} alt={entry.filename} />
            {:else}
              <span class="material-symbols-outlined grid-mat-icon" style="color: {getIconColor(entry)}">{getIcon(entry)}</span>
            {/if}
          </div>
          <div class="grid-name">
            {#if renamingPath === entry.path}
              <input
                class="rename-input"
                type="text"
                bind:value={renameValue}
                on:blur={onCommitRename}
                on:keydown={(e) => { if (e.key === 'Enter') onCommitRename(); if (e.key === 'Escape') onCancelRename(); }}
                autofocus
              />
            {:else}
              {entry.filename}
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if entries.length === 0}
    <div class="empty">
      <span class="material-symbols-outlined" style="font-size: 48px; color: var(--text-outline); margin-bottom: 12px;">folder_open</span>
      <span>Empty directory</span>
    </div>
  {/if}
</div>

<style>
  .file-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .file-list:focus {
    outline: none;
  }

  /* List header */
  .list-header {
    display: grid;
    grid-template-columns: 32px 32px 1fr 130px 90px 180px;
    padding: 0 16px;
    height: 28px;
    align-items: center;
    background: var(--bg-container-low);
    border-bottom: 1px solid var(--text-outline);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 400;
    color: var(--text-outline-full, var(--text-muted));
    text-transform: uppercase;
    letter-spacing: -0.02em;
    position: sticky;
    top: 0;
    cursor: pointer;
    user-select: none;
    z-index: 1;
  }

  .list-header .col-check {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .list-header .col-check input {
    accent-color: var(--accent-primary);
  }

  /* List row */
  .list-row {
    display: grid;
    grid-template-columns: 32px 32px 1fr 130px 90px 180px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border-subtle, rgba(73, 68, 84, 0.3));
    cursor: default;
    align-items: center;
    box-sizing: border-box;
    transition: background 0.1s;
  }

  .list-row:hover {
    background: var(--bg-container);
  }

  .list-row:hover .filename {
    color: var(--accent-primary);
  }

  .list-row.selected {
    background: var(--bg-container-high);
  }

  .list-row.focused {
    outline: 1px solid var(--accent-primary);
    outline-offset: -1px;
  }

  .list-row.drag-over {
    background: rgba(208, 188, 255, 0.08);
    outline: 1px dashed var(--accent-primary);
    outline-offset: -1px;
  }

  .list-row.is-dir .filename {
    font-weight: 500;
  }

  .col-check {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .col-check input[type="checkbox"] {
    accent-color: var(--accent-primary);
    width: 14px;
    height: 14px;
    cursor: pointer;
  }

  .col-icon {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .col-icon .material-symbols-outlined {
    font-size: 18px;
  }

  .col-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding-right: 8px;
    font-size: 13px;
  }

  .filename {
    color: var(--text-primary);
    transition: color 0.15s;
  }

  .col-size {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 12px;
    text-align: right;
  }

  .col-modified {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .col-insight {
    font-size: 11px;
  }

  .insight-placeholder {
    color: var(--text-outline);
  }

  .empty {
    padding: 60px 40px;
    text-align: center;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .rename-input {
    width: 100%;
    padding: 2px 6px;
    border: 1px solid var(--accent-primary);
    border-radius: 4px;
    background: var(--bg-container-lowest);
    color: var(--text-primary);
    font-size: 13px;
    outline: none;
    box-shadow: 0 0 0 2px rgba(208, 188, 255, 0.15);
  }

  /* Grid view */
  .grid-view {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 4px;
    padding: 12px;
  }

  .grid-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 8px 8px;
    border-radius: 8px;
    cursor: default;
    user-select: none;
    transition: background 0.1s;
  }

  .grid-cell:hover {
    background: var(--bg-container);
  }

  .grid-cell.selected {
    background: var(--bg-container-high);
  }

  .grid-cell.focused {
    outline: 1px solid var(--accent-primary);
    outline-offset: -1px;
  }

  .grid-cell.drag-over {
    background: rgba(208, 188, 255, 0.08);
    outline: 1px dashed var(--accent-primary);
  }

  .grid-icon {
    width: 64px;
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 6px;
  }

  .grid-mat-icon {
    font-size: 40px !important;
  }

  .grid-thumb {
    max-width: 64px;
    max-height: 64px;
    border-radius: 4px;
    object-fit: cover;
  }

  .grid-name {
    font-size: 11px;
    text-align: center;
    word-break: break-all;
    line-height: 1.3;
    max-height: 2.6em;
    overflow: hidden;
    width: 100%;
    color: var(--text-primary);
  }
</style>
