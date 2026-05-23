<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  export let path: string | null = null;
  export let visible = false;

  interface FileInfo {
    path: string;
    filename: string;
    size: number;
    created: string;
    modified: string;
    is_dir: boolean;
    is_symlink: boolean;
    permissions: string;
    extension: string | null;
  }

  let info: FileInfo | null = null;

  $: if (path && visible) loadInfo(path);
  $: if (!visible) info = null;

  async function loadInfo(p: string) {
    try { info = await invoke<FileInfo>('get_file_info', { path: p }); }
    catch (_) { info = null; }
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let i = 0; let size = bytes;
    while (size >= 1024 && i < units.length - 1) { size /= 1024; i++; }
    return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function formatDate(iso: string): string { return new Date(iso).toLocaleString(); }
</script>

{#if visible && info}
  <div class="info-overlay" on:click|self={() => visible = false}>
    <div class="info-panel glass-panel">
      <div class="info-header">
        <div class="info-header-left">
          <span class="material-symbols-outlined" style="color: var(--accent-primary); font-size: 18px;">info</span>
          <span class="info-title">File Info</span>
        </div>
        <button class="info-close" on:click={() => visible = false}>
          <span class="material-symbols-outlined" style="font-size: 16px;">close</span>
        </button>
      </div>
      <div class="info-body">
        <div class="info-row"><span class="info-label">Name</span><span class="info-value">{info.filename}</span></div>
        <div class="info-row"><span class="info-label">Kind</span><span class="info-value">{info.is_dir ? 'Directory' : info.extension ? `.${info.extension} file` : 'File'}</span></div>
        <div class="info-row"><span class="info-label">Size</span><span class="info-value">{formatSize(info.size)}{info.size > 1024 ? ` (${info.size.toLocaleString()} bytes)` : ''}</span></div>
        <hr />
        <div class="info-row"><span class="info-label">Path</span><span class="info-value path">{info.path}</span></div>
        <div class="info-row"><span class="info-label">Created</span><span class="info-value">{formatDate(info.created)}</span></div>
        <div class="info-row"><span class="info-label">Modified</span><span class="info-value">{formatDate(info.modified)}</span></div>
        <hr />
        <div class="info-row"><span class="info-label">Permissions</span><span class="info-value mono">{info.permissions}</span></div>
        {#if info.is_symlink}
          <div class="info-row"><span class="info-label">Type</span><span class="info-value">Symbolic Link</span></div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .info-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1500;
  }

  .info-panel {
    width: 400px;
    border: 1px solid var(--text-outline);
    border-radius: 12px;
    box-shadow: 0 8px 32px var(--shadow);
    overflow: hidden;
  }

  .info-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid var(--text-outline);
  }

  .info-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .info-title {
    font-family: var(--font-display);
    font-weight: 500;
    color: var(--text-primary);
    font-size: 15px;
  }

  .info-close {
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
  }

  .info-close:hover {
    color: var(--text-primary);
    background: var(--bg-container-high);
  }

  .info-body {
    padding: 14px 16px;
  }

  .info-body hr {
    border: none;
    border-top: 1px solid var(--text-outline);
    margin: 10px 0;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 5px 0;
    font-size: 12px;
  }

  .info-label {
    color: var(--text-muted);
    flex-shrink: 0;
    min-width: 80px;
    font-weight: 500;
  }

  .info-value {
    color: var(--text-primary);
    text-align: right;
    word-break: break-all;
  }

  .info-value.path {
    font-size: 11px;
    font-family: var(--font-mono);
  }

  .info-value.mono {
    font-family: var(--font-mono);
  }
</style>
