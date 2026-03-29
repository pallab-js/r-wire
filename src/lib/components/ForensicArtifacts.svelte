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

<div class="flex flex-col h-full bg-[#1e1e1e] p-6 overflow-y-auto font-sans text-[#d4d4d4]">
  <div class="max-w-5xl">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h3 class="text-[#4ec9b0] text-sm font-bold uppercase tracking-widest mb-1">Extracted Artifacts</h3>
        <p class="text-sm text-[#888]">Files and objects reassembled from the stream</p>
      </div>
      <div class="px-3 py-1 bg-[#252526] rounded border border-[#3e3e3e] text-xs font-bold text-[#ccc]">
        {artifacts.length} OBJECTS FOUND
      </div>
    </div>

    {#if artifacts.length === 0}
      <div class="bg-[#252526] border border-dashed border-[#3e3e3e] rounded-lg p-12 flex flex-col items-center justify-center text-center">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#3e3e3e" stroke-width="1.5" class="mb-4"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
        <p class="text-[#888] italic">No files or objects identified in this packet's payload.</p>
        <p class="text-xs text-[#666] mt-2 max-w-xs">AuraCap automatically identifies artifacts in reassembled TCP/UDP streams (e.g., HTTP objects, certificates).</p>
      </div>
    {:else}
      <div class="flex flex-col gap-3">
        {#each artifacts as artifact}
          <div class="bg-[#252526] border border-[#3e3e3e] rounded-lg p-4 flex items-center gap-4 hover:border-[#007acc] transition-colors group">
            <div class="w-12 h-12 bg-[#1e1e1e] rounded flex items-center justify-center text-[#4ec9b0]">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <span class="font-bold text-sm truncate">{artifact.name}</span>
                <span class="px-1.5 py-0.5 bg-[#1e1e1e] text-[0.65rem] font-black rounded text-[#888] uppercase tracking-tighter">{artifact.mime_type}</span>
              </div>
              <div class="font-mono text-[0.7rem] text-[#666] flex items-center gap-3">
                <span>SIZE: {formatBytes(artifact.size)}</span>
                <span class="truncate">SHA256: {artifact.hash_sha256}</span>
              </div>
            </div>
            <button class="px-4 py-2 bg-[#007acc] text-white text-xs font-bold rounded opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer hover:bg-[#005a9e]">
              EXPORT
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
