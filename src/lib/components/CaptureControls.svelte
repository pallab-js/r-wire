<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { listen } from '@tauri-apps/api/event';
  import {
    availableInterfaces,
    selectedInterface,
    isCapturing,
    captureError,
    selectedPacket,
    displayFilter,
    bpfFilter,
    setPacketList,
    totalFilteredCount,
  } from '../stores';
  import { onMount } from 'svelte';

  let intensityActive = false;
  let intensityTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    invoke<string[]>('list_interfaces')
      .then((result) => {
        availableInterfaces.set(result);
      })
      .catch((error) => {
        console.error('Failed to list interfaces:', error);
        captureError.set('Failed to list network interfaces');
      });

    let unlistenFn: (() => void) | null = null;
    // Listen for batches to trigger intensity pulse
    listen('new_packet_batch', () => {
      intensityActive = true;
      if (intensityTimer) clearTimeout(intensityTimer);
      intensityTimer = setTimeout(() => {
        intensityActive = false;
      }, 150);
    }).then((fn) => {
      unlistenFn = fn;
    });

    return () => {
      if (unlistenFn) unlistenFn();
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
      const filter = $bpfFilter;
      await invoke('start_capture', { interfaceName, filter: filter || null });
      isCapturing.set(true);
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
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
    } catch (error) {
      captureError.set(`Failed to stop capture: ${error}`);
    }
  }

  async function restartCapture() {
    setPacketList([]);
    selectedPacket.set(null);
    await stopCapture();
    await new Promise((resolve) => setTimeout(resolve, 100));
    await startCapture();
  }

  function clearPackets() {
    setPacketList([]);
    selectedPacket.set(null);
  }

  async function exportPcap() {
    if ($totalFilteredCount === 0) {
      captureError.set('No packets to export');
      return;
    }

    try {
      const { save } = await import('@tauri-apps/api/dialog');
      const { invoke } = await import('@tauri-apps/api/tauri');

      const filePath = await save({
        title: 'Export PCAP File',
        defaultPath: 'capture.pcap',
        filters: [
          {
            name: 'PCAP Files',
            extensions: ['pcap'],
          },
        ],
      });

      if (filePath) {
        captureError.set('Exporting all captured packets...');
        const exportedCount = await invoke<number>('export_pcap_all', {
          filePath,
        });
        captureError.set(null);
        const successMsg = `PCAP exported successfully: ${exportedCount} packets`;
        captureError.set(successMsg);
        setTimeout(() => captureError.set(null), 3000);
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      if (!errorMsg.includes('User cancelled') && !errorMsg.includes('User canceled')) {
        captureError.set(`Failed to export PCAP: ${errorMsg}`);
      }
    }
  }
</script>

<div
  class="px-4 py-3 flex items-center gap-5 flex-wrap relative z-20"
  style="background-color: var(--surface-100); border-bottom: 1px solid var(--border-primary);"
>
  <div class="flex items-center gap-3">
    <div
      class="w-2.5 h-2.5 rounded-full transition-all duration-300 {intensityActive
        ? 'scale-150'
        : $isCapturing
          ? 'bg-[var(--color-success)]'
          : 'bg-[var(--border-medium)]'}"
    ></div>
    <label for="interface-select" class="text-micro">Interface:</label>
    <select
      id="interface-select"
      bind:value={$selectedInterface}
      disabled={$isCapturing}
      class="cursor-input text-sm min-w-[140px] disabled:opacity-50 disabled:cursor-not-allowed"
      style="background-color: var(--surface-100);"
    >
      <option value={null}>Select Device...</option>
      {#each $availableInterfaces as iface}
        <option value={iface}>{iface}</option>
      {/each}
    </select>
  </div>

  <div
    class="flex items-center border rounded min-w-[200px] px-2 transition-colors duration-200 focus-within:border-[var(--border-medium)]"
    style="background-color: var(--surface-100); border-color: var(--border-primary);"
  >
    <div class="text-muted flex items-center" title="Capture Filter (BPF)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"
        ><path d="M10 18h4v-2h-4v2zM3 6v2h18V6H3zm3 7h12v-2H6v2z" /></svg
      >
    </div>
    <input
      id="bpf-filter-input"
      type="text"
      placeholder="BPF Filter (e.g. tcp port 80)..."
      bind:value={$bpfFilter}
      disabled={$isCapturing}
      class="flex-1 p-2 bg-transparent text-sm outline-none placeholder-[rgba(38,37,30,0.55)] disabled:opacity-50"
      spellcheck="false"
    />
  </div>

  <div class="flex items-center gap-2">
    <button
      on:click={startCapture}
      disabled={!$selectedInterface || $isCapturing}
      class="cursor-btn-success flex items-center gap-1.5 disabled:opacity-40 disabled:cursor-not-allowed"
      title="Start Capture"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"
        ><path d="M8 5v14l11-7z" /></svg
      >
      Start
    </button>
    <button
      on:click={stopCapture}
      disabled={!$isCapturing}
      class="cursor-btn-primary flex items-center gap-1.5 disabled:opacity-40 disabled:cursor-not-allowed"
      title="Stop Capture"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"
        ><path d="M6 6h12v12H6z" /></svg
      >
      Stop
    </button>
    <button
      on:click={restartCapture}
      disabled={!$selectedInterface || !$isCapturing}
      class="cursor-btn-primary flex items-center gap-1.5 disabled:opacity-40 disabled:cursor-not-allowed"
      title="Restart Capture"
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
        ><path d="M23 4v6h-6" /><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" /></svg
      >
    </button>
    <div class="w-px h-6 mx-1" style="background-color: var(--border-primary);"></div>
    <button
      on:click={clearPackets}
      disabled={$totalFilteredCount === 0}
      class="cursor-btn-primary flex items-center gap-1.5 disabled:opacity-40 disabled:cursor-not-allowed"
      title="Clear packet list"
    >
      Clear
    </button>
    <button
      on:click={exportPcap}
      disabled={$totalFilteredCount === 0}
      class="cursor-btn-primary flex items-center gap-1.5 disabled:opacity-40 disabled:cursor-not-allowed"
      title="Export to PCAP file"
    >
      Export PCAP
    </button>
  </div>

  <div
    class="flex items-center border rounded flex-1 max-w-[400px] px-2 transition-colors duration-200 focus-within:border-[var(--border-medium)]"
    style="background-color: var(--surface-100); border-color: var(--border-primary);"
  >
    <div class="text-muted flex items-center">
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        ><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"
        ></line></svg
      >
    </div>
    <input
      id="filter-input"
      type="text"
      placeholder="Display Filter (e.g. protocol:tcp)..."
      bind:value={$displayFilter}
      class="flex-1 p-2 bg-transparent text-sm outline-none placeholder-[rgba(38,37,30,0.55)]"
      spellcheck="false"
    />
    {#if $displayFilter}
      <button
        class="bg-transparent border-none text-lg cursor-pointer px-1 hover:text-[var(--color-error)] transition-colors"
        style="color: rgba(38, 37, 30, 0.55);"
        on:click={() => displayFilter.set('')}
        title="Clear display filter"
      >
        ×
      </button>
    {/if}
  </div>

  <div class="ml-auto">
    <span
      class="text-micro px-3 py-1.5 rounded border flex items-center gap-2"
      style="background-color: var(--surface-100); border-color: var(--border-primary);"
    >
      <span>Packets</span>
      <strong
        class="font-mono text-base"
        style="color: var(--color-success); font-family: var(--font-mono);"
        >{$totalFilteredCount.toLocaleString()}</strong
      >
    </span>
  </div>

  {#if $captureError}
    <div
      class="fixed top-[60px] right-5 px-4 py-3 rounded-md border flex items-center gap-4 z-[1000] animate-[slideIn_0.3s_ease-out]"
      style="background-color: {$captureError.startsWith('PCAP exported')
        ? 'rgba(31, 138, 101, 0.15)'
        : 'rgba(207, 45, 86, 0.15)'};
             color: {$captureError.startsWith('PCAP exported')
        ? 'var(--color-success)'
        : 'var(--color-error)'};
             border-color: {$captureError.startsWith('PCAP exported')
        ? 'var(--color-success)'
        : 'var(--color-error)'};"
    >
      <span class="text-body-serif-sm">{$captureError}</span>
      <button
        class="bg-transparent border-none text-current text-xl cursor-pointer opacity-70 hover:opacity-100 transition-opacity"
        on:click={() => captureError.set(null)}>×</button
      >
    </div>
  {/if}
</div>

<style>
  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
</style>
