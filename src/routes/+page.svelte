<script lang="ts">
  import CaptureControls from '../lib/components/CaptureControls.svelte';
  import PacketList from '../lib/components/PacketList.svelte';
  import PacketDetailTabs from '../lib/components/PacketDetailTabs.svelte';
  import StatisticsPanel from '../lib/components/StatisticsPanel.svelte';
  import { selectedPacket } from '../lib/stores';

  let showStats = false;
</script>

<div class="app-container">
  <div class="top-pane">
    <CaptureControls />
  </div>
  <div class="middle-pane">
    <PacketList />
  </div>
  <div class="bottom-pane">
    <div class="bottom-tabs">
      <button 
        class="tab-button"
        class:active={!showStats}
        on:click={() => showStats = false}
      >
        Packet Details
      </button>
      <button 
        class="tab-button"
        class:active={showStats}
        on:click={() => showStats = true}
      >
        Statistics
      </button>
    </div>
    <div class="bottom-content">
      {#if showStats}
        <StatisticsPanel />
      {:else}
        <PacketDetailTabs />
      {/if}
    </div>
  </div>
</div>

<style>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: #1e1e1e;
    color: #d4d4d4;
  }

  .top-pane {
    flex-shrink: 0;
  }

  .middle-pane {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .bottom-pane {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    border-top: 2px solid #3e3e3e;
    display: flex;
    flex-direction: column;
  }

  .bottom-tabs {
    display: flex;
    background: #252526;
    border-bottom: 1px solid #3e3e3e;
  }

  .tab-button {
    padding: 0.75rem 1.5rem;
    background: transparent;
    border: none;
    color: #ccc;
    cursor: pointer;
    font-size: 0.9rem;
    border-bottom: 2px solid transparent;
    transition: all 0.2s;
  }

  .tab-button:hover {
    background: #2d2d2d;
    color: #fff;
  }

  .tab-button.active {
    color: #4ec9b0;
    border-bottom-color: #4ec9b0;
    background: #1e1e1e;
  }

  .bottom-content {
    flex: 1;
    overflow: auto;
  }
</style>
