<script lang="ts">
  import type { Artifact } from '../stores';

  export let artifacts: Artifact[];

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
</script>

<div class="flex flex-col h-full p-6 overflow-y-auto" style="background-color: var(--cursor-cream); color: var(--cursor-dark);">
  <div class="max-w-5xl">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h3 class="text-sm font-bold uppercase tracking-widest mb-1" style="color: var(--color-success); font-family: var(--font-mono);">Extracted Artifacts</h3>
        <p class="text-sm" style="color: rgba(38, 37, 30, 0.55); font-family: var(--font-gothic);">Files and objects reassembled from the stream</p>
      </div>
      <div class="px-3 py-1 rounded border text-xs font-bold" style="background-color: var(--surface-200); border-color: var(--border-primary); color: var(--cursor-dark);">
        {artifacts.length} OBJECTS FOUND
      </div>
    </div>

    {#if artifacts.length === 0}
      <div class="border-dashed rounded-lg p-12 flex flex-col items-center justify-center text-center" style="background-color: var(--surface-200); border: 1px dashed var(--border-primary);">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--border-primary)" stroke-width="1.5" class="mb-4"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
        <p class="italic" style="color: rgba(38, 37, 30, 0.55); font-family: var(--font-gothic);">No files or objects identified in this packet's payload.</p>
        <p class="text-xs mt-2 max-w-xs" style="color: rgba(38, 37, 30, 0.55);">AuraCap automatically identifies artifacts in reassembled TCP/UDP streams (e.g., HTTP objects, certificates).</p>
      </div>
    {:else}
      <div class="flex flex-col gap-3">
        {#each artifacts as artifact}
          <div class="border rounded-lg p-4 flex items-center gap-4 hover:border-[var(--color-read)] transition-colors group" style="background-color: var(--surface-200); border-color: var(--border-primary);">
            <div class="w-12 h-12 rounded flex items-center justify-center" style="background-color: var(--surface-100); color: var(--color-success);">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <span class="font-bold text-sm truncate" style="color: var(--cursor-dark); font-family: var(--font-gothic);">{artifact.name}</span>
                <span class="px-1.5 py-0.5 text-[0.65rem] font-black rounded uppercase tracking-tighter" style="background-color: var(--surface-100); color: rgba(38, 37, 30, 0.55);">{artifact.mime_type}</span>
              </div>
              <div class="font-mono text-[0.7rem] flex items-center gap-3" style="color: rgba(38, 37, 30, 0.55); font-family: var(--font-mono);">
                <span>SIZE: {formatBytes(artifact.size)}</span>
                <span class="truncate">SHA256: {artifact.hash_sha256}</span>
              </div>
            </div>
            <button class="px-4 py-2 text-white text-xs font-bold rounded opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer" style="background-color: var(--color-read);" on:mouseover={(e) => e.currentTarget.style.backgroundColor = 'var(--color-read-hover, var(--color-read))'} on:mouseout={(e) => e.currentTarget.style.backgroundColor = 'var(--color-read)'}>
              EXPORT
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
