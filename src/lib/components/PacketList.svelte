<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/tauri';
  import { selectedPacket, filteredPackets, addPackets, setPacketList, packetList, type PacketSummary, type PacketDetail } from '../stores';
  import { onMount } from 'svelte';

  let selectedId: number | null = null;
  
  // Use memoized filtered packets from store
  $: filteredPacketsList = $filteredPackets;
  
  // Timestamp formatting cache
  const timestampCache = new Map<number, string>();

  onMount(() => {
    let unlistenFn: (() => void) | null = null;
    
    listen('new_packet_batch', (event: any) => {
      const newPackets = event.payload as PacketSummary[];
      // Use optimized addPackets function with limit
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
      // Don't show error to user for individual packet failures
      // Just log it and keep previous selection
    }
  }

  function formatTimestamp(timestamp: number): string {
    // Handle invalid timestamps
    if (!timestamp || timestamp <= 0) {
      return '00:00:00.000';
    }
    
    // Check cache first
    if (timestampCache.has(timestamp)) {
      return timestampCache.get(timestamp)!;
    }
    
    try {
      const date = new Date(timestamp / 1000000); // Convert nanoseconds to milliseconds
      if (isNaN(date.getTime())) {
        return '00:00:00.000';
      }
      const formatted = date.toLocaleTimeString('en-US', { 
        hour12: false, 
        hour: '2-digit', 
        minute: '2-digit', 
        second: '2-digit',
        fractionalSecondDigits: 3
      });
      
      // Cache the result (limit cache size to prevent memory issues)
      if (timestampCache.size > 10000) {
        // Remove oldest entries (simple FIFO - remove first key)
        const firstKey = timestampCache.keys().next().value;
        if (firstKey !== undefined) {
          timestampCache.delete(firstKey);
        }
      }
      timestampCache.set(timestamp, formatted);
      return formatted;
    } catch (e) {
      return '00:00:00.000';
    }
  }
</script>

<div class="packet-list-container">
  <table class="packet-table">
    <thead>
      <tr>
        <th>No.</th>
        <th>Time</th>
        <th>Source</th>
        <th>Destination</th>
        <th>Protocol</th>
        <th>Length</th>
        <th>Info</th>
      </tr>
    </thead>
    <tbody>
      {#each filteredPacketsList as packet (packet.id)}
        <tr 
          class:selected={selectedId === packet.id}
          class:protocol-tcp={packet.protocol === 'TCP'}
          class:protocol-udp={packet.protocol === 'UDP'}
          class:protocol-icmp={packet.protocol === 'ICMP' || packet.protocol === 'ICMPv6'}
          class:protocol-arp={packet.protocol === 'ARP'}
          class:protocol-dns={packet.protocol === 'DNS'}
          class:protocol-http={packet.protocol === 'HTTP' || packet.protocol === 'HTTPS'}
          on:click={() => selectPacket(packet)}
        >
          <td>{packet.id}</td>
          <td>{formatTimestamp(packet.timestamp)}</td>
          <td class="address">{packet.source_addr}</td>
          <td class="address">{packet.dest_addr}</td>
          <td class="protocol protocol-{packet.protocol.toLowerCase()}">{packet.protocol}</td>
          <td>{packet.length}</td>
          <td class="info">{packet.info}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if filteredPacketsList.length === 0}
    <div class="empty-state">
      {#if $packetList.length > 0}
        No packets match the current filter. ({$packetList.length.toLocaleString()} total packets)
      {:else}
        No packets captured yet. Click "Start" to begin capturing.
      {/if}
    </div>
  {/if}
</div>

<style>
  .packet-list-container {
    flex: 1;
    overflow: auto;
    background: #1e1e1e;
  }

  .packet-table {
    width: 100%;
    border-collapse: collapse;
    font-family: 'Courier New', monospace;
    font-size: 0.85rem;
  }

  thead {
    position: sticky;
    top: 0;
    background: #252526;
    z-index: 10;
  }

  th {
    padding: 0.5rem;
    text-align: left;
    color: #ccc;
    font-weight: 600;
    border-bottom: 2px solid #3e3e3e;
    background: #252526;
  }

  td {
    padding: 0.4rem 0.5rem;
    color: #d4d4d4;
    border-bottom: 1px solid #2d2d2d;
  }

  tbody tr {
    cursor: pointer;
    transition: background 0.1s;
  }

  tbody tr:hover {
    background: #2a2a2a;
  }

  tbody tr.selected {
    background: #094771;
  }

  tbody tr.selected:hover {
    background: #0a5a8a;
  }

  /* Protocol-based row coloring */
  tbody tr.protocol-tcp {
    border-left: 3px solid #4a9eff;
  }

  tbody tr.protocol-udp {
    border-left: 3px solid #ffa500;
  }

  tbody tr.protocol-icmp {
    border-left: 3px solid #00ff00;
  }

  tbody tr.protocol-arp {
    border-left: 3px solid #ff6b6b;
  }

  tbody tr.protocol-dns {
    border-left: 3px solid #9b59b6;
  }

  tbody tr.protocol-http {
    border-left: 3px solid #3498db;
  }

  .address {
    color: #4ec9b0;
  }

  .protocol {
    color: #dcdcaa;
    font-weight: 600;
  }

  /* Protocol-specific colors */
  .protocol-tcp {
    color: #4a9eff;
  }

  .protocol-udp {
    color: #ffa500;
  }

  .protocol-icmp {
    color: #00ff00;
  }

  .protocol-arp {
    color: #ff6b6b;
  }

  .protocol-dns {
    color: #9b59b6;
  }

  .protocol-http {
    color: #3498db;
  }

  .protocol-https {
    color: #2ecc71;
  }

  .info {
    color: #ce9178;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: #888;
    font-style: italic;
  }
</style>
