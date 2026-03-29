<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/tauri';
  import { selectedPacket, filteredPackets, addPackets, packetList, type PacketSummary, type PacketDetail } from '../stores';
  import { onMount } from 'svelte';

  let selectedId: number | null = null;
  
  // Use memoized filtered packets from store
  $: filteredPacketsList = $filteredPackets;
  
  // Timestamp formatting cache
  const timestampCache = new Map<number, string>();

  // Virtual Scrolling State
  let scrollTop = 0;
  let clientHeight = 600;
  const ROW_HEIGHT = 28; // Fixed height per row
  const OVERSCAN = 20;   // Render 20 rows above/below

  $: totalPacketsCount = filteredPacketsList.length;
  $: startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  $: endIndex = Math.min(totalPacketsCount, Math.floor((scrollTop + clientHeight) / ROW_HEIGHT) + OVERSCAN);
  $: visiblePackets = filteredPacketsList.slice(startIndex, endIndex);
  $: paddingTop = startIndex * ROW_HEIGHT;
  $: paddingBottom = Math.max(0, (totalPacketsCount - endIndex) * ROW_HEIGHT);

  onMount(() => {
    let unlistenFn: (() => void) | null = null;
    
    listen('new_packet_batch', (event: any) => {
      const newPackets = event.payload as PacketSummary[];
      addPackets(newPackets);
    }).then(fn => {
      unlistenFn = fn;
    });

    return () => {
      if (unlistenFn) {
        unlistenFn();
      }
    };
  });

  async function selectPacket(packet: PacketSummary) {
    selectedId = packet.id;
    try {
      const detail = await invoke<PacketDetail>('get_packet_detail', { id: packet.id });
      if (detail) {
        selectedPacket.set(detail);
      }
    } catch (error: any) {
      console.error('Failed to get packet detail:', error);
    }
  }

  function formatTimestamp(timestamp: number): string {
    if (!timestamp || timestamp <= 0) return '00:00:00.000';
    if (timestampCache.has(timestamp)) return timestampCache.get(timestamp)!;
    
    try {
      const date = new Date(timestamp / 1000000);
      if (isNaN(date.getTime())) return '00:00:00.000';
      const formatted = date.toLocaleTimeString('en-US', { 
        hour12: false, 
        hour: '2-digit', 
        minute: '2-digit', 
        second: '2-digit',
        fractionalSecondDigits: 3
      });
      
      if (timestampCache.size > 10000) {
        const firstKey = timestampCache.keys().next().value;
        if (firstKey !== undefined) timestampCache.delete(firstKey);
      }
      timestampCache.set(timestamp, formatted);
      return formatted;
    } catch (e) {
      return '00:00:00.000';
    }
  }

  function handleScroll(e: Event) {
    scrollTop = (e.target as HTMLElement).scrollTop;
  }

  function getProtocolColor(protocol: string) {
    const p = protocol.toLowerCase();
    if (p === 'tcp') return 'text-[#4a9eff] border-l-[#4a9eff]';
    if (p === 'udp') return 'text-[#ffa500] border-l-[#ffa500]';
    if (p === 'icmp' || p === 'icmpv6') return 'text-[#00ff00] border-l-[#00ff00]';
    if (p === 'arp') return 'text-[#ff6b6b] border-l-[#ff6b6b]';
    if (p === 'dns') return 'text-[#9b59b6] border-l-[#9b59b6]';
    if (p === 'http' || p === 'https') return 'text-[#3498db] border-l-[#3498db]';
    return 'text-[#dcdcaa] border-l-transparent'; // default
  }
</script>

<div class="flex-1 overflow-y-auto overflow-x-hidden bg-[#1e1e1e] h-full relative" on:scroll={handleScroll} bind:clientHeight>
  <table class="w-full border-collapse font-mono text-[0.85rem] table-fixed">
    <thead class="sticky top-0 bg-[#252526] z-10">
      <tr>
        <th class="w-[80px] px-2 py-1.5 text-left text-[#ccc] font-semibold border-b-2 border-[#3e3e3e]">No.</th>
        <th class="w-[120px] px-2 py-1.5 text-left text-[#ccc] font-semibold border-b-2 border-[#3e3e3e]">Time</th>
        <th class="w-[200px] px-2 py-1.5 text-left text-[#ccc] font-semibold border-b-2 border-[#3e3e3e]">Source</th>
        <th class="w-[200px] px-2 py-1.5 text-left text-[#ccc] font-semibold border-b-2 border-[#3e3e3e]">Destination</th>
        <th class="w-[100px] px-2 py-1.5 text-left text-[#ccc] font-semibold border-b-2 border-[#3e3e3e]">Protocol</th>
        <th class="w-[80px] px-2 py-1.5 text-left text-[#ccc] font-semibold border-b-2 border-[#3e3e3e]">Length</th>
        <th class="px-2 py-1.5 text-left text-[#ccc] font-semibold border-b-2 border-[#3e3e3e]">Info</th>
      </tr>
    </thead>
    <tbody>
      {#if paddingTop > 0}
        <tr style="height: {paddingTop}px;">
          <td colspan="7" class="p-0 border-none"></td>
        </tr>
      {/if}
      
      {#each visiblePackets as packet (packet.id)}
        {@const colorClasses = getProtocolColor(packet.protocol)}
        <tr 
          class="cursor-pointer bg-[#1e1e1e] hover:bg-[#2a2a2a] {selectedId === packet.id ? '!bg-[#094771]' : ''} border-l-[3px]"
          class:!border-l-[#4a9eff]={packet.protocol === 'TCP'}
          class:!border-l-[#ffa500]={packet.protocol === 'UDP'}
          class:!border-l-[#00ff00]={packet.protocol === 'ICMP' || packet.protocol === 'ICMPv6'}
          class:!border-l-[#ff6b6b]={packet.protocol === 'ARP'}
          class:!border-l-[#9b59b6]={packet.protocol === 'DNS'}
          class:!border-l-[#3498db]={packet.protocol === 'HTTP' || packet.protocol === 'HTTPS'}
          on:click={() => selectPacket(packet)}
        >
          <td class="px-2 py-0 text-[#d4d4d4] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]">{packet.id}</td>
          <td class="px-2 py-0 text-[#d4d4d4] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]">{formatTimestamp(packet.timestamp)}</td>
          <td class="px-2 py-0 text-[#4ec9b0] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]">{packet.source_addr}</td>
          <td class="px-2 py-0 text-[#4ec9b0] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]">{packet.dest_addr}</td>
          <td class="px-2 py-0 border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-semibold {colorClasses.split(' ')[0]}">{packet.protocol}</td>
          <td class="px-2 py-0 text-[#d4d4d4] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]">{packet.length}</td>
          <td class="px-2 py-0 text-[#ce9178] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]" title={packet.info}>{packet.info}</td>
        </tr>
      {/each}
      
      {#if paddingBottom > 0}
        <tr style="height: {paddingBottom}px;">
          <td colspan="7" class="p-0 border-none"></td>
        </tr>
      {/if}
    </tbody>
  </table>
  
  {#if filteredPacketsList.length === 0}
    <div class="absolute top-[50px] left-0 right-0 p-8 text-center text-[#888] italic">
      {#if $packetList.length > 0}
        No packets match the current filter. ({$packetList.length.toLocaleString()} total packets)
      {:else}
        No packets captured yet. Click "Start" to begin capturing.
      {/if}
    </div>
  {/if}
</div>
