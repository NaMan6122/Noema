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
    try {
      info = await invoke<FileInfo>('get_file_info', { path: p });
    } catch (_) {
      info = null;
    }
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let i = 0;
    let size = bytes;
    while (size >= 1024 && i < units.length - 1) { size /= 1024; i++; }
    return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleString();
  }
</script>

{#if visible && info}
  <div class="info-panel-overlay" on:click|self={() => visible = false}>
    <div class="info-panel">
      <div class="info-header">
        <span class="info-title">Info</span>
        <button class="info-close" on:click={() => visible = false}>×</button>
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
  .info-panel-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1500;
  }

  .info-panel {
    width: 380px;
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }

  .info-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid #313244;
  }

  .info-title {
    font-weight: 600;
    color: #cdd6f4;
    font-size: 14px;
  }

  .info-close {
    border: none;
    background: none;
    color: #6c7086;
    font-size: 18px;
    cursor: pointer;
    padding: 0 4px;
  }

  .info-close:hover {
    color: #cdd6f4;
  }

  .info-body {
    padding: 12px 16px;
  }

  .info-body hr {
    border: none;
    border-top: 1px solid #313244;
    margin: 8px 0;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 4px 0;
    font-size: 12px;
  }

  .info-label {
    color: #6c7086;
    flex-shrink: 0;
    min-width: 80px;
  }

  .info-value {
    color: #cdd6f4;
    text-align: right;
    word-break: break-all;
  }

  .info-value.path {
    font-size: 11px;
    font-family: 'SF Mono', Monaco, monospace;
  }

  .info-value.mono {
    font-family: 'SF Mono', Monaco, monospace;
  }
</style>
