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
    background: var(--shadow);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }

  .dialog {
    background: var(--bg-base);
    border: 1px solid var(--bg-surface1);
    border-radius: 10px;
    padding: 20px 24px;
    min-width: 340px;
    max-width: 420px;
    box-shadow: 0 8px 24px var(--overlay);
  }

  h3 {
    margin: 0 0 8px;
    color: var(--text-primary);
    font-size: 15px;
  }

  .message {
    color: var(--text-subtext);
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
    color: var(--text-subtext);
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
    border: 1px solid var(--bg-surface1);
    border-radius: 6px;
    background: var(--bg-surface0);
    color: var(--text-primary);
    font-size: 13px;
    cursor: pointer;
  }

  .btn:hover {
    background: var(--bg-surface1);
  }

  .btn.replace {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    color: var(--bg-base);
    font-weight: 500;
  }

  .btn.replace:hover {
    background: var(--accent-sapphire);
    border-color: var(--accent-sapphire);
  }
</style>
