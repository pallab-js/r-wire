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

<div class="hex-view">
  <div class="hex-header">
    <span class="offset-header">Offset</span>
    <span class="hex-header-text">Hex</span>
    <span class="ascii-header">ASCII</span>
  </div>
  <div class="hex-content">
    {#each rows as row, rowIndex}
      <div class="hex-line">
        <span class="offset">{((rowIndex * bytesPerLine).toString(16).padStart(8, '0').toUpperCase())}</span>
        <span class="hex-bytes">
          {#each row as byte, byteIndex}
            <span class="hex-byte">{formatHex(byte)}</span>
            {#if (byteIndex + 1) % 8 === 0 && byteIndex < row.length - 1}
              <span class="hex-separator"> </span>
            {/if}
          {/each}
          {#if row.length < bytesPerLine}
            {#each Array(bytesPerLine - row.length) as _}
              <span class="hex-byte empty">  </span>
            {/each}
          {/if}
        </span>
        <span class="ascii-bytes">
          {#each row as byte}
            <span class="ascii-byte">{formatAscii(byte)}</span>
          {/each}
        </span>
      </div>
    {/each}
  </div>
</div>

<style>
  .hex-view {
    padding: 0.5rem;
    font-family: 'Courier New', monospace;
    font-size: 0.85rem;
    background: #1e1e1e;
    color: #d4d4d4;
  }

  .hex-header {
    display: grid;
    grid-template-columns: 80px 1fr 200px;
    gap: 1rem;
    padding: 0.5rem;
    background: #252526;
    border-bottom: 2px solid #3e3e3e;
    font-weight: 600;
    color: #ccc;
  }

  .hex-content {
    overflow-y: auto;
    max-height: 100%;
  }

  .hex-line {
    display: grid;
    grid-template-columns: 80px 1fr 200px;
    gap: 1rem;
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid #2d2d2d;
  }

  .hex-line:hover {
    background: #2a2a2a;
  }

  .offset {
    color: #858585;
    user-select: none;
  }

  .hex-bytes {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }

  .hex-byte {
    color: #4ec9b0;
    min-width: 2ch;
    text-align: center;
  }

  .hex-byte.empty {
    color: transparent;
  }

  .hex-separator {
    margin: 0 0.25rem;
  }

  .ascii-bytes {
    display: flex;
    gap: 0;
  }

  .ascii-byte {
    color: #ce9178;
    min-width: 1ch;
    text-align: center;
  }
</style>
