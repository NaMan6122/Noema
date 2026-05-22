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
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', id);
    }
  }

  function handleDragOver(e: DragEvent, id: string) {
    e.preventDefault();
    dragOverTabId = id;
  }

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
    dragTabId = null;
    dragOverTabId = null;
  }

  function handleDragEnd() {
    dragTabId = null;
    dragOverTabId = null;
  }
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
        >×</button>
      {/if}
    </div>
  {/each}
  <button class="tab-new" on:click={onNew} title="New tab (Cmd+T)">+</button>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: stretch;
    background: var(--bg-crust);
    border-bottom: 1px solid var(--bg-surface0);
    overflow-x: auto;
    min-height: 32px;
    gap: 1px;
    padding: 0 4px;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    background: var(--bg-mantle);
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
    border-radius: 6px 6px 0 0;
    margin-top: 4px;
    min-width: 80px;
    max-width: 200px;
    user-select: none;
    transition: background 0.1s;
  }

  .tab:hover {
    background: var(--bg-base);
    color: var(--text-subtext);
  }

  .tab.active {
    background: var(--bg-base);
    color: var(--text-primary);
    border-bottom: 2px solid var(--accent-blue);
  }

  .tab.drag-over {
    background: var(--bg-surface0);
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
    padding: 0 2px;
    font-size: 14px;
    line-height: 1;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .tab-close:hover {
    background: var(--bg-surface1);
    color: var(--accent-red);
  }

  .tab-new {
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px 10px;
    font-size: 16px;
    margin-top: 4px;
    border-radius: 6px 6px 0 0;
    flex-shrink: 0;
  }

  .tab-new:hover {
    background: var(--bg-base);
    color: var(--text-primary);
  }
</style>
