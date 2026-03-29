<script lang="ts">
  import { selectedPacket } from '../stores';
  import ProtocolTree from './ProtocolTree.svelte';
  import HexView from './HexView.svelte';

  let activeTab: 'detail' | 'hex' = 'detail';
</script>

<div class="flex flex-col h-full bg-[#1e1e1e]">
  {#if $selectedPacket}
    <div class="flex bg-[#252526] border-b border-[#3e3e3e]">
      <button 
        class="px-6 py-3 bg-transparent border-none text-[#ccc] cursor-pointer text-sm border-b-2 border-transparent transition-all duration-200 hover:bg-[#2d2d2d] hover:text-white {activeTab === 'detail' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : ''}"
        on:click={() => activeTab = 'detail'}
      >
        Protocol Tree
      </button>
      <button 
        class="px-6 py-3 bg-transparent border-none text-[#ccc] cursor-pointer text-sm border-b-2 border-transparent transition-all duration-200 hover:bg-[#2d2d2d] hover:text-white {activeTab === 'hex' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : ''}"
        on:click={() => activeTab = 'hex'}
      >
        Hex View
      </button>
    </div>

    <div class="flex-1 overflow-auto">
      {#if activeTab === 'detail'}
        <ProtocolTree layers={$selectedPacket.layers} />
      {:else}
        <HexView rawBytes={$selectedPacket.raw_bytes} />
      {/if}
    </div>
  {:else}
    <div class="p-8 text-center text-[#888] italic">
      No packet selected. Click on a packet in the list above to view details.
    </div>
  {/if}
</div>
