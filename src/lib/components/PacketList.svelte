<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/tauri';
  import {
    selectedPacket,
    selectedStream,
    captureError,
    addPackets,
    totalFilteredCount,
    debouncedFilter,
    type PacketSummary,
    type PacketDetail,
    type StreamMessage,
  } from '../stores';
  import { onMount } from 'svelte';

  let selectedId: number | null = null;

  // Context Menu state
  let contextMenuVisible = false;
  let contextMenuPos = { x: 0, y: 0 };
  let contextMenuPacketId: number | null = null;
  let contextMenuProtocol: string | null = null;

  // Derive if current context menu packet is a stream-capable protocol
  $: isStream =
    contextMenuProtocol?.toLowerCase() === 'tcp' || contextMenuProtocol?.toLowerCase() === 'udp';

  // Local cache for current window of packets
  let visiblePackets: PacketSummary[] = [];
  // Scroll offset that `visiblePackets` was fetched for. The spacers are
  // derived from this rather than from the requested window, so rows are never
  // drawn at a position that disagrees with their packet id while a fetch is
  // still in flight (or after one failed).
  let loadedOffset = 0;

  // Timestamp formatting cache
  const timestampCache = new Map<number, string>();

  // Virtual Scrolling State
  let scrollTop = 0;
  let clientHeight = 600;
  const ROW_HEIGHT = 28; // Fixed height per row
  const OVERSCAN = 30; // Render 30 rows above/below
  const FETCH_DEBOUNCE_MS = 50;

  $: totalPacketsCount = $totalFilteredCount;
  $: startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  $: endIndex = Math.min(
    totalPacketsCount,
    Math.floor((scrollTop + clientHeight) / ROW_HEIGHT) + OVERSCAN,
  );

  $: renderedCount = visiblePackets.length;
  $: paddingTop = loadedOffset * ROW_HEIGHT;
  $: paddingBottom = Math.max(0, (totalPacketsCount - loadedOffset - renderedCount) * ROW_HEIGHT);

  // Fetch packets when the window changes
  let currentFetchId = 0;
  let fetchTimer: ReturnType<typeof setTimeout> | undefined;

  $: scheduleFetch(startIndex, endIndex, $debouncedFilter);

  function scheduleFetch(offset: number, end: number, filter: string) {
    const limit = Math.max(0, end - offset);
    clearTimeout(fetchTimer);

    if (limit <= 0) {
      fetchTimer = setTimeout(() => loadWindow(offset, limit, filter), 0);
      return;
    }

    // The cached rows already contain everything on screen — scrolling inside
    // the overscanned window costs no query at all.
    if (
      renderedCount > 0 &&
      offset >= loadedOffset &&
      offset + limit <= loadedOffset + renderedCount
    ) {
      return;
    }

    // Query straight away once the cached rows no longer intersect the
    // viewport (otherwise the list would sit blank); otherwise wait for
    // scrolling to settle. One IPC call per scroll event adds up fast — a
    // single wheel flick fires dozens of them.
    const isStale =
      renderedCount === 0 ||
      offset + limit <= loadedOffset ||
      offset >= loadedOffset + renderedCount;

    fetchTimer = setTimeout(
      () => loadWindow(offset, limit, filter),
      isStale ? 0 : FETCH_DEBOUNCE_MS,
    );
  }

  function loadWindow(offset: number, limit: number, filter: string) {
    if (limit <= 0) {
      visiblePackets = [];
      loadedOffset = 0;
      return;
    }

    const fetchId = ++currentFetchId;
    invoke<PacketSummary[]>('get_packets', { offset, limit, filter: filter || null })
      .then((packets) => {
        // A newer window superseded this request while it was in flight.
        if (fetchId !== currentFetchId) return;
        visiblePackets = packets;
        loadedOffset = offset;
      })
      .catch((err) => {
        // Keep the previous rows: the spacers still match `loadedOffset`, so
        // what stays on screen remains correctly positioned.
        console.error('Failed to fetch packets:', err);
      });
  }

  // Update total count when filter changes. Responses are not ordered, so a
  // slow answer for a previous filter must not overwrite the current one:
  // `totalPacketsCount` drives the virtual scroller's height, and a stale value
  // leaves rows rendered at the wrong scroll offset until the next change.
  let currentCountId = 0;
  $: {
    const countId = ++currentCountId;
    const filter = $debouncedFilter;
    invoke<number>('get_packet_count', { filter: filter || null })
      .then((count) => {
        if (countId !== currentCountId) return; // superseded by a newer filter
        totalFilteredCount.set(count);
      })
      .catch((err) => console.error('Failed to get count:', err));
  }

  onMount(() => {
    let unlistenFn: (() => void) | null = null;

    listen('new_packet_batch', (event) => {
      const newPackets = event.payload as PacketSummary[];
      addPackets(newPackets);
    }).then((fn) => {
      unlistenFn = fn;
    });

    return () => {
      if (unlistenFn) {
        unlistenFn();
      }
    };
  });

  // Detail requests are not ordered: clicking packet 1 then packet 2 must never
  // let packet 1's slower response land on top of packet 2's, or the detail pane
  // describes a packet that is no longer the selected row.
  let currentDetailId = 0;
  async function selectPacket(packet: PacketSummary) {
    selectedId = packet.id;
    const detailId = ++currentDetailId;
    try {
      const detail = await invoke<PacketDetail>('get_packet_detail', { id: packet.id });
      if (detailId !== currentDetailId) return; // a newer packet was clicked
      if (detail) {
        selectedPacket.set(detail);
      }
    } catch (error) {
      // Drop the detail rather than keep showing the previous packet's, but only
      // if this is still the request on screen.
      if (detailId === currentDetailId) selectedPacket.set(null);
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

  // Sequence number for Follow Stream requests: see `followStream`.
  let currentStreamId = 0;

  // A transient message clears itself, but only if it is still the message on
  // screen: blindly setting `null` three seconds later can wipe a newer, more
  // serious message (a real capture failure, say) that arrived in between.
  function clearLater(message: string, ms = 3000) {
    setTimeout(() => {
      captureError.update((current) => (current === message ? null : current));
    }, ms);
  }

  async function followStream() {
    if (contextMenuPacketId === null) return;

    // Only allow for TCP/UDP
    if (!isStream) {
      const msg = 'Follow Stream is only supported for TCP and UDP traffic.';
      captureError.set(msg);
      clearLater(msg);
      closeContextMenu();
      return;
    }

    // Snapshot the id and start a new sequence: a second Follow Stream while
    // this one is in flight supersedes it, so only the newest may publish.
    const packetId = contextMenuPacketId;
    const streamId = ++currentStreamId;
    try {
      captureError.set('Reassembling stream...');
      const messages = await invoke<StreamMessage[]>('get_stream_content', {
        packetId,
      });
      if (streamId !== currentStreamId) return; // superseded while in flight
      if (messages && messages.length > 0) {
        selectedStream.set(messages);
        captureError.set(null);
      } else {
        const msg = 'No conversational data found for this packet.';
        captureError.set(msg);
        clearLater(msg);
      }
    } catch (err) {
      if (streamId !== currentStreamId) return;
      console.error('Failed to follow stream:', err);
      const msg = `Error reassembling stream: ${err}`;
      captureError.set(msg);
      clearLater(msg); // this one used to stay on screen forever
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
        fractionalSecondDigits: 3,
      });

      if (timestampCache.size > 10000) {
        const firstKey = timestampCache.keys().next().value;
        if (firstKey !== undefined) timestampCache.delete(firstKey);
      }
      timestampCache.set(timestamp, formatted);
      return formatted;
    } catch {
      return '00:00:00.000';
    }
  }

  function handleScroll(e: Event) {
    scrollTop = (e.target as HTMLElement).scrollTop;
  }

  function getProtocolColor(protocol: string) {
    const p = protocol.toLowerCase();
    if (p === 'tcp') return 'text-blue-400';
    if (p === 'udp') return 'text-orange-400';
    if (p === 'icmp' || p === 'icmpv6') return 'text-[var(--brand-green)]';
    if (p === 'arp') return 'text-purple-400';
    if (p === 'dns') return 'text-[var(--brand-green)]';
    if (p === 'http' || p === 'https') return 'text-[var(--text-primary)]';
    return 'text-[var(--text-muted)]';
  }
