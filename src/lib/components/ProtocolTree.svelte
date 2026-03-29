<script lang="ts">
  import type { ProtocolLayer } from '../stores';

  export let layers: ProtocolLayer[];

  let filterText = '';

  function matchesFilter(fieldName: string, fieldValue: string) {
    if (!filterText) return true;
    const lowerFilter = filterText.toLowerCase();
    return fieldName.toLowerCase().includes(lowerFilter) || 
           fieldValue.toLowerCase().includes(lowerFilter);
  }

  function copyToClipboard(value: string) {
    navigator.clipboard.writeText(value);
  }
</script>

<div class="flex flex-col h-full font-sans">
  <div class="p-2 px-3 bg-[#252526] border-b border-[#3e3e3e] sticky top-0 z-10">
    <div class="flex items-center bg-[#1e1e1e] border border-[#444] rounded px-2 py-1">
      <input 
        type="text" 
        placeholder="Search fields (e.g., 'flag', 'ack', 'seq')..." 
        bind:value={filterText}
        class="flex-1 bg-transparent border-none text-[#ccc] text-sm outline-none p-1"
      />
      {#if filterText}
        <button on:click={() => filterText = ''} class="bg-transparent border-none text-[#888] cursor-pointer text-lg px-1">×</button>
      {/if}
    </div>
  </div>

  <div class="p-3 overflow-y-auto">
    {#if !layers || layers.length === 0}
      <div class="p-8 text-center text-[#888] italic">No protocol layers available</div>
    {:else}
      {#each layers as layer}
        {@const filteredFields = layer.fields.filter(([n, v]) => matchesFilter(n, v))}
        {#if filteredFields.length > 0}
          <details class="mb-2 border border-[#3e3e3e] rounded bg-[#252526] shadow-sm" open>
            <summary class="px-3 py-2.5 cursor-pointer select-none bg-[#2d2d2d] rounded-t flex items-center gap-2 hover:bg-[#353535]">
              <span class="text-[#4ec9b0] font-semibold text-[0.95rem]">{layer.name}</span>
              <span class="text-[#888] text-xs">({filteredFields.length} fields)</span>
            </summary>
            <div class="py-2 bg-[#1e1e1e]">
              {#each filteredFields as [fieldName, fieldValue]}
                <div class="flex px-3 py-1.5 gap-4 border-b border-[#252526] transition-colors duration-100 hover:bg-[#2a2d2e] group">
                  <span class="text-[#9cdcfe] min-w-[140px] font-medium text-[0.85rem]">{fieldName}:</span>
                  <div class="flex items-center gap-2 flex-1">
                    <span class="text-[#ce9178] font-mono text-sm break-all">{fieldValue}</span>
                    <button 
                      class="bg-transparent border-none text-[#666] cursor-pointer p-0.5 opacity-0 transition-all duration-200 flex items-center group-hover:opacity-100 hover:text-[#4ec9b0]"
                      title="Copy value"
                      on:click|stopPropagation={() => copyToClipboard(fieldValue)}
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          </details>
        {/if}
      {/each}
    {/if}
  </div>
</div>
