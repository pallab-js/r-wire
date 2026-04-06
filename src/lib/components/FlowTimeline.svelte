<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import { selectedPacket, type PacketSummary } from '../stores';

  let flowPackets: PacketSummary[] = [];
  let loading = false;

  $: if ($selectedPacket) {
    loadFlowPackets($selectedPacket.summary.id);
  }

  async function loadFlowPackets(id: number) {
    loading = true;
    try {
      flowPackets = await invoke<PacketSummary[]>('get_flow_packets', { packetId: id });
    } catch (err) {
      console.error('Failed to load flow packets:', err);
    } finally {
      loading = false;
    }
  }

  function getX(timestamp: number, min: number, max: number): string {
    if (max === min) return '50%';
    const percent = ((timestamp - min) / (max - min)) * 100;
    return `${percent}%`;
  }

  $: timestamps = flowPackets.map(p => p.timestamp);
  $: minTime = Math.min(...timestamps);
  $: maxTime = Math.max(...timestamps);
  $: durationMs = (maxTime - minTime) / 1000000;
</script>

<div class="flex flex-col h-full p-6 overflow-hidden" style="background-color: var(--cursor-cream); font-family: var(--font-mono);">
  <div class="mb-6 flex items-center justify-between">
    <div>
      <h3 class="text-micro mb-1" style="color: var(--color-success);">Flow Timeline</h3>
      <p class="text-xs" style="color: rgba(38, 37, 30, 0.55);">Temporal distribution of packets in this conversation</p>
    </div>
    {#if flowPackets.length > 0}
      <div class="text-xs px-3 py-1 rounded border" style="background-color: var(--surface-200); border-color: var(--border-primary); color: var(--cursor-dark); font-family: var(--font-mono);">
        DURATION: {durationMs.toFixed(3)} ms | PACKETS: {flowPackets.length}
      </div>
    {/if}
  </div>

  <div class="flex-1 flex flex-col min-h-0">
    {#if loading}
      <div class="flex-1 flex items-center justify-center italic" style="color: rgba(38, 37, 30, 0.55);">Analyzing temporal data...</div>
    {:else if flowPackets.length === 0}
      <div class="flex-1 flex items-center justify-center italic" style="color: rgba(38, 37, 30, 0.55);">No flow data available for this packet.</div>
    {:else}
      <div class="relative h-24 rounded-lg border mb-8 overflow-hidden" style="background-color: var(--surface-200); border-color: var(--border-primary);">
        <!-- Time scale -->
        <div class="absolute inset-x-0 bottom-0 h-6 border-t flex justify-between px-2 items-center text-xs font-mono" style="border-color: var(--border-primary); background-color: var(--surface-200); color: rgba(38, 37, 30, 0.55);">
          <span>0ms</span>
          <span>{durationMs.toFixed(1)}ms</span>
        </div>

        <!-- Packet markers -->
        {#each flowPackets as p}
          <div
            class="absolute top-4 bottom-8 w-px hover:w-0.5 hover:bg-[var(--cursor-dark)] transition-all cursor-crosshair group"
            style="left: {getX(p.timestamp, minTime, maxTime)}; background-color: var(--color-success);"
            title="Packet #{p.id} | {((p.timestamp - minTime)/1000000).toFixed(3)}ms"
          >
            <div class="hidden group-hover:block absolute -top-2 left-1/2 -translate-x-1/2 text-[var(--cursor-dark)] text-xs px-1 rounded font-bold z-10 whitespace-nowrap" style="background-color: var(--surface-100);">
              #{p.id}
            </div>
          </div>
        {/each}
      </div>

      <div class="flex-1 overflow-y-auto pr-2">
        <h4 class="text-xs font-bold uppercase mb-3 tracking-wider" style="color: rgba(38, 37, 30, 0.55);">Event Log</h4>
        <div class="flex flex-col gap-1 text-xs">
          {#each flowPackets as p}
            <div class="flex items-center gap-4 py-1.5 px-3 rounded transition-colors border-b" style="border-color: var(--border-primary);" on:mouseenter={(e) => e.currentTarget.style.backgroundColor = 'var(--surface-200)'} on:mouseleave={(e) => e.currentTarget.style.backgroundColor = ''}>
              <span class="w-12" style="color: rgba(38, 37, 30, 0.55);">#{p.id}</span>
              <span class="w-24" style="color: var(--color-success);">+{((p.timestamp - minTime)/1000000).toFixed(3)} ms</span>
              <span class="w-24" style="color: rgba(38, 37, 30, 0.55);">{p.source_addr === flowPackets[0].source_addr ? 'SRC -> DST' : 'DST -> SRC'}</span>
              <span class="flex-1 truncate" style="color: var(--cursor-dark);">{p.info}</span>
              <span class="text-xs" style="color: rgba(38, 37, 30, 0.55);">{p.length} B</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: var(--border-primary);
    border-radius: 10px;
  }
</style>