</script>

<div
  class="flex-1 overflow-y-auto overflow-x-auto h-full relative"
  style="background-color: var(--bg-page);"
  on:scroll={handleScroll}
  on:click={closeContextMenu}
  on:keydown={(e) => e.key === 'Escape' && closeContextMenu()}
  bind:clientHeight
  role="grid"
  aria-label="Packet list"
  tabindex="0"
>
  <table class="w-full border-collapse text-sm min-w-max" style="font-family: var(--font-mono);">
    <thead
      class="sticky top-0 z-10"
      style="background-color: var(--border-standard); border-bottom: 1px solid var(--border-standard);"
    >
      <tr>
        <th
          class="w-[80px] min-w-[80px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase"
          style="color: var(--text-muted);">No.</th
        >
        <th
          class="w-[140px] min-w-[140px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase"
          style="color: var(--text-muted);">Time</th
        >
        <th
          class="w-[200px] min-w-[150px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase"
          style="color: var(--text-muted);">Source</th
        >
        <th
          class="w-[200px] min-w-[150px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase"
          style="color: var(--text-muted);">Destination</th
        >
        <th
          class="w-[100px] min-w-[100px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase"
          style="color: var(--text-muted);">Protocol</th
        >
        <th
          class="w-[80px] min-w-[80px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase"
          style="color: var(--text-muted);">Length</th
        >
        <th
          class="min-w-[300px] px-3 py-2 text-left font-medium text-xs tracking-wider uppercase"
          style="color: var(--text-muted);">Info</th
        >
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
          class="cursor-pointer hover:bg-[var(--border-standard)] group"
          style="background-color: {selectedId === packet.id
            ? 'var(--border-prominent)'
            : 'transparent'};"
          on:click={() => selectPacket(packet)}
          on:contextmenu={(e) => handleContextMenu(e, packet)}
        >
          <td
            class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[var(--text-primary)]"
            style="color: var(--text-muted); border-color: var(--border-subtle);">{packet.id}</td
          >
          <td
            class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[var(--text-primary)]"
            style="color: var(--text-muted); border-color: var(--border-subtle);"
            >{formatTimestamp(packet.timestamp)}</td
          >
          <td
            class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-medium"
            style="color: var(--text-secondary); border-color: var(--border-subtle);"
            >{packet.source_addr}</td
          >
          <td
            class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-medium"
            style="color: var(--text-secondary); border-color: var(--border-subtle);"
            >{packet.dest_addr}</td
          >
          <td
            class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] font-medium {colorClasses.split(
              ' ',
            )[0]}"
            style="border-color: var(--border-subtle);">{packet.protocol}</td
          >
          <td
            class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px] group-hover:text-[var(--text-primary)]"
            style="color: var(--text-muted); border-color: var(--border-subtle);"
            >{packet.length}</td
          >
          <td
            class="px-3 py-0 border-b h-[28px] whitespace-nowrap overflow-hidden text-ellipsis leading-[28px]"
            style="color: var(--text-muted); border-color: var(--border-subtle);"
            title={packet.info}>{packet.info}</td
          >
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
    <div
      class="absolute top-[50px] left-0 right-0 p-8 text-center"
      style="color: var(--text-muted);"
    >
      {#if $debouncedFilter.trim()}
        <!-- The outer check is `totalPacketsCount === 0`, and totalPacketsCount
             is just $totalFilteredCount — so a `$totalFilteredCount > 0` branch
             here could never render. Filter set + zero matches is the real
             "nothing to show" case. -->
        No packets match the current filter.
      {:else}
        No packets captured yet. Click "Start" to begin capturing.
      {/if}
    </div>
  {/if}

  <!-- Context Menu -->
  {#if contextMenuVisible}
    <div
      class="fixed z-[200] rounded py-1 min-w-[160px]"
      style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px; background-color: var(--bg-button); border: 1px solid var(--border-standard); font-family: var(--font-mono);"
      on:click|stopPropagation
      on:keydown={(e) => e.key === 'Escape' && closeContextMenu()}
      role="menu"
      aria-label="Packet context menu"
      tabindex="-1"
    >
      <button
        on:click={followStream}
        disabled={!isStream}
        class="w-full text-left px-4 py-2 hover:not-disabled:bg-[var(--border-standard)] hover:not-disabled:text-[var(--text-primary)] bg-transparent border-none text-sm flex items-center gap-2 cursor-pointer disabled:cursor-not-allowed transition-colors"
        style="color: {isStream ? 'var(--text-primary)' : 'var(--text-muted)'}; opacity: {isStream
          ? '1'
          : '0.5'};"
        title={isStream ? 'Follow Stream' : 'Follow Stream (TCP/UDP only)'}
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          ><path d="M7 11V7a5 5 0 0 1 10 0v4" /><rect
            x="3"
            y="11"
            width="18"
            height="11"
            rx="2"
          /><circle cx="12" cy="16" r="2" /></svg
        >
        Follow Stream
      </button>
    </div>
  {/if}
</div>
