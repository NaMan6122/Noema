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

  interface AiContext {
    version: number;
    summary: string;
    entities: { text: string; entity_type: string }[];
    tags: string[];
    user_edited: boolean;
    generated_at: string;
    model_id: string;
  }

  let info: FileInfo | null = null;
  let aiContext: AiContext | null = null;
  let aiLoading = false;
  let aiError: string | null = null;
  let suggestedName: string | null = null;

  $: if (path && visible) { loadInfo(path); loadAiContext(path); }
  $: if (!visible) { info = null; aiContext = null; aiError = null; suggestedName = null; }

  async function loadInfo(p: string) {
    try { info = await invoke<FileInfo>('get_file_info', { path: p }); }
    catch (_) { info = null; }
  }

  async function loadAiContext(p: string) {
    aiError = null;
    try {
      const result = await invoke<AiContext | null>('get_file_context', { path: p });
      aiContext = result;
    } catch (_) {
      aiContext = null;
    }
  }

  async function analyzeFile() {
    if (!path) return;
    aiLoading = true;
    aiError = null;
    try {
      const result = await invoke<{ summary: string; entities: any[]; suggested_tags: string[]; key_phrases: string[] }>(
        'generate_file_context', { path }
      );
      await loadAiContext(path);
    } catch (e: any) {
      aiError = e?.toString() || 'Analysis failed';
    } finally {
      aiLoading = false;
    }
  }

  async function acceptTags(tags: string[]) {
    if (!path) return;
    try {
      await invoke('apply_ai_tags', { path, tags });
    } catch (_) {}
  }

  async function suggestRename() {
    if (!path) return;
    try {
      suggestedName = await invoke<string>('suggest_filename', { path });
    } catch (_) {
      suggestedName = null;
    }
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

        {#if !info.is_dir}
          <hr />
          <div class="ai-section">
            <div class="ai-header">
              <span class="material-symbols-outlined" style="font-size: 16px; color: var(--accent-primary);">auto_awesome</span>
              <span class="ai-title">AI Context</span>
              <button class="ai-btn" on:click={analyzeFile} disabled={aiLoading}>
                {aiLoading ? 'Analyzing...' : aiContext ? 'Re-analyze' : 'Analyze'}
              </button>
            </div>

            {#if aiError}
              <div class="ai-error">{aiError}</div>
            {/if}

            {#if aiContext}
              <div class="ai-content">
                <div class="ai-summary">{aiContext.summary}</div>

                {#if aiContext.tags.length > 0}
                  <div class="ai-tags-section">
                    <span class="info-label">Tags</span>
                    <div class="ai-tags">
                      {#each aiContext.tags as tag}
                        <span class="ai-tag">{tag}</span>
                      {/each}
                    </div>
                    <button class="ai-btn-sm" on:click={() => acceptTags(aiContext?.tags || [])}>Accept all</button>
                  </div>
                {/if}

                {#if aiContext.entities.length > 0}
                  <div class="ai-entities-section">
                    <span class="info-label">Entities</span>
                    <div class="ai-entities">
                      {#each aiContext.entities as entity}
                        <span class="ai-entity" title={entity.entity_type}>{entity.text}</span>
                      {/each}
                    </div>
                  </div>
                {/if}

                <div class="ai-meta">
                  v{aiContext.version} {aiContext.user_edited ? '(edited)' : ''} · {aiContext.model_id}
                </div>
              </div>
            {:else if !aiLoading}
              <div class="ai-empty">No AI context yet. Click Analyze to generate.</div>
            {/if}

            <div class="ai-actions">
              <button class="ai-btn-sm" on:click={suggestRename}>Suggest rename</button>
              {#if suggestedName}
                <span class="ai-suggestion">{suggestedName}</span>
              {/if}
            </div>
          </div>
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
    width: 420px;
    max-height: 80vh;
    overflow-y: auto;
    border: 1px solid var(--text-outline);
    border-radius: 12px;
    box-shadow: 0 8px 32px var(--shadow);
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
    font-size: 11px;
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

  .ai-section {
    margin-top: 4px;
  }

  .ai-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
  }

  .ai-title {
    font-weight: 500;
    font-size: 12px;
    color: var(--text-primary);
    flex: 1;
  }

  .ai-btn {
    border: 1px solid var(--text-outline);
    background: var(--bg-container-high);
    color: var(--text-primary);
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
  }

  .ai-btn:hover { background: var(--accent-primary); color: var(--bg-surface); }
  .ai-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .ai-btn-sm {
    border: 1px solid var(--text-outline);
    background: none;
    color: var(--text-muted);
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    cursor: pointer;
  }

  .ai-btn-sm:hover { color: var(--text-primary); border-color: var(--accent-primary); }

  .ai-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ai-summary {
    font-size: 12px;
    color: var(--text-primary);
    line-height: 1.4;
  }

  .ai-tags-section, .ai-entities-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .ai-tags, .ai-entities {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .ai-tag {
    background: var(--accent-primary);
    color: var(--bg-surface);
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    font-weight: 500;
  }

  .ai-entity {
    background: var(--bg-container-high);
    color: var(--text-secondary);
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid var(--text-outline);
  }

  .ai-meta {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .ai-empty {
    font-size: 11px;
    color: var(--text-muted);
    font-style: italic;
  }

  .ai-error {
    font-size: 11px;
    color: var(--error);
    margin-bottom: 4px;
  }

  .ai-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }

  .ai-suggestion {
    font-size: 11px;
    color: var(--accent-primary);
    font-family: var(--font-mono);
  }
</style>
