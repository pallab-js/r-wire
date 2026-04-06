<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/tauri';
  import { selectedPacket, selectedStream, captureError, addPackets, totalFilteredCount, debouncedFilter, packetList, type PacketSummary, type PacketDetail, type StreamMessage } from '../stores';
  import { onMount } from 'svelte';

  let selectedId: number | null = null;
  
  // Context Menu state
  let contextMenuVisible = false;
  let contextMenuPos = { x: 0, y: 0 };
  let contextMenuPacketId: number | null = null;
  let contextMenuProtocol: string | null = null;

  // Derive if current context menu packet is a stream-capable protocol
  $: isStream = contextMenuProtocol?.toLowerCase() === 'tcp' || contextMenuProtocol?.toLowerCase() === 'udp';

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

  async function handleContextMenu(e: MouseEvent, packet: PacketSummary) {
    e.preventDefault();
    contextMenuVisible = true;
    contextMenuPos = { x: e.clientX, y: e.clientY };
    contextMenuPacketId = packet.id;
    contextMenuProtocol = packet.protocol;
  }

  function closeContextMenu() {
    contextMenuVisible = false;
  }

  async function followStream() {
    if (contextMenuPacketId === null) return;
    
    // Only allow for TCP/UDP
    if (!isStream) {
      captureError.set("Follow Stream is only supported for TCP and UDP traffic.");
      setTimeout(() => captureError.set(null), 3000);
      closeContextMenu();
      return;
    }

    try {
      captureError.set("Reassembling stream...");
      const messages = await invoke<StreamMessage[]>('get_stream_content', { packetId: contextMenuPacketId });
      if (messages && messages.length > 0) {
        selectedStream.set(messages);
        captureError.set(null);
      } else {
        captureError.set("No conversational data found for this packet.");
        setTimeout(() => captureError.set(null), 3000);
      }
    } catch (err) {
      console.error('Failed to follow stream:', err);
      captureError.set(`Error reassembling stream: ${err}`);
    }
    closeContextMenu();
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
    if (p === 'tcp') return 'text-[var(--color-read)]';
    if (p === 'udp') return 'text-[var(--color-thinking)]';
    if (p === 'icmp' || p === 'icmpv6') return 'text-[var(--color-success)]';
    if (p === 'arp') return 'text-[var(--color-edit)]';
    if (p === 'dns') return 'text-[var(--color-success)]';
    if (p === 'http' || p === 'https') return 'text-[var(--cursor-dark)]';
    return 'text-[rgba(38,37,30,0.55)]';
  }
</script>

<div
  class="flex-1 overflow-y-auto overflow-x-auto h-full relative"
  style="background-color: var(--cursor-cream);"
  on:scroll={handleScroll}
  on:click={closeContextMenu}
  on:keydown={(e) => e.key === 'Escape' && closeContextMenu()}
  bind:clientHeight
  role="grid"
  aria-label="Packet list"
  tabindex="0"
>
  <table class="w-full border-collapse text-sm min-w-max" style="font-family: var(--font-mono);">
    <thead class="sticky top-0 z-10 shadow-sm" style="background-color: var(--surface-200); border-bottom: 1px solid var(--border-primary);">
      <tr>
        <th class="w-[80px] min-w-[80px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase" style="color: rgba(38, 37, 30, 0.55);">No.</th>
        <th class="w-[140px] min-w-[140px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase" style="color: rgba(38, 37, 30, 0.55);">Time</th>
        <th class="w-[200px] min-w-[150px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase" style="color: rgba(38, 37, 30, 0.55);">Source</th>
        <th class="w-[200px] min-w-[150px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase" style="color: rgba(38, 37, 30, 0.55);">Destination</th>
        <th class="w-[100px] min-w-[100px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase" style="color: rgba(38, 37, 30, 0.55);">Protocol</th>
        <th class="w-[80px] min-w-[80px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase" style="color: rgba(38, 37, 30, 0.55);">Length</th>
        <th class="min-w-[300px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase" style="color: rgba(38, 37, 30, 0.55);">Info</th>
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
          class="cursor-pointer hover:bg-[var(--surface-300)] group"
          style="background-color: {selectedId === packet.id ? 'var(--surface-400)' : 'var(--cursor-cream)'};"
          on:click={() => selectPacket(packet)}
          on:contextmenu={(e) => handleContextMenu(e, packet)}
        >
          <td class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[var(--cursor-dark)]" style="color: rgba(38, 37, 30, 0.55); border-color: var(--border-primary);">{packet.id}</td>
          <td class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[var(--cursor-dark)]" style="color: rgba(38, 37, 30, 0.55); border-color: var(--border-primary);">{formatTimestamp(packet.timestamp)}</td>
          <td class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-medium" style="color: rgba(38, 37, 30, 0.55); border-color: var(--border-primary);">{packet.source_addr}</td>
          <td class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-medium" style="color: rgba(38, 37, 30, 0.55); border-color: var(--border-primary);">{packet.dest_addr}</td>
          <td class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-bold {colorClasses.split(' ')[0]}" style="border-color: var(--border-primary);">{packet.protocol}</td>
          <td class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[var(--cursor-dark)]" style="color: rgba(38, 37, 30, 0.55); border-color: var(--border-primary);">{packet.length}</td>
          <td class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]" style="color: rgba(38, 37, 30, 0.55); border-color: var(--border-primary);" title={packet.info}>{packet.info}</td>
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
    <div class="absolute top-[50px] left-0 right-0 p-8 text-center italic" style="color: rgba(38, 37, 30, 0.55);">
      {#if $totalFilteredCount > 0}
        No packets match the current filter.
      {:else}
        No packets captured yet. Click "Start" to begin capturing.
      {/if}
    </div>
  {/if}

  <!-- Context Menu -->
  {#if contextMenuVisible}
    <div
      class="fixed z-[200] shadow-xl rounded py-1 min-w-[160px]"
      style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px; background-color: var(--surface-200); border: 1px solid var(--border-primary); font-family: var(--font-mono);"
      on:click|stopPropagation
      on:keydown={(e) => e.key === 'Escape' && closeContextMenu()}
      role="menu"
      aria-label="Packet context menu"
      tabindex="-1"
    >
      <button
        on:click={followStream}
        disabled={!isStream}
        class="w-full text-left px-4 py-2 hover:not-disabled:bg-[var(--surface-300)] hover:not-disabled:text-[var(--cursor-dark)] bg-transparent border-none text-sm flex items-center gap-2 cursor-pointer disabled:cursor-not-allowed transition-colors"
        style="color: {isStream ? 'var(--cursor-dark)' : 'rgba(38, 37, 30, 0.3)'};"
        title={isStream ? "Follow Stream" : "Follow Stream (TCP/UDP only)"}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M7 11V7a5 5 0 0 1 10 0v4"/><rect x="3" y="11" width="18" height="11" rx="2"/><circle cx="12" cy="16" r="2"/></svg>
        Follow Stream
      </button>
    </div>
  {/if}
</div>
