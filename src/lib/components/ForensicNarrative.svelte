<script lang="ts">
  import type { ForensicNarrative } from '../stores';

  export let narrative: ForensicNarrative;
</script>

<div class="flex flex-col h-full p-6 overflow-y-auto" style="background-color: var(--cursor-cream); color: var(--cursor-dark);">
  <div class="max-w-3xl">
    <div class="mb-8">
      <h3 class="text-sm font-bold uppercase tracking-widest mb-3" style="color: var(--color-success); font-family: var(--font-mono);">Analyst Summary</h3>
      <p class="text-xl leading-relaxed font-medium" style="color: var(--cursor-dark); font-family: var(--font-gothic);">
        {narrative.summary}
      </p>
    </div>

    {#if narrative.technical_details.length > 0}
      <div>
        <h3 class="text-sm font-bold uppercase tracking-widest mb-4" style="color: rgba(38, 37, 30, 0.55); font-family: var(--font-mono);">Technical Breakdown</h3>
        <div class="flex flex-col gap-3">
          {#each narrative.technical_details as detail}
            <div class="flex items-start gap-3 p-4 rounded border" style="background-color: var(--surface-200); border-color: var(--border-primary);">
              <div class="mt-1" style="color: var(--color-success);">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"></polyline></svg>
              </div>
              <span class="text-[0.95rem] leading-snug" style="color: var(--cursor-dark); font-family: var(--font-gothic);">{detail}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
