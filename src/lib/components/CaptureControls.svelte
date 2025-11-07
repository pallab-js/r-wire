<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { listen } from '@tauri-apps/api/event';
  import { 
    availableInterfaces, 
    selectedInterface, 
    isCapturing, 
    captureError,
    packetList,
    selectedPacket,
    displayFilter,
    setPacketList
  } from '../stores';
  import { onMount } from 'svelte';

  let interfaces: string[] = [];

  onMount(async () => {
    try {
      const result = await invoke<string[]>('list_interfaces');
      availableInterfaces.set(result);
      interfaces = result;
    } catch (error) {
      console.error('Failed to list interfaces:', error);
      captureError.set('Failed to list network interfaces');
    }
  });

  async function startCapture() {
    const interfaceName = $selectedInterface;
    if (!interfaceName) {
      captureError.set('Please select a network interface');
      return;
    }

    try {
      captureError.set(null);
      await invoke('start_capture', { interfaceName });
      isCapturing.set(true);
    } catch (error: any) {
      const errorMsg = error.toString();
      if (errorMsg.includes('PermissionError') || errorMsg.includes('Permission denied')) {
        captureError.set('Permission denied. Please run with administrator/sudo privileges.');
      } else {
        captureError.set(`Failed to start capture: ${errorMsg}`);
      }
      isCapturing.set(false);
    }
  }

  async function stopCapture() {
    try {
      await invoke('stop_capture');
      isCapturing.set(false);
    } catch (error: any) {
      captureError.set(`Failed to stop capture: ${error}`);
    }
  }

  async function restartCapture() {
    // Clear packet list and selected packet
    setPacketList([]);
    selectedPacket.set(null);
    
    // Stop and start
    await stopCapture();
    // Small delay to ensure stop completes
    setTimeout(() => {
      startCapture();
    }, 100);
  }

  function clearPackets() {
    setPacketList([]);
    selectedPacket.set(null);
  }

  async function exportPcap() {
    if ($packetList.length === 0) {
      captureError.set('No packets to export');
      return;
    }

    try {
      const { save } = await import('@tauri-apps/api/dialog');
      const { invoke } = await import('@tauri-apps/api/tauri');
      
      const filePath = await save({
        title: 'Export PCAP File',
        defaultPath: 'capture.pcap',
        filters: [{
          name: 'PCAP Files',
          extensions: ['pcap']
        }]
      });

      if (filePath) {
        const packetIds = $packetList.map(p => p.id);
        const exportedCount = await invoke<number>('export_pcap', { 
          filePath, 
          packetIds 
        });
        captureError.set(null);
        // Show success message briefly
        const successMsg = `PCAP exported successfully: ${exportedCount} packets`;
        captureError.set(successMsg);
        setTimeout(() => captureError.set(null), 3000);
      }
    } catch (error: any) {
      const errorMsg = error.toString();
      if (!errorMsg.includes('User cancelled') && !errorMsg.includes('User canceled')) {
        captureError.set(`Failed to export PCAP: ${errorMsg}`);
      }
    }
  }
</script>

<div class="controls">
  <div class="control-group">
    <label for="interface-select">Network Interface:</label>
    <select 
      id="interface-select"
      bind:value={$selectedInterface}
      disabled={$isCapturing}
    >
      <option value={null}>Select an interface...</option>
      {#each $availableInterfaces as iface}
        <option value={iface}>{iface}</option>
      {/each}
    </select>
  </div>

  <div class="button-group">
    <button 
      on:click={startCapture}
      disabled={!$selectedInterface || $isCapturing}
      class="btn btn-primary"
    >
      Start
    </button>
    <button 
      on:click={stopCapture}
      disabled={!$isCapturing}
      class="btn btn-secondary"
    >
      Stop
    </button>
    <button 
      on:click={restartCapture}
      disabled={!$selectedInterface || !$isCapturing}
      class="btn btn-secondary"
    >
      Restart
    </button>
    <button 
      on:click={clearPackets}
      disabled={$packetList.length === 0}
      class="btn btn-secondary"
      title="Clear packet list"
    >
      Clear
    </button>
    <button 
      on:click={exportPcap}
      disabled={$packetList.length === 0}
      class="btn btn-secondary"
      title="Export to PCAP file"
    >
      Export PCAP
    </button>
  </div>

  <div class="filter-group">
    <label for="filter-input">Filter:</label>
    <input 
      id="filter-input"
      type="text"
      placeholder="protocol:tcp, ip:192.168, port:80, or search..."
      bind:value={$displayFilter}
      class="filter-input"
    />
    {#if $displayFilter}
      <button 
        class="clear-filter-btn"
        on:click={() => displayFilter.set('')}
        title="Clear filter"
      >
        ×
      </button>
    {/if}
  </div>

  <div class="stats">
    <span class="packet-count">Packets: <strong>{$packetList.length.toLocaleString()}</strong></span>
  </div>

  {#if $captureError}
    <div class="error-alert" class:success={$captureError.startsWith('PCAP exported')}>
      <span>{$captureError}</span>
      <button class="dismiss-btn" on:click={() => captureError.set(null)}>×</button>
    </div>
  {/if}
</div>

<style>
  .controls {
    padding: 1rem;
    background: #1e1e1e;
    border-bottom: 1px solid #333;
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .control-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  label {
    color: #ccc;
    font-size: 0.9rem;
  }

  select {
    padding: 0.5rem;
    background: #2d2d2d;
    color: #fff;
    border: 1px solid #444;
    border-radius: 4px;
    font-size: 0.9rem;
    min-width: 200px;
  }

  select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .button-group {
    display: flex;
    gap: 0.5rem;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 500;
    transition: background 0.2s;
  }

  .btn-primary {
    background: #007acc;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #005a9e;
  }

  .btn-secondary {
    background: #3e3e3e;
    color: white;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #4e4e4e;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-alert {
    background: #5a1a1a;
    color: #ff6b6b;
    padding: 0.75rem 1rem;
    border-radius: 4px;
    border: 1px solid #7a2a2a;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex: 1;
    max-width: 500px;
  }

  .error-alert.success {
    background: #1a5a1a;
    color: #6bff6b;
    border-color: #2a7a2a;
  }

  .dismiss-btn {
    background: transparent;
    border: none;
    color: #ff6b6b;
    font-size: 1.5rem;
    cursor: pointer;
    padding: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  .error-alert.success .dismiss-btn {
    color: #6bff6b;
  }

  .dismiss-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 50%;
  }

  .stats {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .packet-count {
    color: #ccc;
    font-size: 0.9rem;
    padding: 0.5rem 1rem;
    background: #2d2d2d;
    border-radius: 4px;
    border: 1px solid #444;
  }

  .packet-count strong {
    color: #4ec9b0;
    font-weight: 600;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    max-width: 400px;
  }

  .filter-group label {
    color: #ccc;
    font-size: 0.9rem;
    white-space: nowrap;
  }

  .filter-input {
    flex: 1;
    padding: 0.5rem;
    background: #2d2d2d;
    color: #fff;
    border: 1px solid #444;
    border-radius: 4px;
    font-size: 0.9rem;
    font-family: 'Courier New', monospace;
  }

  .filter-input:focus {
    outline: none;
    border-color: #007acc;
  }

  .filter-input::placeholder {
    color: #666;
  }

  .clear-filter-btn {
    background: transparent;
    border: none;
    color: #ccc;
    font-size: 1.2rem;
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    line-height: 1;
    border-radius: 4px;
    transition: background 0.2s;
  }

  .clear-filter-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
</style>
