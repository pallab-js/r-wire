<script lang="ts">
  import type { ForensicNarrative } from '../stores';

  export let narrative: ForensicNarrative;
</script>

<div class="flex flex-col h-full bg-[#1e1e1e] p-6 overflow-y-auto font-sans">
  <div class="max-w-3xl">
    <div class="mb-8">
      <h3 class="text-[#4ec9b0] text-sm font-bold uppercase tracking-widest mb-3">Analyst Summary</h3>
      <p class="text-xl text-[#d4d4d4] leading-relaxed font-medium">
        {narrative.summary}
      </p>
    </div>

    {#if narrative.technical_details.length > 0}
      <div>
        <h3 class="text-[#888] text-sm font-bold uppercase tracking-widest mb-4">Technical Breakdown</h3>
        <div class="flex flex-col gap-3">
          {#each narrative.technical_details as detail}
            <div class="flex items-start gap-3 bg-[#252526] p-4 rounded border border-[#3e3e3e]">
              <div class="mt-1 text-[#4ec9b0]">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"></polyline></svg>
              </div>
              <span class="text-[#ccc] text-[0.95rem] leading-snug">{detail}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
