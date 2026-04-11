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

<div
  class="flex flex-col h-full text-sm"
  style="background-color: var(--cursor-cream); color: var(--cursor-dark); font-family: var(--font-mono);"
>
  <div
    class="grid grid-cols-[80px_1fr_160px] gap-4 p-2 border-b shrink-0 sticky top-0 z-10 font-semibold tracking-wider text-xs"
    style="background-color: var(--surface-200); border-color: var(--border-primary); color: rgba(38, 37, 30, 0.55);"
  >
    <span class="select-none text-center">OFFSET</span>
    <span>HEX</span>
    <span>ASCII</span>
  </div>
  <div class="flex-1 overflow-y-auto p-2">
    {#if safeBytes.length === 0}
      <div
        class="flex h-full items-center justify-center italic"
        style="color: rgba(38, 37, 30, 0.55);"
      >
        No payload data
      </div>
    {:else}
      {#each rows as row, rowIndex}
        <div
          class="grid grid-cols-[80px_1fr_160px] gap-4 px-2 py-0.5 rounded group transition-colors"
          role="row"
          tabindex="0"
          on:mouseenter={(e) => (e.currentTarget.style.backgroundColor = 'var(--surface-200)')}
          on:mouseleave={(e) => (e.currentTarget.style.backgroundColor = '')}
          on:focus={(e) => (e.currentTarget.style.backgroundColor = 'var(--surface-200)')}
          on:blur={(e) => (e.currentTarget.style.backgroundColor = '')}
        >
          <span
            class="select-none text-right pr-2 border-r group-hover:text-[var(--cursor-dark)]"
            style="color: rgba(38, 37, 30, 0.55); border-color: var(--border-primary);"
            >{(rowIndex * bytesPerLine).toString(16).padStart(8, '0').toUpperCase()}</span
          >
          <span class="flex gap-1.5 flex-wrap">
            {#each row as byte, byteIndex}
              {@const absoluteIndex = rowIndex * bytesPerLine + byteIndex}
              <span
                class="min-w-[2ch] text-center transition-colors duration-150 rounded-sm"
                style={isHighlighted(absoluteIndex, $highlightedRange)
                  ? 'background-color: var(--color-read); color: var(--color-text-on-dark, #26251e); font-weight: bold;'
                  : 'color: var(--cursor-dark);'}
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
          <span
            class="flex gap-0 tracking-widest opacity-80 group-hover:opacity-100"
            style="color: rgba(38, 37, 30, 0.55);"
          >
            {#each row as byte, byteIndex}
              {@const absoluteIndex = rowIndex * bytesPerLine + byteIndex}
              <span
                class={isHighlighted(absoluteIndex, $highlightedRange)
                  ? 'bg-[var(--color-read)] text-[#26251e]'
                  : ''}
              >
                {formatAscii(byte)}
              </span>
            {/each}
          </span>
        </div>
      {/each}
    {/if}
  </div>
</div>
