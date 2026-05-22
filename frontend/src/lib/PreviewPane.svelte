<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

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

  export let entry: FileEntry | null = null;
  export let visible = false;

  let content = '';
  let highlightedHtml = '';
  let imageDataUri = '';
  let loading = false;

  const IMAGE_EXTS = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg'];
  const TEXT_EXTS = ['txt', 'md', 'rtf', 'csv', 'json', 'xml', 'yaml', 'yml', 'toml', 'ini', 'cfg', 'log',
    'rs', 'py', 'js', 'ts', 'jsx', 'tsx', 'go', 'c', 'cpp', 'h', 'hpp', 'java', 'rb', 'php',
    'sh', 'bash', 'zsh', 'fish', 'html', 'css', 'scss', 'less', 'sql', 'svelte', 'vue'];

  const CODE_EXTS = ['rs', 'py', 'js', 'ts', 'jsx', 'tsx', 'go', 'c', 'cpp', 'h', 'hpp', 'java', 'rb', 'php',
    'sh', 'bash', 'zsh', 'html', 'css', 'scss', 'less', 'sql', 'svelte', 'vue', 'json', 'yaml', 'yml', 'toml'];

  $: if (entry && visible) loadPreview(entry);

  function isImage(e: FileEntry): boolean {
    return !!e.extension && IMAGE_EXTS.includes(e.extension.toLowerCase());
  }

  function isText(e: FileEntry): boolean {
    return !!e.extension && TEXT_EXTS.includes(e.extension.toLowerCase());
  }

  function isCode(e: FileEntry): boolean {
    return !!e.extension && CODE_EXTS.includes(e.extension.toLowerCase());
  }

  async function loadPreview(e: FileEntry) {
    content = '';
    highlightedHtml = '';
    imageDataUri = '';
    if (e.is_dir) return;
    loading = true;

    try {
      if (isImage(e)) {
        imageDataUri = await invoke<string>('get_thumbnail', { path: e.path });
      } else if (isCode(e)) {
        highlightedHtml = await invoke<string>('highlight_code', { path: e.path });
      } else if (isText(e)) {
        content = await invoke<string>('read_file_preview', { path: e.path });
      }
    } catch (_) {}
    loading = false;
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB'];
    let i = 0;
    let size = bytes;
    while (size >= 1024 && i < units.length - 1) { size /= 1024; i++; }
    return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleString();
  }
</script>

{#if visible && entry}
  <div class="preview-pane">
    <div class="preview-header">
      <span class="preview-filename">{entry.filename}</span>
      <span class="preview-meta">{formatSize(entry.size)}</span>
    </div>

    <div class="preview-body">
      {#if loading}
        <div class="preview-loading">Loading...</div>
      {:else if entry.is_dir}
        <div class="preview-placeholder">Directory</div>
      {:else if imageDataUri}
        <img class="preview-image" src={imageDataUri} alt={entry.filename} />
      {:else if highlightedHtml}
        <div class="preview-code">{@html highlightedHtml}</div>
      {:else if content}
        <pre class="preview-text">{content}</pre>
      {:else}
        <div class="preview-placeholder">No preview available</div>
      {/if}
    </div>

    <div class="preview-footer">
      <div>Modified: {formatDate(entry.modified)}</div>
      <div>Created: {formatDate(entry.created)}</div>
    </div>
  </div>
{/if}

<style>
  .preview-pane {
    width: 40%;
    min-width: 250px;
    max-width: 500px;
    border-left: 1px solid #313244;
    background: #181825;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    border-bottom: 1px solid #313244;
  }

  .preview-filename {
    font-weight: 600;
    color: #cdd6f4;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-meta {
    color: #6c7086;
    font-size: 11px;
    flex-shrink: 0;
    margin-left: 8px;
  }

  .preview-body {
    flex: 1;
    overflow: auto;
    padding: 12px;
    display: flex;
    align-items: flex-start;
    justify-content: center;
  }

  .preview-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 4px;
  }

  .preview-text {
    width: 100%;
    margin: 0;
    font-family: 'SF Mono', Monaco, 'Cascadia Code', monospace;
    font-size: 12px;
    line-height: 1.5;
    color: #cdd6f4;
    white-space: pre-wrap;
    word-break: break-all;
    tab-size: 4;
  }

  .preview-code {
    width: 100%;
    overflow: auto;
    font-family: 'SF Mono', Monaco, 'Cascadia Code', monospace;
    font-size: 12px;
    line-height: 1.5;
    tab-size: 4;
  }

  .preview-code :global(pre) {
    margin: 0;
    padding: 8px;
    border-radius: 4px;
  }

  .preview-placeholder, .preview-loading {
    color: #6c7086;
    font-size: 13px;
    padding: 40px;
    text-align: center;
  }

  .preview-footer {
    padding: 8px 14px;
    border-top: 1px solid #313244;
    font-size: 11px;
    color: #6c7086;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
</style>
