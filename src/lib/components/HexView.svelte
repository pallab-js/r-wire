<script lang="ts">
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
</script>

<div class="p-2 font-mono text-[0.85rem] bg-[#1e1e1e] text-[#d4d4d4]">
  <div class="grid grid-cols-[80px_1fr_200px] gap-4 p-2 bg-[#252526] border-b-2 border-[#3e3e3e] font-semibold text-[#ccc]">
    <span class="select-none">Offset</span>
    <span>Hex</span>
    <span>ASCII</span>
  </div>
  <div class="overflow-y-auto max-h-full">
    {#each rows as row, rowIndex}
      <div class="grid grid-cols-[80px_1fr_200px] gap-4 px-2 py-1 border-b border-[#2d2d2d] hover:bg-[#2a2a2a]">
        <span class="text-[#858585] select-none">{((rowIndex * bytesPerLine).toString(16).padStart(8, '0').toUpperCase())}</span>
        <span class="flex gap-1 flex-wrap">
          {#each row as byte, byteIndex}
            <span class="text-[#4ec9b0] min-w-[2ch] text-center">{formatHex(byte)}</span>
            {#if (byteIndex + 1) % 8 === 0 && byteIndex < row.length - 1}
              <span class="mx-1"> </span>
            {/if}
          {/each}
          {#if row.length < bytesPerLine}
            {#each Array(bytesPerLine - row.length) as _}
              <span class="text-transparent min-w-[2ch] text-center">  </span>
            {/each}
          {/if}
        </span>
        <span class="flex gap-0">
          {#each row as byte}
            <span class="text-[#ce9178] min-w-[1ch] text-center">{formatAscii(byte)}</span>
          {/each}
        </span>
      </div>
    {/each}
  </div>
</div>
