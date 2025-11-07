<script lang="ts">
  import { selectedPacket } from '../stores';
  import ProtocolTree from './ProtocolTree.svelte';
  import HexView from './HexView.svelte';

  let activeTab: 'detail' | 'hex' = 'detail';
</script>

<div class="detail-container">
  {#if $selectedPacket}
    <div class="tabs">
      <button 
        class="tab-btn"
        class:active={activeTab === 'detail'}
        on:click={() => activeTab = 'detail'}
      >
        Protocol Tree
      </button>
      <button 
        class="tab-btn"
        class:active={activeTab === 'hex'}
        on:click={() => activeTab = 'hex'}
      >
        Hex View
      </button>
    </div>

    <div class="tab-content">
      {#if activeTab === 'detail'}
        <ProtocolTree layers={$selectedPacket.layers} />
      {:else}
        <HexView rawBytes={$selectedPacket.raw_bytes} />
      {/if}
    </div>
  {:else}
    <div class="empty-state">
      No packet selected. Click on a packet in the list above to view details.
    </div>
  {/if}
</div>

<style>
  .detail-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e1e;
  }

  .tabs {
    display: flex;
    background: #252526;
    border-bottom: 1px solid #3e3e3e;
  }

  .tab-btn {
    padding: 0.75rem 1.5rem;
    background: transparent;
    border: none;
    color: #ccc;
    cursor: pointer;
    font-size: 0.9rem;
    border-bottom: 2px solid transparent;
    transition: all 0.2s;
  }

  .tab-btn:hover {
    background: #2d2d2d;
    color: #fff;
  }

  .tab-btn.active {
    color: #4ec9b0;
    border-bottom-color: #4ec9b0;
    background: #1e1e1e;
  }

  .tab-content {
    flex: 1;
    overflow: auto;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: #888;
    font-style: italic;
  }
</style>
