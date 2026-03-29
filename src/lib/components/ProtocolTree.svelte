<script lang="ts">
  import type { ProtocolLayer } from '../stores';

  export let layers: ProtocolLayer[];

  let filterText = '';
  // Track expanded state for each layer explicitly
  let expandedState: Record<number, boolean> = {};

  // Expand all by default when layers change
  $: {
    if (layers) {
      const newState: Record<number, boolean> = {};
      layers.forEach((_, i) => {
        // Only override if not already set by user interaction
        if (expandedState[i] === undefined) {
          newState[i] = true;
        } else {
          newState[i] = expandedState[i];
        }
      });
      expandedState = newState;
    }
  }

  function toggleLayer(index: number) {
    expandedState[index] = !expandedState[index];
    expandedState = { ...expandedState };
  }

  function matchesFilter(fieldName: string, fieldValue: string) {
    if (!filterText) return true;
    const lowerFilter = filterText.toLowerCase();
    return fieldName.toLowerCase().includes(lowerFilter) || 
           fieldValue.toLowerCase().includes(lowerFilter);
  }

  function copyToClipboard(value: string) {
    navigator.clipboard.writeText(value).catch(err => {
      console.error('Failed to copy', err);
    });
  }
</script>

<div class="flex flex-col h-full font-sans bg-[#1e1e1e] text-[#d4d4d4]">
  <div class="p-2 border-b border-[#3e3e3e] bg-[#252526] sticky top-0 z-10 shrink-0">
    <div class="relative flex items-center bg-[#1e1e1e] border border-[#3e3e3e] rounded-sm transition-colors focus-within:border-[#007acc]">
      <div class="pl-2 text-[#888]">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
      </div>
      <input 
        type="text" 
        placeholder="Filter fields..." 
        bind:value={filterText}
        class="flex-1 bg-transparent border-none text-[#ccc] text-sm outline-none px-2 py-1.5 placeholder-[#666]"
      />
      {#if filterText}
        <button on:click={() => filterText = ''} class="bg-transparent border-none text-[#888] cursor-pointer hover:text-white px-2" title="Clear filter">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        </button>
      {/if}
    </div>
  </div>

  <div class="flex-1 overflow-y-auto p-2 font-mono text-[0.85rem]">
    {#if !layers || layers.length === 0}
      <div class="flex h-full items-center justify-center text-[#888] italic">No protocol layers available</div>
    {:else}
      <div class="flex flex-col gap-0.5">
        {#each layers as layer, i}
          {@const filteredFields = layer.fields.filter(([n, v]) => matchesFilter(n, v))}
          {#if filteredFields.length > 0}
            <div class="group">
              <button 
                class="w-full text-left flex items-center gap-1.5 px-1 py-1 hover:bg-[#2a2d2e] rounded cursor-pointer transition-colors border-none bg-transparent"
                on:click={() => toggleLayer(i)}
              >
                <svg 
                  width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                  class="text-[#888] transition-transform duration-200 shrink-0"
                  style="transform: {expandedState[i] ? 'rotate(90deg)' : 'rotate(0deg)'}"
                >
                  <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
                <strong class="text-[#4ec9b0] font-medium">{layer.name}</strong>
              </button>
              
              {#if expandedState[i]}
                <div class="ml-[22px] border-l border-[#3e3e3e] pl-2 my-0.5 flex flex-col gap-0.5">
                  {#each filteredFields as [fieldName, fieldValue]}
                    <div class="flex px-2 py-0.5 hover:bg-[#2a2d2e] rounded group/field items-start">
                      <span class="text-[#9cdcfe] w-[180px] shrink-0 font-medium">{fieldName}:</span>
                      <span class="text-[#ce9178] break-all flex-1">{fieldValue}</span>
                      <button 
                        class="opacity-0 group-hover/field:opacity-100 bg-transparent border-none text-[#888] hover:text-white cursor-pointer px-1 shrink-0 transition-opacity"
                        title="Copy value"
                        on:click={(e) => { e.stopPropagation(); copyToClipboard(fieldValue); }}
                      >
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                      </button>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
</div>
