<script lang="ts">
  import { selectedStream, type StreamMessage } from '../stores';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  function close() {
    selectedStream.set(null);
    dispatch('close');
  }

  function formatData(data: number[]): string {
    try {
      const uint8 = new Uint8Array(data);
      const decoder = new TextDecoder('utf-8', { fatal: true });
      return decoder.decode(uint8);
    } catch (e) {
      return data.map(b => b.toString(16).padStart(2, '0')).join(' ');
    }
  }

  function isPrintable(data: number[]): boolean {
    return data.every(b => (b >= 32 && b <= 126) || b === 10 || b === 13 || b === 9);
  }
</script>

{#if $selectedStream}
  <div class="fixed inset-0 z-[100] flex items-center justify-center p-8" style="background-color: rgba(38, 37, 30, 0.4); backdrop-filter: blur(8px);">
    <div class="rounded-lg shadow-2xl w-full max-w-5xl h-full max-h-[80vh] flex flex-col overflow-hidden cursor-card-elevated" style="background-color: var(--surface-100); border: 1px solid var(--border-primary);">
      <!-- Header -->
      <div class="border-b p-4 flex items-center justify-between" style="background-color: var(--surface-200); border-color: var(--border-primary);">
        <div class="flex items-center gap-3">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" stroke-width="2"><path d="M7 11V7a5 5 0 0 1 10 0v4"/><rect x="3" y="11" width="18" height="11" rx="2"/><circle cx="12" cy="16" r="2"/></svg>
          <h2 class="m-0 text-lg" style="font-family: var(--font-gothic); letter-spacing: -0.11px; color: var(--cursor-dark);">Follow Stream</h2>
        </div>
        <button
          on:click={close}
          aria-label="Close stream view"
          class="bg-transparent border-none cursor-pointer p-1 rounded-md transition-colors cursor-btn-ghost"
          style="color: rgba(38, 37, 30, 0.55);"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6 flex flex-col gap-4 text-sm leading-relaxed" style="font-family: var(--font-mono);">
        {#each $selectedStream as msg}
          <div class="flex flex-col {msg.is_client ? 'items-start' : 'items-end'}">
            <div
              class="max-w-[90%] p-3 rounded-lg border"
              style="{msg.is_client ? 'background-color: var(--color-read); color: #26251e; border-color: var(--border-primary); border-bottom-left-radius: 0;' : 'background-color: var(--surface-300); color: var(--cursor-dark); border-color: var(--border-primary); border-bottom-right-radius: 0;'}"
            >
              <pre class="whitespace-pre-wrap break-all m-0">{formatData(msg.data)}</pre>
            </div>
            <span class="text-xs mt-1 px-1" style="color: rgba(38, 37, 30, 0.55);">
              {new Date(msg.timestamp / 1000000).toLocaleTimeString()}
            </span>
          </div>
        {/each}
      </div>

      <!-- Footer -->
      <div class="border-t p-4 flex items-center justify-between text-xs" style="background-color: var(--surface-200); border-color: var(--border-primary); color: rgba(38, 37, 30, 0.55);">
        <div class="flex gap-4">
          <span class="flex items-center gap-1.5"><div class="w-2 h-2 rounded-full" style="background-color: var(--color-read);"></div> Client</span>
          <span class="flex items-center gap-1.5"><div class="w-2 h-2 rounded-full" style="background-color: var(--surface-300);"></div> Server</span>
        </div>
        <div>Total Messages: {$selectedStream.length}</div>
      </div>
    </div>
  </div>
{/if}
