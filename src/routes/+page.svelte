<script lang="ts">
  import CaptureControls from '../lib/components/CaptureControls.svelte';
  import PacketList from '../lib/components/PacketList.svelte';
  import PacketDetailTabs from '../lib/components/PacketDetailTabs.svelte';
  import StatisticsPanel from '../lib/components/StatisticsPanel.svelte';
  import { selectedPacket } from '../lib/stores';

  let showStats = false;
</script>

<div class="flex flex-col h-screen w-screen bg-[#1e1e1e] text-[#d4d4d4]">
  <div class="shrink-0">
    <CaptureControls />
  </div>
  <div class="flex-1 min-h-0 overflow-hidden">
    <PacketList />
  </div>
  <div class="flex-1 min-h-0 overflow-hidden border-t-2 border-[#3e3e3e] flex flex-col">
    <div class="flex bg-[#252526] border-b border-[#3e3e3e]">
      <button 
        class="px-6 py-3 bg-transparent border-none text-[#ccc] cursor-pointer text-sm border-b-2 border-transparent transition-all duration-200 hover:bg-[#2d2d2d] hover:text-white { !showStats ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : '' }"
        on:click={() => showStats = false}
      >
        Packet Details
      </button>
      <button 
        class="px-6 py-3 bg-transparent border-none text-[#ccc] cursor-pointer text-sm border-b-2 border-transparent transition-all duration-200 hover:bg-[#2d2d2d] hover:text-white { showStats ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : '' }"
        on:click={() => showStats = true}
      >
        Statistics
      </button>
    </div>
    <div class="flex-1 overflow-auto">
      {#if showStats}
        <StatisticsPanel />
      {:else}
        <PacketDetailTabs />
      {/if}
    </div>
  </div>
</div>
