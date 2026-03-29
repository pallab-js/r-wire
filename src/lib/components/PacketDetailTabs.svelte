<script lang="ts">
  import { selectedPacket } from '../stores';
  import ProtocolTree from './ProtocolTree.svelte';
  import HexView from './HexView.svelte';
  import SequenceDiagram from './SequenceDiagram.svelte';
  import PayloadInspector from './PayloadInspector.svelte';
  import ForensicNarrative from './ForensicNarrative.svelte';
  import ForensicIntelligence from './ForensicIntelligence.svelte';
  import ForensicArtifacts from './ForensicArtifacts.svelte';
  import FlowTimeline from './FlowTimeline.svelte';

  let activeTab: 'detail' | 'hex' | 'sequence' | 'payload' | 'narrative' | 'intelligence' | 'artifacts' | 'timeline' = 'detail';
</script>

<div class="flex flex-col h-full bg-[#1e1e1e] font-sans">
  {#if $selectedPacket}
    <!-- Expert Summary Bar -->
    {#if $selectedPacket.expert_summary && $selectedPacket.expert_summary.length > 0}
      <div class="bg-[#5a1a1a] border-b border-[#7a2a2a] p-2 flex flex-col gap-1 shrink-0">
        {#each $selectedPacket.expert_summary as expert}
          <div class="flex items-center gap-2 text-[#ff6b6b] text-xs font-bold uppercase tracking-tight">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>
            {expert}
          </div>
        {/each}
      </div>
    {/if}

    <div class="flex bg-[#252526] border-b border-[#3e3e3e] shrink-0 sticky top-0 z-20 overflow-x-auto no-scrollbar">
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white shrink-0 { activeTab === 'narrative' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'narrative'}
      >
        Narrative
      </button>
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white shrink-0 { activeTab === 'detail' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'detail'}
      >
        Protocol Tree
      </button>
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white shrink-0 { activeTab === 'hex' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'hex'}
      >
        Hex View
      </button>
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white shrink-0 { activeTab === 'payload' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'payload'}
      >
        Payload
      </button>
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white shrink-0 { activeTab === 'intelligence' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'intelligence'}
      >
        Intelligence
      </button>
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white shrink-0 { activeTab === 'artifacts' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'artifacts'}
      >
        Artifacts
      </button>
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white shrink-0 { activeTab === 'timeline' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'timeline'}
      >
        Timeline
      </button>
      <button 
        class="px-4 py-2 bg-transparent border-none text-[#ccc] cursor-pointer text-[0.85rem] font-medium border-b-[3px] border-transparent transition-colors hover:text-white shrink-0 { activeTab === 'sequence' ? '!text-[#4ec9b0] !border-[#4ec9b0] !bg-[#1e1e1e]' : 'hover:bg-[#2d2d2d]' }"
        on:click={() => activeTab = 'sequence'}
      >
        Sequence
      </button>
    </div>
    
    <div class="flex-1 overflow-hidden relative">
      {#if activeTab === 'narrative'}
        <ForensicNarrative narrative={$selectedPacket.narrative} />
      {:else if activeTab === 'detail'}
        <ProtocolTree layers={$selectedPacket.layers} />
      {:else if activeTab === 'hex'}
        <HexView rawBytes={$selectedPacket.raw_bytes} />
      {:else if activeTab === 'payload'}
        <PayloadInspector rawBytes={$selectedPacket.raw_bytes} />
      {:else if activeTab === 'intelligence'}
        <ForensicIntelligence intelligence={$selectedPacket.intelligence} />
      {:else if activeTab === 'artifacts'}
        <ForensicArtifacts artifacts={$selectedPacket.artifacts} />
      {:else if activeTab === 'timeline'}
        <FlowTimeline />
      {:else if activeTab === 'sequence'}
        <SequenceDiagram />
      {/if}
    </div>
  {:else}
    <div class="flex flex-1 items-center justify-center text-[#888] italic bg-[#1e1e1e] p-8 text-center">
      No packet selected. Click on a packet in the list above to view details.
    </div>
  {/if}
</div>

<style>
  .no-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .no-scrollbar {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>