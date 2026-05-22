<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';

  interface OpState {
    id: string;
    opType: string;
    totalItems: number;
    processed: number;
    current: string;
    done: boolean;
    success: boolean;
    error: string | null;
  }

  let operations: OpState[] = [];

  onMount(() => {
    const unlisteners: Array<() => void> = [];

    listen<{ id: string; opType: string; totalItems: number }>('op:started', (e) => {
      operations = [...operations, {
        id: e.payload.id,
        opType: e.payload.opType,
        totalItems: e.payload.totalItems,
        processed: 0,
        current: '',
        done: false,
        success: true,
        error: null,
      }];
    }).then(u => unlisteners.push(u));

    listen<{ id: string; processed: number; current: string }>('op:progress', (e) => {
      operations = operations.map(op =>
        op.id === e.payload.id
          ? { ...op, processed: e.payload.processed, current: e.payload.current }
          : op
      );
    }).then(u => unlisteners.push(u));

    listen<{ id: string; success: boolean; error: string | null }>('op:complete', (e) => {
      operations = operations.map(op =>
        op.id === e.payload.id
          ? { ...op, done: true, success: e.payload.success, error: e.payload.error }
          : op
      );
      // Auto-dismiss after 3s
      setTimeout(() => {
        operations = operations.filter(op => op.id !== e.payload.id);
      }, 3000);
    }).then(u => unlisteners.push(u));

    return () => {
      unlisteners.forEach(u => u());
    };
  });

  function getLabel(op: OpState): string {
    const verb = op.done
      ? (op.success ? 'Completed' : 'Failed')
      : op.opType === 'Copy' ? 'Copying' : op.opType === 'Move' ? 'Moving' : 'Deleting';
    return `${verb} ${op.totalItems} item${op.totalItems > 1 ? 's' : ''}`;
  }

  function getPercent(op: OpState): number {
    if (op.totalItems === 0) return 100;
    return Math.round((op.processed / op.totalItems) * 100);
  }
</script>

{#if operations.length > 0}
  <div class="toast-container">
    {#each operations as op (op.id)}
      <div class="toast" class:error={op.done && !op.success}>
        <div class="toast-header">
          <span>{getLabel(op)}</span>
          <span class="percent">{op.done ? '' : `${getPercent(op)}%`}</span>
        </div>
        {#if !op.done}
          <div class="progress-bar">
            <div class="progress-fill" style="width: {getPercent(op)}%"></div>
          </div>
          <div class="current-file">{op.current.split('/').pop()}</div>
        {/if}
        {#if op.error}
          <div class="error-msg">{op.error}</div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: 32px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 900;
    max-width: 300px;
  }

  .toast {
    background: #313244;
    border: 1px solid #45475a;
    border-radius: 8px;
    padding: 10px 14px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .toast.error {
    border-color: #f38ba8;
  }

  .toast-header {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: #cdd6f4;
    margin-bottom: 6px;
  }

  .percent {
    color: #a6adc8;
  }

  .progress-bar {
    height: 4px;
    background: #45475a;
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: #89b4fa;
    transition: width 0.2s;
  }

  .current-file {
    margin-top: 4px;
    font-size: 11px;
    color: #6c7086;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error-msg {
    margin-top: 4px;
    font-size: 11px;
    color: #f38ba8;
  }
</style>
