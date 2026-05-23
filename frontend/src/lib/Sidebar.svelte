<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { themeChoice, setTheme } from './themeStore';
  import type { ThemeChoice } from './themeStore';

  interface FavoriteEntry {
    name: string;
    path: string;
    kind: string;
  }

  interface SmartFolder {
    id: number;
    name: string;
    icon: string | null;
  }

  export let currentPath: string;
  export let onNavigate: (path: string) => void;
  export let onSmartFolder: ((id: number) => void) | null = null;

  let favorites: FavoriteEntry[] = [];
  let volumes: FavoriteEntry[] = [];
  let recents: FavoriteEntry[] = [];
  let smartFolders: SmartFolder[] = [];

  const favoriteIcons: Record<string, string> = {
    Home: 'home',
    Desktop: 'desktop_windows',
    Documents: 'description',
    Downloads: 'download',
    Pictures: 'image',
    Music: 'music_note',
    Videos: 'movie',
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
    try {
      smartFolders = await invoke<SmartFolder[]>('list_smart_folders');
    } catch (_) {}
  });

  function isActive(path: string): boolean {
    return currentPath === path || currentPath.startsWith(path + '/');
  }
</script>

<aside class="sidebar">
  <div class="brand">
    <div class="brand-logo">
      <span class="material-symbols-outlined" style="font-variation-settings: 'FILL' 1; color: var(--accent-on-primary); font-size: 20px;">dataset</span>
    </div>
    <div class="brand-text">
      <h1 class="brand-name">Noema</h1>
      <p class="brand-sub">Semantic Intelligence</p>
    </div>
  </div>

  <nav class="nav-sections">
    <section>
      <h3 class="section-label">Navigation</h3>
      {#each favorites as fav}
        <button
          class="nav-item"
          class:active={isActive(fav.path)}
          on:click={() => onNavigate(fav.path)}
          title={fav.path}
        >
          <span class="material-symbols-outlined">{favoriteIcons[fav.name] || 'folder'}</span>
          <span class="nav-label">{fav.name}</span>
        </button>
      {/each}
    </section>

    {#if volumes.length > 0}
      <section>
        <h3 class="section-label">Volumes</h3>
        {#each volumes as vol}
          <button
            class="nav-item"
            class:active={isActive(vol.path)}
            on:click={() => onNavigate(vol.path)}
            title={vol.path}
          >
            <span class="material-symbols-outlined">hard_drive</span>
            <span class="nav-label">{vol.name}</span>
          </button>
        {/each}
      </section>
    {/if}

    {#if recents.length > 0}
      <section>
        <h3 class="section-label">Recents</h3>
        {#each recents as rec}
          <button
            class="nav-item"
            class:active={isActive(rec.path)}
            on:click={() => onNavigate(rec.path)}
            title={rec.path}
          >
            <span class="material-symbols-outlined">schedule</span>
            <span class="nav-label">{rec.name}</span>
          </button>
        {/each}
      </section>
    {/if}

    {#if smartFolders.length > 0}
      <section>
        <h3 class="section-label">Smart Folders</h3>
        {#each smartFolders as sf}
          <button
            class="nav-item"
            on:click={() => onSmartFolder && onSmartFolder(sf.id)}
            title={sf.name}
          >
            <span class="material-symbols-outlined">{sf.icon || 'auto_awesome_mosaic'}</span>
            <span class="nav-label">{sf.name}</span>
          </button>
        {/each}
      </section>
    {/if}

    <section class="system-section">
      <h3 class="section-label">System</h3>
      <button class="nav-item" on:click={() => {
        const order: ThemeChoice[] = ['system', 'light', 'dark'];
        const next = order[(order.indexOf($themeChoice) + 1) % 3];
        setTheme(next);
      }}>
        <span class="material-symbols-outlined">
          {$themeChoice === 'light' ? 'light_mode' : $themeChoice === 'dark' ? 'dark_mode' : 'brightness_auto'}
        </span>
        <span class="nav-label">{$themeChoice === 'system' ? 'System Theme' : $themeChoice === 'light' ? 'Light Mode' : 'Dark Mode'}</span>
      </button>
      <button class="nav-item">
        <span class="material-symbols-outlined">settings</span>
        <span class="nav-label">Settings</span>
      </button>
    </section>
  </nav>
</aside>

<style>
  .sidebar {
    width: 240px;
    min-width: 240px;
    background: var(--bg-container-low);
    border-right: 1px solid var(--text-outline);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Brand */
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
  }

  .brand-logo {
    width: 32px;
    height: 32px;
    border-radius: 4px;
    background: var(--accent-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .brand-name {
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    line-height: 1.2;
    letter-spacing: -0.01em;
  }

  .brand-sub {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-secondary);
    margin: 0;
    font-weight: 500;
  }

  /* Navigation */
  .nav-sections {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  section {
    margin-bottom: 4px;
  }

  .section-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-outline-full, var(--text-muted));
    margin: 20px 12px 8px;
    padding: 0;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 8px 16px;
    border: none;
    border-left: 2px solid transparent;
    border-radius: 0;
    background: none;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
  }

  .nav-item:hover {
    background: var(--bg-container-high);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--accent-secondary-container);
    color: var(--accent-on-secondary-container);
    border-left-color: var(--accent-primary);
  }

  .nav-item .material-symbols-outlined {
    font-size: 20px;
  }

  .nav-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .system-section {
    margin-top: auto;
  }
</style>
