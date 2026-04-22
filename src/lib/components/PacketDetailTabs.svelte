<script lang="ts">
  import { selectedPacket } from '../stores';
  import ProtocolTree from './ProtocolTree.svelte';
  import HexView from './HexView.svelte';
  import PacketOverview from './PacketOverview.svelte';

  type TabId = 'overview' | 'layers' | 'bytes';

  let activeTab: TabId = 'overview';

  const tabs: { id: TabId; label: string; description: string }[] = [
    { id: 'overview', label: 'Overview', description: 'Packet summary and context' },
    { id: 'layers', label: 'Layers', description: 'Protocol layer breakdown' },
    { id: 'bytes', label: 'Bytes', description: 'Raw packet data' },
  ];
</script>

<div
  class="flex flex-col h-full"
  style="background-color: var(--bg-page); font-family: var(--font-sans);"
>
  {#if $selectedPacket}
    {#if $selectedPacket.expert_summary && $selectedPacket.expert_summary.length > 0}
      <div
        class="border-b p-2 flex flex-col gap-1 shrink-0"
        style="background-color: rgba(239, 68, 68, 0.1); border-color: #ef4444;"
      >
        {#each $selectedPacket.expert_summary as expert}
          <div
            class="flex items-center gap-2 text-xs font-medium uppercase tracking-tight"
            style="color: #ef4444;"
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
      style="background-color: var(--border-standard); border-color: var(--border-standard);"
    >
      {#each tabs as tab}
        <button
          class="tab-btn px-4 py-2 cursor-pointer text-sm font-medium border-b-[3px] border-transparent transition-colors shrink-0 {activeTab ===
          tab.id
            ? 'tab-active'
            : ''}"
          style="color: var(--text-muted);"
          on:click={() => (activeTab = tab.id)}
          title={tab.description}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div class="flex-1 overflow-hidden relative">
      {#if activeTab === 'overview'}
        <PacketOverview packet={$selectedPacket} />
      {:else if activeTab === 'layers'}
        <ProtocolTree layers={$selectedPacket.layers} />
      {:else if activeTab === 'bytes'}
        <HexView rawBytes={$selectedPacket.raw_bytes} />
      {/if}
    </div>
  {:else}
    <div
      class="flex flex-1 items-center justify-center p-8 text-center"
      style="color: var(--text-muted); background-color: var(--bg-page);"
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
    color: var(--text-muted);
    transition:
      color 0.15s ease,
      border-color 0.15s ease,
      background-color 0.15s ease;
  }

  .tab-btn:hover {
    color: var(--text-primary);
    background-color: var(--border-standard);
  }

  .tab-active {
    color: var(--brand-green) !important;
    border-bottom-color: var(--brand-green) !important;
    background-color: var(--bg-button) !important;
  }
</style>
