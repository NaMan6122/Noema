<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface FavoriteEntry {
    name: string;
    path: string;
    kind: string;
  }

  export let currentPath: string;
  export let onNavigate: (path: string) => void;

  let favorites: FavoriteEntry[] = [];
  let volumes: FavoriteEntry[] = [];
  let recents: FavoriteEntry[] = [];

  const favoriteIcons: Record<string, string> = {
    Home: '🏠',
    Desktop: '🖥️',
    Documents: '📄',
    Downloads: '📥',
    Pictures: '🖼️',
    Music: '🎵',
    Videos: '🎬',
  };

  onMount(async () => {
    try {
      const all = await invoke<FavoriteEntry[]>('get_favorites');
      favorites = all.filter(e => e.kind === 'favorite');
      volumes = all.filter(e => e.kind === 'volume');
    } catch (e) {
      console.error('Failed to load favorites:', e);
    }
    try {
      recents = await invoke<FavoriteEntry[]>('get_recent_files');
    } catch (_) {}
  });

  function isActive(path: string): boolean {
    return currentPath === path || currentPath.startsWith(path + '/');
  }
</script>

<nav class="sidebar">
  <section>
    <h3>Favorites</h3>
    {#each favorites as fav}
      <button
        class="sidebar-item"
        class:active={isActive(fav.path)}
        on:click={() => onNavigate(fav.path)}
        title={fav.path}
      >
        <span class="sidebar-icon">{favoriteIcons[fav.name] || '📁'}</span>
        <span class="sidebar-label">{fav.name}</span>
      </button>
    {/each}
  </section>

  {#if volumes.length > 0}
    <section>
      <h3>Volumes</h3>
      {#each volumes as vol}
        <button
          class="sidebar-item"
          class:active={isActive(vol.path)}
          on:click={() => onNavigate(vol.path)}
          title={vol.path}
        >
          <span class="sidebar-icon">💾</span>
          <span class="sidebar-label">{vol.name}</span>
        </button>
      {/each}
    </section>
  {/if}

  {#if recents.length > 0}
    <section>
      <h3>Recents</h3>
      {#each recents as rec}
        <button
          class="sidebar-item"
          class:active={isActive(rec.path)}
          on:click={() => onNavigate(rec.path)}
          title={rec.path}
        >
          <span class="sidebar-icon">🕐</span>
          <span class="sidebar-label">{rec.name}</span>
        </button>
      {/each}
    </section>
  {/if}
</nav>

<style>
  .sidebar {
    width: 200px;
    min-width: 200px;
    background: var(--bg-mantle);
    border-right: 1px solid var(--bg-surface0);
    overflow-y: auto;
    padding: 8px 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  section {
    padding: 0 8px;
  }

  h3 {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin: 8px 4px 4px;
    font-weight: 600;
  }

  .sidebar-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--text-primary);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }

  .sidebar-item:hover {
    background: var(--bg-surface0);
  }

  .sidebar-item.active {
    background: var(--bg-surface1);
    color: var(--accent-blue);
  }

  .sidebar-icon {
    font-size: 14px;
    width: 20px;
    text-align: center;
    flex-shrink: 0;
  }

  .sidebar-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
