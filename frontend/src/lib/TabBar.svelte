<script lang="ts">
  export let tabs: Array<{ id: string; path: string; title: string }>;
  export let activeTabId: string;
  export let onSelect: (id: string) => void;
  export let onClose: (id: string) => void;
  export let onNew: () => void;

  let dragTabId: string | null = null;
  let dragOverTabId: string | null = null;

  function handleDragStart(e: DragEvent, id: string) {
    dragTabId = id;
    if (e.dataTransfer) { e.dataTransfer.effectAllowed = 'move'; e.dataTransfer.setData('text/plain', id); }
  }

  function handleDragOver(e: DragEvent, id: string) { e.preventDefault(); dragOverTabId = id; }

  function handleDrop(e: DragEvent, targetId: string) {
    e.preventDefault();
    if (dragTabId && dragTabId !== targetId) {
      const fromIdx = tabs.findIndex(t => t.id === dragTabId);
      const toIdx = tabs.findIndex(t => t.id === targetId);
      if (fromIdx >= 0 && toIdx >= 0) {
        const reordered = [...tabs];
        const [moved] = reordered.splice(fromIdx, 1);
        reordered.splice(toIdx, 0, moved);
        tabs = reordered;
      }
    }
    dragTabId = null; dragOverTabId = null;
  }

  function handleDragEnd() { dragTabId = null; dragOverTabId = null; }
</script>

<div class="tab-bar">
  {#each tabs as tab (tab.id)}
    <div
      class="tab"
      class:active={tab.id === activeTabId}
      class:drag-over={dragOverTabId === tab.id}
      draggable="true"
      on:click={() => onSelect(tab.id)}
      on:dragstart={(e) => handleDragStart(e, tab.id)}
      on:dragover={(e) => handleDragOver(e, tab.id)}
      on:drop={(e) => handleDrop(e, tab.id)}
      on:dragend={handleDragEnd}
      title={tab.path}
    >
      <span class="tab-title">{tab.title}</span>
      {#if tabs.length > 1}
        <button
          class="tab-close"
          on:click|stopPropagation={() => onClose(tab.id)}
          title="Close tab"
        >
          <span class="material-symbols-outlined" style="font-size: 14px;">close</span>
        </button>
      {/if}
    </div>
  {/each}
  <button class="tab-new" on:click={onNew} title="New tab (⌘T)">
    <span class="material-symbols-outlined" style="font-size: 16px;">add</span>
  </button>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: stretch;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--text-outline);
    overflow-x: auto;
    min-height: 36px;
    gap: 0;
    padding: 0;
    flex-shrink: 0;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 16px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    min-width: 80px;
    max-width: 200px;
    user-select: none;
    transition: color 0.15s;
    border-bottom: 2px solid transparent;
    position: relative;
  }

  .tab:hover {
    color: var(--text-primary);
    background: var(--bg-container);
  }

  .tab.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
  }

  .tab.drag-over {
    background: var(--bg-container-high);
  }

  .tab-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .tab-close {
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px;
    border-radius: 4px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .tab-close:hover {
    background: var(--bg-container-high);
    color: var(--accent-error);
  }

  .tab-new {
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 12px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .tab-new:hover {
    color: var(--text-primary);
  }
</style>
