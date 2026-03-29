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
  let intensityActive = false;
  let intensityTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    try {
      const result = await invoke<string[]>('list_interfaces');
      availableInterfaces.set(result);
      interfaces = result;
    } catch (error) {
      console.error('Failed to list interfaces:', error);
      captureError.set('Failed to list network interfaces');
    }

    // Listen for batches to trigger intensity pulse
    const unlisten = await listen('new_packet_batch', () => {
      intensityActive = true;
      if (intensityTimer) clearTimeout(intensityTimer);
      intensityTimer = setTimeout(() => {
        intensityActive = false;
      }, 150);
    });

    return () => {
      unlisten();
      if (intensityTimer) clearTimeout(intensityTimer);
    };
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
    setPacketList([]);
    selectedPacket.set(null);
    await stopCapture();
    await new Promise(resolve => setTimeout(resolve, 100));
    await startCapture();
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

<div class="px-4 py-3 bg-[#252526] border-b border-[#3e3e3e] flex items-center gap-5 flex-wrap shadow-[0_2px_8px_rgba(0,0,0,0.2)] relative z-20">
  <div class="flex items-center gap-3">
    <div class="w-2.5 h-2.5 rounded-full transition-all duration-300 {intensityActive ? 'scale-150 bg-white shadow-[0_0_12px_#fff]' : ($isCapturing ? 'bg-[#4ec9b0] shadow-[0_0_8px_rgba(78,201,176,0.4)]' : 'bg-[#444]')}"></div>
    <label for="interface-select" class="text-[#888] text-[0.85rem] font-semibold uppercase tracking-wider">Interface:</label>
    <select 
      id="interface-select"
      bind:value={$selectedInterface}
      disabled={$isCapturing}
      class="px-2.5 py-1.5 bg-[#1e1e1e] text-[#ccc] border border-[#444] rounded text-sm min-w-[160px] outline-none focus:border-[#007acc] disabled:opacity-50 disabled:cursor-not-allowed"
    >
      <option value={null}>Select Device...</option>
      {#each $availableInterfaces as iface}
        <option value={iface}>{iface}</option>
      {/each}
    </select>
  </div>

  <div class="flex items-center gap-2">
    <button 
      on:click={startCapture}
      disabled={!$selectedInterface || $isCapturing}
      class="px-3 py-1.5 border-none rounded cursor-pointer text-[0.85rem] font-semibold transition-all duration-200 flex items-center gap-1.5 bg-[#007acc] text-white hover:not-disabled:bg-[#005a9e] hover:not-disabled:-translate-y-px disabled:opacity-40 disabled:cursor-not-allowed"
      title="Start Capture"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
      Start
    </button>
    <button 
      on:click={stopCapture}
      disabled={!$isCapturing}
      class="px-3 py-1.5 border-none rounded cursor-pointer text-[0.85rem] font-semibold transition-all duration-200 flex items-center gap-1.5 bg-[#333] text-[#ccc] hover:not-disabled:bg-[#444] hover:not-disabled:text-white disabled:opacity-40 disabled:cursor-not-allowed"
      title="Stop Capture"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h12v12H6z"/></svg>
      Stop
    </button>
    <button 
      on:click={restartCapture}
      disabled={!$selectedInterface || !$isCapturing}
      class="px-3 py-1.5 border-none rounded cursor-pointer text-[0.85rem] font-semibold transition-all duration-200 flex items-center gap-1.5 bg-[#333] text-[#ccc] hover:not-disabled:bg-[#444] hover:not-disabled:text-white disabled:opacity-40 disabled:cursor-not-allowed"
      title="Restart Capture"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M23 4v6h-6"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
    </button>
    <div class="w-px h-6 bg-[#3e3e3e] mx-1"></div>
    <button 
      on:click={clearPackets}
      disabled={$packetList.length === 0}
      class="px-3 py-1.5 border-none rounded cursor-pointer text-[0.85rem] font-semibold transition-all duration-200 flex items-center gap-1.5 bg-[#333] text-[#ccc] hover:not-disabled:bg-[#444] hover:not-disabled:text-white disabled:opacity-40 disabled:cursor-not-allowed"
      title="Clear packet list"
    >
      Clear
    </button>
    <button 
      on:click={exportPcap}
      disabled={$packetList.length === 0}
      class="px-3 py-1.5 border-none rounded cursor-pointer text-[0.85rem] font-semibold transition-all duration-200 flex items-center gap-1.5 bg-[#333] text-[#ccc] hover:not-disabled:bg-[#444] hover:not-disabled:text-white disabled:opacity-40 disabled:cursor-not-allowed"
      title="Export to PCAP file"
    >
      Export PCAP
    </button>
  </div>

  <div class="flex items-center bg-[#1e1e1e] border border-[#444] rounded flex-1 max-w-[450px] px-2 transition-colors duration-200 focus-within:border-[#007acc]">
    <div class="text-[#666] flex items-center">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
    </div>
    <input 
      id="filter-input"
      type="text"
      placeholder="Filter (e.g. protocol:tcp, ip:192.168, port:80)..."
      bind:value={$displayFilter}
      class="flex-1 p-2 bg-transparent text-white border-none text-sm font-sans outline-none placeholder-[#666]"
    />
    {#if $displayFilter}
      <button 
        class="bg-transparent border-none text-[#888] text-lg cursor-pointer px-1 hover:text-white"
        on:click={() => displayFilter.set('')}
        title="Clear filter"
      >
        ×
      </button>
    {/if}
  </div>

  <div class="ml-auto">
    <span class="text-[#888] text-[0.85rem] px-3 py-1.5 bg-[#1e1e1e] rounded border {$isCapturing ? 'border-[#4ec9b0]' : 'border-[#3e3e3e]'} flex items-center gap-2">
      <span class="uppercase font-semibold text-[0.75rem]">Packets</span>
      <strong class="text-[#4ec9b0] font-mono text-base">{$packetList.length.toLocaleString()}</strong>
    </span>
  </div>

  {#if $captureError}
    <div class="fixed top-[60px] right-5 bg-{$captureError.startsWith('PCAP exported') ? '[#1a5a1a]' : '[#5a1a1a]'} text-{$captureError.startsWith('PCAP exported') ? '[#6bff6b]' : '[#ff6b6b]'} px-4 py-3 rounded-md border border-{$captureError.startsWith('PCAP exported') ? '[#2a7a2a]' : '[#7a2a2a]'} flex items-center gap-4 z-[1000] shadow-[0_4px_12px_rgba(0,0,0,0.3)] animate-[slideIn_0.3s_ease-out]">
      <span>{$captureError}</span>
      <button class="bg-transparent border-none text-current text-xl cursor-pointer opacity-70 hover:opacity-100" on:click={() => captureError.set(null)}>×</button>
    </div>
  {/if}
</div>

<style>
  @keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }
</style>
