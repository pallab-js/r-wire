<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/tauri';
  import { selectedPacket, addPackets, totalFilteredCount, debouncedFilter, packetList, type PacketSummary, type PacketDetail } from '../stores';
  import { onMount } from 'svelte';

  let selectedId: number | null = null;
  
  // Local cache for current window of packets
  let visiblePackets: PacketSummary[] = [];
  
  // Timestamp formatting cache
  const timestampCache = new Map<number, string>();

  // Virtual Scrolling State
  let scrollTop = 0;
  let clientHeight = 600;
  const ROW_HEIGHT = 28; // Fixed height per row
  const OVERSCAN = 30;   // Render 30 rows above/below

  $: totalPacketsCount = $totalFilteredCount;
  $: startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  $: endIndex = Math.min(totalPacketsCount, Math.floor((scrollTop + clientHeight) / ROW_HEIGHT) + OVERSCAN);
  
  $: paddingTop = startIndex * ROW_HEIGHT;
  $: paddingBottom = Math.max(0, (totalPacketsCount - endIndex) * ROW_HEIGHT);

  // Fetch packets when the window changes
  let currentFetchId = 0;
  $: {
    const fetchId = ++currentFetchId;
    const offset = startIndex;
    const limit = Math.max(0, endIndex - startIndex);
    const filter = $debouncedFilter;
    
    if (limit > 0) {
      invoke<PacketSummary[]>('get_packets', { offset, limit, filter: filter || null })
        .then(packets => {
          if (fetchId === currentFetchId) {
            visiblePackets = packets;
          }
        })
        .catch(err => console.error('Failed to fetch packets:', err));
    } else {
      visiblePackets = [];
    }
  }

  // Update total count when filter changes
  $: {
    const filter = $debouncedFilter;
    invoke<number>('get_packet_count', { filter: filter || null })
      .then(count => {
        totalFilteredCount.set(count);
      })
      .catch(err => console.error('Failed to get count:', err));
  }

  onMount(() => {
    let unlistenFn: (() => void) | null = null;
    
    listen('new_packet_batch', (event: any) => {
      const newPackets = event.payload as PacketSummary[];
      // We still update statistics via addPackets, which also updates total count
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
    if (p === 'tcp') return 'text-[#569cd6]'; // VSCode blue
    if (p === 'udp') return 'text-[#d7ba7d]'; // VSCode yellow/gold
    if (p === 'icmp' || p === 'icmpv6') return 'text-[#b5cea8]'; // VSCode light green
    if (p === 'arp') return 'text-[#c586c0]'; // VSCode purple
    if (p === 'dns') return 'text-[#4ec9b0]'; // VSCode teal
    if (p === 'http' || p === 'https') return 'text-[#9cdcfe]'; // VSCode light blue
    return 'text-[#dcdcaa]'; // default
  }
</script>

<div class="flex-1 overflow-y-auto overflow-x-auto bg-[#1e1e1e] h-full relative" on:scroll={handleScroll} bind:clientHeight>
  <table class="w-full border-collapse font-mono text-[0.85rem] min-w-max">
    <thead class="sticky top-0 bg-[#252526] z-10 shadow-sm border-b border-[#3e3e3e]">
      <tr>
        <th class="w-[80px] min-w-[80px] px-3 py-2 text-left text-[#999] font-medium text-xs tracking-wider uppercase">No.</th>
        <th class="w-[140px] min-w-[140px] px-3 py-2 text-left text-[#999] font-medium text-xs tracking-wider uppercase">Time</th>
        <th class="w-[200px] min-w-[150px] px-3 py-2 text-left text-[#999] font-medium text-xs tracking-wider uppercase">Source</th>
        <th class="w-[200px] min-w-[150px] px-3 py-2 text-left text-[#999] font-medium text-xs tracking-wider uppercase">Destination</th>
        <th class="w-[100px] min-w-[100px] px-3 py-2 text-left text-[#999] font-medium text-xs tracking-wider uppercase">Protocol</th>
        <th class="w-[80px] min-w-[80px] px-3 py-2 text-left text-[#999] font-medium text-xs tracking-wider uppercase">Length</th>
        <th class="min-w-[300px] px-3 py-2 text-left text-[#999] font-medium text-xs tracking-wider uppercase">Info</th>
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
          class="cursor-pointer bg-[#1e1e1e] hover:bg-[#2a2d2e] {selectedId === packet.id ? '!bg-[#094771]' : ''} group"
          on:click={() => selectPacket(packet)}
        >
          <td class="px-3 py-0 text-[#888] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[#ccc]">{packet.id}</td>
          <td class="px-3 py-0 text-[#888] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[#ccc]">{formatTimestamp(packet.timestamp)}</td>
          <td class="px-3 py-0 text-[#dcdcaa] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-medium">{packet.source_addr}</td>
          <td class="px-3 py-0 text-[#dcdcaa] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-medium">{packet.dest_addr}</td>
          <td class="px-3 py-0 border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-bold {colorClasses.split(' ')[0]}">{packet.protocol}</td>
          <td class="px-3 py-0 text-[#888] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[#ccc]">{packet.length}</td>
          <td class="px-3 py-0 text-[#ce9178] border-b border-[#2d2d2d] h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]" title={packet.info}>{packet.info}</td>
        </tr>
      {/each}
      
      {#if paddingBottom > 0}
        <tr style="height: {paddingBottom}px;">
          <td colspan="7" class="p-0 border-none"></td>
        </tr>
      {/if}
    </tbody>
  </table>
  
  {#if totalPacketsCount === 0}
    <div class="absolute top-[50px] left-0 right-0 p-8 text-center text-[#888] italic">
      {#if $totalFilteredCount > 0}
        No packets match the current filter.
      {:else}
        No packets captured yet. Click "Start" to begin capturing.
      {/if}
    </div>
  {/if}
</div>
