<script lang="ts">
  import { selectedPacket } from '../stores';
  import ProtocolTree from './ProtocolTree.svelte';
  import HexView from './HexView.svelte';

  let activeTab: 'detail' | 'hex' = 'detail';
</script>

<div class="flex flex-col h-full bg-[#1e1e1e] font-sans">
  {#if $selectedPacket}
    <div class="flex bg-[#252526] border-b border-[#3e3e3e] shrink-0 sticky top-0 z-20">
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white { activeTab === 'detail' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'detail'}
      >
        Protocol Tree
      </button>
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white { activeTab === 'hex' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'hex'}
      >
        Hex View
      </button>
    </div>
    
    <div class="flex-1 overflow-hidden relative">
      {#if activeTab === 'detail'}
        <ProtocolTree layers={$selectedPacket.layers} />
      {:else}
        <HexView rawBytes={$selectedPacket.raw_bytes} />
      {/if}
    </div>
  {:else}
    <div class="flex flex-1 items-center justify-center text-[#888] italic bg-[#1e1e1e] p-8 text-center">
      No packet selected. Click on a packet in the list above to view details.
    </div>
  {/if}
</div>