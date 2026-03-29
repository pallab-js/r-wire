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

<div class="flex flex-col h-full bg-[#1e1e1e] p-6 overflow-hidden font-sans">
  <div class="mb-6 flex items-center justify-between">
    <div>
      <h3 class="text-[#4ec9b0] text-sm font-bold uppercase tracking-widest mb-1">Flow Timeline</h3>
      <p class="text-xs text-[#888]">Temporal distribution of packets in this conversation</p>
    </div>
    {#if flowPackets.length > 0}
      <div class="text-xs font-mono text-[#ccc] bg-[#252526] px-3 py-1 rounded border border-[#3e3e3e]">
        DURATION: {durationMs.toFixed(3)} ms | PACKETS: {flowPackets.length}
      </div>
    {/if}
  </div>

  <div class="flex-1 flex flex-col min-h-0">
    {#if loading}
      <div class="flex-1 flex items-center justify-center text-[#888] italic">Analyzing temporal data...</div>
    {:else if flowPackets.length === 0}
      <div class="flex-1 flex items-center justify-center text-[#888] italic">No flow data available for this packet.</div>
    {:else}
      <div class="relative h-24 bg-[#252526] rounded-lg border border-[#3e3e3e] mb-8 overflow-hidden">
        <!-- Time scale -->
        <div class="absolute inset-x-0 bottom-0 h-6 border-t border-[#3e3e3e] bg-[#1e1e1e]/50 flex justify-between px-2 items-center text-[0.65rem] text-[#666] font-mono">
          <span>0ms</span>
          <span>{durationMs.toFixed(1)}ms</span>
        </div>

        <!-- Packet markers -->
        {#each flowPackets as p}
          <div 
            class="absolute top-4 bottom-8 w-px bg-[#4ec9b0] hover:w-0.5 hover:bg-white transition-all cursor-crosshair group"
            style="left: {getX(p.timestamp, minTime, maxTime)}"
            title="Packet #{p.id} | {((p.timestamp - minTime)/1000000).toFixed(3)}ms"
          >
            <div class="hidden group-hover:block absolute -top-2 left-1/2 -translate-x-1/2 bg-white text-black text-[0.6rem] px-1 rounded font-bold z-10 whitespace-nowrap">
              #{p.id}
            </div>
          </div>
        {/each}
      </div>

      <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar">
        <h4 class="text-[0.7rem] font-bold text-[#888] uppercase mb-3 tracking-wider">Event Log</h4>
        <div class="flex flex-col gap-1 font-mono text-[0.75rem]">
          {#each flowPackets as p}
            <div class="flex items-center gap-4 py-1.5 px-3 rounded hover:bg-[#2a2d2e] transition-colors border-b border-white/5">
              <span class="text-[#666] w-12">#{p.id}</span>
              <span class="text-[#4ec9b0] w-24">+{((p.timestamp - minTime)/1000000).toFixed(3)} ms</span>
              <span class="text-[#dcdcaa] w-24">{p.source_addr === flowPackets[0].source_addr ? 'SRC → DST' : 'DST → SRC'}</span>
              <span class="text-[#ccc] flex-1 truncate">{p.info}</span>
              <span class="text-[#888] text-[0.7rem]">{p.length} B</span>
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
    background: #3e3e3e;
    border-radius: 10px;
  }
</style>
