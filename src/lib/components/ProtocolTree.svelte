<script lang="ts">
  import { highlightedRange, type ProtocolLayer, type PacketField } from '../stores';

  export let layers: ProtocolLayer[];

  let filterText = '';
  let expandedState: Record<number, boolean> = {};

  $: {
    if (layers) {
      const newState: Record<number, boolean> = {};
      layers.forEach((_, i) => {
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

  function matchesFilter(field: PacketField) {
    if (!filterText) return true;
    const lowerFilter = filterText.toLowerCase();
    return (
      field.name.toLowerCase().includes(lowerFilter) ||
      field.value.toLowerCase().includes(lowerFilter)
    );
  }

  function copyToClipboard(value: string) {
    navigator.clipboard.writeText(value).catch((err) => {
      console.error('Failed to copy', err);
    });
  }

  function handleMouseEnter(range: [number, number]) {
    highlightedRange.set(range);
  }

  function handleMouseLeave() {
    highlightedRange.set(null);
  }

  function handleKeyDown(e: KeyboardEvent, range: [number, number]) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleMouseEnter(range);
    }
  }
</script>

<div
  class="flex flex-col h-full"
  style="background-color: var(--bg-page); color: var(--text-primary); font-family: var(--font-mono);"
>
  <div
    class="p-2 border-b sticky top-0 z-10 shrink-0"
    style="background-color: var(--border-standard); border-color: var(--border-standard);"
  >
    <div
      class="relative flex items-center border rounded-sm transition-colors focus-within:border-[var(--brand-border)]"
      style="background-color: var(--bg-button); border-color: var(--border-standard);"
    >
      <div class="pl-2" style="color: var(--text-muted);">
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
        type="text"
        placeholder="Filter fields..."
        bind:value={filterText}
        class="flex-1 bg-transparent border-none text-sm outline-none px-2 py-1.5"
        style="color: var(--text-primary);"
        spellcheck="false"
      />
      {#if filterText}
        <button
          on:click={() => (filterText = '')}
          class="bg-transparent border-none cursor-pointer px-2 btn-ghost"
          style="color: var(--text-muted);"
          title="Clear filter"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"
            ></line></svg
          >
        </button>
      {/if}
    </div>
  </div>

  <div class="flex-1 overflow-y-auto p-2 text-sm">
    {#if !layers || layers.length === 0}
      <div class="flex h-full items-center justify-center" style="color: var(--text-muted);">
        No protocol layers available
      </div>
    {:else}
      <div class="flex flex-col gap-0.5">
        {#each layers as layer, i}
          {@const filteredFields = layer.fields.filter((f) => matchesFilter(f))}
          {#if filteredFields.length > 0}
            <div class="group">
              <button
                class="w-full text-left flex items-center gap-1.5 px-1 py-1 rounded cursor-pointer transition-colors border-none bg-transparent"
                style="color: var(--text-primary);"
                on:click={() => toggleLayer(i)}
                on:mouseenter={(e) =>
                  (e.currentTarget.style.backgroundColor = 'var(--border-standard)')}
                on:mouseleave={(e) => (e.currentTarget.style.backgroundColor = 'transparent')}
              >
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="transition-transform duration-200 shrink-0"
                  style="color: var(--text-muted); transform: {expandedState[i]
                    ? 'rotate(90deg)'
                    : 'rotate(0deg)'}"
                >
                  <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
                <strong class="font-medium" style="color: var(--brand-green);">{layer.name}</strong>
              </button>

              {#if expandedState[i]}
                <div
                  class="ml-[22px] border-l pl-2 my-0.5 flex flex-col gap-0.5"
                  style="border-color: var(--border-subtle);"
                >
                  {#each filteredFields as field}
                    <div
                      class="flex px-2 py-0.5 rounded group/field items-start transition-colors"
                      style="color: var(--text-primary);"
                      on:mouseenter={() => {
                        handleMouseEnter(field.range);
                      }}
                      on:mouseleave={handleMouseLeave}
                      on:keydown={(e) => handleKeyDown(e, field.range)}
                      role="button"
                      tabindex="0"
                      aria-label="Inspect field {field.name}"
                    >
                      <span
                        class="w-[180px] shrink-0 font-medium"
                        style="color: var(--text-secondary);">{field.name}:</span
                      >
                      <span class="break-all flex-1" style="color: var(--text-muted);"
                        >{field.value}</span
                      >
                      {#if field.expert}
                        <span
                          class="ml-2 px-1.5 py-0.5 text-xs rounded font-medium uppercase"
                          style="background-color: rgba(239, 68, 68, 0.15); color: #ef4444;"
                          title={field.expert}>EXPERT</span
                        >
                      {/if}
                      <button
                        class="opacity-0 group-hover/field:opacity-100 bg-transparent border-none cursor-pointer px-1 shrink-0 transition-colors btn-ghost"
                        style="color: var(--text-muted);"
                        title="Copy value"
                        on:click={(e) => {
                          e.stopPropagation();
                          copyToClipboard(field.value);
                        }}
                      >
                        <svg
                          width="12"
                          height="12"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          ><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path
                            d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                          ></path></svg
                        >
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
