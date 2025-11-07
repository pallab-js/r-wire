<script lang="ts">
  import type { ProtocolLayer } from '../stores';

  export let layers: ProtocolLayer[];
</script>

<div class="protocol-tree">
  {#if !layers || layers.length === 0}
    <div class="empty-layers">No protocol layers available</div>
  {:else}
    {#each layers as layer}
    <details class="layer-details" open>
      <summary class="layer-summary">
        <span class="layer-name">{layer.name}</span>
      </summary>
      <div class="layer-fields">
        {#each layer.fields as [fieldName, fieldValue]}
          <div class="field-row">
            <span class="field-name">{fieldName}:</span>
            <span class="field-value">{fieldValue}</span>
          </div>
        {/each}
      </div>
    </details>
    {/each}
  {/if}
</div>

<style>
  .protocol-tree {
    padding: 0.5rem;
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
  }

  .layer-details {
    margin-bottom: 0.5rem;
    border: 1px solid #3e3e3e;
    border-radius: 4px;
    background: #252526;
  }

  .layer-summary {
    padding: 0.75rem;
    cursor: pointer;
    user-select: none;
    background: #2d2d2d;
    border-radius: 4px 4px 0 0;
  }

  .layer-summary:hover {
    background: #353535;
  }

  .layer-name {
    color: #4ec9b0;
    font-weight: 600;
    font-size: 1rem;
  }

  .layer-fields {
    padding: 0.5rem 0.75rem;
    background: #1e1e1e;
  }

  .field-row {
    display: flex;
    padding: 0.25rem 0;
    gap: 1rem;
  }

  .field-name {
    color: #9cdcfe;
    min-width: 150px;
    font-weight: 500;
  }

  .field-value {
    color: #ce9178;
    flex: 1;
  }

  .empty-layers {
    padding: 2rem;
    text-align: center;
    color: #888;
    font-style: italic;
  }
</style>
