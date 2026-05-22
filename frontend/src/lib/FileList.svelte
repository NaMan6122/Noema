<script lang="ts">
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

  $: totalHeight = entries.length * ROW_HEIGHT;
  $: startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER);
  $: endIndex = Math.min(entries.length, Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + BUFFER);
  $: visibleEntries = entries.slice(startIndex, endIndex);
  $: offsetY = startIndex * ROW_HEIGHT;

  // Reset focus when entries change
  $: if (entries) focusIndex = entries.length > 0 ? 0 : -1;

  function handleScroll() {
    scrollTop = container.scrollTop;
  }

  onMount(() => {
    if (container) {
      containerHeight = container.clientHeight;
      const ro = new ResizeObserver(() => {
        containerHeight = container.clientHeight;
      });
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

    if (rowTop < viewTop) {
      container.scrollTop = rowTop;
    } else if (rowBottom > viewBottom) {
      container.scrollTop = rowBottom - containerHeight;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (renamingPath || entries.length === 0) return;

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        focusIndex = Math.min(entries.length - 1, focusIndex + 1);
        scrollToIndex(focusIndex);
        if (!e.shiftKey) {
          onSelect(entries[focusIndex], focusIndex, e as unknown as MouseEvent);
        }
        break;
      case 'ArrowUp':
        e.preventDefault();
        focusIndex = Math.max(0, focusIndex - 1);
        scrollToIndex(focusIndex);
        if (!e.shiftKey) {
          onSelect(entries[focusIndex], focusIndex, e as unknown as MouseEvent);
        }
        break;
      case 'Enter':
        e.preventDefault();
        if (focusIndex >= 0 && focusIndex < entries.length) {
          onOpen(entries[focusIndex]);
        }
        break;
      case ' ':
        e.preventDefault();
        if (focusIndex >= 0 && focusIndex < entries.length) {
          const entry = entries[focusIndex];
          const fakeEvent = { metaKey: true, ctrlKey: false, shiftKey: false } as MouseEvent;
          onSelect(entry, focusIndex, fakeEvent);
        }
        break;
      case 'Home':
        e.preventDefault();
        focusIndex = 0;
        scrollToIndex(focusIndex);
        onSelect(entries[focusIndex], focusIndex, e as unknown as MouseEvent);
        break;
      case 'End':
        e.preventDefault();
        focusIndex = entries.length - 1;
        scrollToIndex(focusIndex);
        onSelect(entries[focusIndex], focusIndex, e as unknown as MouseEvent);
        break;
      default:
        if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.altKey) {
          e.preventDefault();
          handleTypeAhead(e.key);
        }
        break;
    }
  }

  function handleTypeAhead(char: string) {
    typeAheadBuffer += char.toLowerCase();
    if (typeAheadTimer) clearTimeout(typeAheadTimer);
    typeAheadTimer = setTimeout(() => { typeAheadBuffer = ''; }, 500);

    const match = entries.findIndex(e =>
      e.filename.toLowerCase().startsWith(typeAheadBuffer)
    );
    if (match >= 0) {
      focusIndex = match;
      scrollToIndex(focusIndex);
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
      <div class="col-icon"></div>
      <div class="col-name" on:click={() => onToggleSort('name')}>
        Name{sortIndicator('name')}
      </div>
      <div class="col-size" on:click={() => onToggleSort('size')}>
        Size{sortIndicator('size')}
      </div>
      <div class="col-modified" on:click={() => onToggleSort('modified')}>
        Modified{sortIndicator('modified')}
      </div>
    </div>

    <div class="virtual-scroller" style="height: {totalHeight}px; position: relative;">
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
            <div class="col-icon">{getIcon(entry)}</div>
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
                {entry.filename}
              {/if}
            </div>
            <div class="col-size">{entry.is_dir ? '—' : formatSize(entry.size)}</div>
            <div class="col-modified">{formatDate(entry.modified)}</div>
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
          <div class="grid-icon">{getIcon(entry)}</div>
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
    <div class="empty">Empty directory</div>
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
    z-index: 1;
  }

  .list-row {
    display: grid;
    grid-template-columns: 32px 1fr 90px 120px;
    padding: 0 12px;
    border-bottom: 1px solid #31324420;
    cursor: default;
    align-items: center;
    box-sizing: border-box;
  }

  .list-row:hover {
    background: #313244;
  }

  .list-row.selected {
    background: #45475a;
  }

  .list-row.focused {
    outline: 1px solid #89b4fa;
    outline-offset: -1px;
  }

  .list-row.drag-over {
    background: #89b4fa20;
    outline: 1px dashed #89b4fa;
    outline-offset: -1px;
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

  .empty {
    padding: 40px;
    text-align: center;
    color: #6c7086;
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
    border-radius: 6px;
    cursor: default;
    user-select: none;
  }

  .grid-cell:hover {
    background: #313244;
  }

  .grid-cell.selected {
    background: #45475a;
  }

  .grid-cell.focused {
    outline: 1px solid #89b4fa;
    outline-offset: -1px;
  }

  .grid-cell.drag-over {
    background: #89b4fa20;
    outline: 1px dashed #89b4fa;
  }

  .grid-icon {
    font-size: 40px;
    margin-bottom: 6px;
  }

  .grid-name {
    font-size: 11px;
    text-align: center;
    word-break: break-all;
    line-height: 1.3;
    max-height: 2.6em;
    overflow: hidden;
    width: 100%;
  }
</style>
