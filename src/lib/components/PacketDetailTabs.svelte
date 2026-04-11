<script lang="ts">
  import { selectedPacket } from '../stores';
  import ProtocolTree from './ProtocolTree.svelte';
  import HexView from './HexView.svelte';
  import PayloadInspector from './PayloadInspector.svelte';
  import ForensicNarrative from './ForensicNarrative.svelte';
  import ForensicIntelligence from './ForensicIntelligence.svelte';
  import ForensicArtifacts from './ForensicArtifacts.svelte';
  import FlowTimeline from './FlowTimeline.svelte';

  type TabId =
    | 'essentials'
    | 'detail'
    | 'hex'
    | 'payload'
    | 'intelligence'
    | 'artifacts'
    | 'timeline';

  let activeTab: TabId = 'essentials';

  const tabs: { id: TabId; label: string; description: string }[] = [
    { id: 'essentials', label: 'Essentials', description: 'Quick overview' },
    { id: 'detail', label: 'Protocol Tree', description: 'Full packet breakdown' },
    { id: 'hex', label: 'Hex View', description: 'Raw bytes' },
    { id: 'payload', label: 'Payload', description: 'Decoded content' },
    { id: 'intelligence', label: 'Intelligence', description: 'Risk & fingerprints' },
    { id: 'artifacts', label: 'Artifacts', description: 'Extracted files' },
    { id: 'timeline', label: 'Timeline', description: 'Flow timing' },
  ];
</script>

<div
  class="flex flex-col h-full"
  style="background-color: var(--cursor-cream); font-family: var(--font-gothic);"
>
  {#if $selectedPacket}
    {#if $selectedPacket.expert_summary && $selectedPacket.expert_summary.length > 0}
      <div
        class="border-b p-2 flex flex-col gap-1 shrink-0"
        style="background-color: rgba(255, 77, 109, 0.1); border-color: var(--color-error);"
      >
        {#each $selectedPacket.expert_summary as expert}
          <div
            class="flex items-center gap-2 text-xs font-bold uppercase tracking-tight"
            style="color: var(--color-error);"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="3"
              stroke-linecap="round"
              stroke-linejoin="round"
              ><path
                d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"
              /><path d="M12 9v4" /><path d="M12 17h.01" /></svg
            >
            {expert}
          </div>
        {/each}
      </div>
    {/if}

    <div
      class="flex border-b shrink-0 sticky top-0 z-20 overflow-x-auto no-scrollbar"
      style="background-color: var(--surface-200); border-color: var(--border-primary);"
    >
      {#each tabs as tab}
        <button
          class="tab-btn px-4 py-2 cursor-pointer text-sm font-medium border-b-[3px] border-transparent transition-colors shrink-0 {activeTab ===
          tab.id
            ? 'tab-active'
            : ''}"
          style="color: rgba(38, 37, 30, 0.55);"
          on:click={() => (activeTab = tab.id)}
          title={tab.description}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div class="flex-1 overflow-hidden relative">
      {#if activeTab === 'essentials'}
        <div class="h-full overflow-y-auto p-6" style="background-color: var(--cursor-cream);">
          <div class="max-w-4xl space-y-6">
            <ForensicNarrative narrative={$selectedPacket.narrative} />
            <div class="border-t" style="border-color: var(--border-primary);"></div>
            <PayloadInspector rawBytes={$selectedPacket.raw_bytes} />
          </div>
        </div>
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
      {/if}
    </div>
  {:else}
    <div
      class="flex flex-1 items-center justify-center italic p-8 text-center"
      style="color: rgba(38, 37, 30, 0.55); background-color: var(--cursor-cream);"
    >
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

  .tab-btn {
    background: transparent;
    border: none;
    color: rgba(38, 37, 30, 0.55);
    transition:
      color 0.15s ease,
      border-color 0.15s ease,
      background-color 0.15s ease;
  }

  .tab-btn:hover {
    color: var(--cursor-dark);
    background-color: var(--surface-100);
  }

  .tab-active {
    color: var(--color-success) !important;
    border-bottom-color: var(--color-success) !important;
    background-color: var(--surface-100) !important;
  }
</style>
