<script lang="ts">
  import { highlightedRange } from '../stores';

  export let rawBytes: number[];

  // Handle empty or invalid bytes
  $: safeBytes = rawBytes && Array.isArray(rawBytes) ? rawBytes : [];

  function formatHex(byte: number): string {
    if (isNaN(byte) || byte < 0 || byte > 255) return '00';
    return byte.toString(16).padStart(2, '0').toUpperCase();
  }

  function formatAscii(byte: number): string {
    if (byte >= 32 && byte <= 126) {
      return String.fromCharCode(byte);
    }
    return '.';
  }

  function chunkArray<T>(arr: T[], size: number): T[][] {
    const chunks: T[][] = [];
    for (let i = 0; i < arr.length; i += size) {
      chunks.push(arr.slice(i, i + size));
    }
    return chunks;
  }

  const bytesPerLine = 16;
  $: rows = chunkArray(safeBytes, bytesPerLine);

  function isHighlighted(index: number, range: [number, number] | null): boolean {
    if (!range) return false;
    return index >= range[0] && index < range[1];
  }
</script>

<div class="flex flex-col h-full font-mono text-[0.85rem] bg-[#1e1e1e] text-[#d4d4d4]">
  <div class="grid grid-cols-[80px_1fr_160px] gap-4 p-2 bg-[#252526] border-b border-[#3e3e3e] font-semibold text-[#888] tracking-wider shrink-0 sticky top-0 z-10 text-xs">
    <span class="select-none text-center">OFFSET</span>
    <span>HEX</span>
    <span>ASCII</span>
  </div>
  <div class="flex-1 overflow-y-auto p-2">
    {#if safeBytes.length === 0}
      <div class="flex h-full items-center justify-center text-[#888] italic">No payload data</div>
    {:else}
      {#each rows as row, rowIndex}
        <div class="grid grid-cols-[80px_1fr_160px] gap-4 px-2 py-0.5 hover:bg-[#2a2d2e] rounded group transition-colors">
          <span class="text-[#858585] select-none text-right pr-2 border-r border-[#3e3e3e] group-hover:text-[#aaa]">{((rowIndex * bytesPerLine).toString(16).padStart(8, '0').toUpperCase())}</span>
          <span class="flex gap-1.5 flex-wrap">
            {#each row as byte, byteIndex}
              {@const absoluteIndex = rowIndex * bytesPerLine + byteIndex}
              <span 
                class="min-w-[2ch] text-center transition-colors duration-150 rounded-sm {isHighlighted(absoluteIndex, $highlightedRange) ? 'bg-[#007acc] text-white font-bold' : 'text-[#9cdcfe]'}"
              >
                {formatHex(byte)}
              </span>
              {#if (byteIndex + 1) % 8 === 0 && byteIndex < row.length - 1}
                <span class="mx-1"></span>
              {/if}
            {/each}
            {#if row.length < bytesPerLine}
              {#each Array(bytesPerLine - row.length) as _}
                <span class="text-transparent min-w-[2ch] text-center"></span>
              {/each}
            {/if}
          </span>
          <span class="flex gap-0 text-[#ce9178] tracking-widest opacity-80 group-hover:opacity-100">
            {#each row as byte, byteIndex}
              {@const absoluteIndex = rowIndex * bytesPerLine + byteIndex}
              <span class="{isHighlighted(absoluteIndex, $highlightedRange) ? 'bg-[#007acc] text-white' : ''}">
                {formatAscii(byte)}
              </span>
            {/each}
          </span>
        </div>
      {/each}
    {/if}
  </div>
</div>
