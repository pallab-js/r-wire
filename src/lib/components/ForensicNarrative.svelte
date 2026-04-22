<script lang="ts">
  import type { ForensicNarrative } from '../stores';

  export let narrative: ForensicNarrative;
</script>

<div
  class="flex flex-col h-full p-6 overflow-y-auto"
  style="background-color: var(--bg-page); color: var(--text-primary);"
>
  <div class="max-w-3xl">
    <div class="mb-8">
      <h3
        class="text-sm font-medium uppercase tracking-widest mb-3"
        style="color: var(--brand-green); font-family: var(--font-mono);"
      >
        ANALYST SUMMARY
      </h3>
      <p
        class="text-xl leading-relaxed font-medium"
        style="color: var(--text-primary); font-family: var(--font-sans);"
      >
        {narrative.summary}
      </p>
    </div>

    {#if narrative.technical_details.length > 0}
      <div>
        <h3
          class="text-sm font-medium uppercase tracking-widest mb-4"
          style="color: var(--text-muted); font-family: var(--font-mono);"
        >
          TECHNICAL BREAKDOWN
        </h3>
        <div class="flex flex-col gap-3">
          {#each narrative.technical_details as detail}
            <div
              class="flex items-start gap-3 p-4 rounded border"
              style="background-color: var(--border-standard); border-color: var(--border-standard);"
            >
              <div class="mt-1" style="color: var(--brand-green);">
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"><polyline points="20 6 9 17 4 12"></polyline></svg
                >
              </div>
              <span
                class="text-[0.95rem] leading-snug"
                style="color: var(--text-primary); font-family: var(--font-sans);">{detail}</span
              >
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
