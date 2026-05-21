<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let conflicts: Array<{ source: string; dest: string; filename: string }> = [];

  const dispatch = createEventDispatcher<{
    resolve: { action: 'replace' | 'skip' | 'rename'; applyAll: boolean };
  }>();

  let applyAll = false;

  function resolve(action: 'replace' | 'skip' | 'rename') {
    dispatch('resolve', { action, applyAll });
  }
</script>

{#if conflicts.length > 0}
  <div class="overlay" on:click|self={() => resolve('skip')}>
    <div class="dialog">
      <h3>File Conflict</h3>
      <p class="message">
        {conflicts.length === 1
          ? `"${conflicts[0].filename}" already exists in the destination.`
          : `${conflicts.length} files already exist in the destination.`}
      </p>
      <div class="actions">
        <label class="apply-all">
          <input type="checkbox" bind:checked={applyAll} />
          Apply to all
        </label>
        <div class="buttons">
          <button class="btn skip" on:click={() => resolve('skip')}>Skip</button>
          <button class="btn rename" on:click={() => resolve('rename')}>Keep Both</button>
          <button class="btn replace" on:click={() => resolve('replace')}>Replace</button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }

  .dialog {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 10px;
    padding: 20px 24px;
    min-width: 340px;
    max-width: 420px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  }

  h3 {
    margin: 0 0 8px;
    color: #cdd6f4;
    font-size: 15px;
  }

  .message {
    color: #a6adc8;
    font-size: 13px;
    margin: 0 0 16px;
  }

  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .apply-all {
    font-size: 12px;
    color: #a6adc8;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .buttons {
    display: flex;
    gap: 8px;
  }

  .btn {
    padding: 6px 14px;
    border: 1px solid #45475a;
    border-radius: 6px;
    background: #313244;
    color: #cdd6f4;
    font-size: 13px;
    cursor: pointer;
  }

  .btn:hover {
    background: #45475a;
  }

  .btn.replace {
    background: #89b4fa;
    border-color: #89b4fa;
    color: #1e1e2e;
    font-weight: 500;
  }

  .btn.replace:hover {
    background: #74c7ec;
    border-color: #74c7ec;
  }
</style>
