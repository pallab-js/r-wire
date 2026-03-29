<script lang="ts">
  import { selectedStream, type StreamMessage } from '../stores';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  function close() {
    selectedStream.set(null);
    dispatch('close');
  }

  function formatData(data: number[]): string {
    // Try to convert to string, or hex if not printable
    try {
      const uint8 = new Uint8Array(data);
      const decoder = new TextDecoder('utf-8', { fatal: true });
      return decoder.decode(uint8);
    } catch (e) {
      // Fallback to hex
      return data.map(b => b.toString(16).padStart(2, '0')).join(' ');
    }
  }

  function isPrintable(data: number[]): boolean {
    return data.every(b => (b >= 32 && b <= 126) || b === 10 || b === 13 || b === 9);
  }
</script>

{#if $selectedStream}
  <div class="fixed inset-0 bg-black/60 backdrop-blur-sm z-[100] flex items-center justify-center p-8">
    <div class="bg-[#1e1e1e] border border-[#3e3e3e] rounded-lg shadow-2xl w-full max-w-5xl h-full max-h-[80vh] flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="bg-[#252526] border-b border-[#3e3e3e] p-4 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#4ec9b0" stroke-width="2"><path d="M7 11V7a5 5 0 0 1 10 0v4"/><rect x="3" y="11" width="18" height="11" rx="2"/><circle cx="12" cy="16" r="2"/></svg>
          <h2 class="text-white font-semibold m-0 text-lg">Follow Stream</h2>
        </div>
        <button 
          on:click={close}
          aria-label="Close stream view"
          class="bg-transparent border-none text-[#888] hover:text-white cursor-pointer p-1 rounded-md hover:bg-white/10 transition-colors"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6 flex flex-col gap-4 font-mono text-sm leading-relaxed">
        {#each $selectedStream as msg}
          <div class="flex flex-col {msg.is_client ? 'items-start' : 'items-end'}">
            <div 
              class="max-w-[90%] p-3 rounded-lg {msg.is_client ? 'bg-[#094771] text-[#d4d4d4] rounded-bl-none' : 'bg-[#2d2d2d] text-[#dcdcaa] rounded-br-none'} border border-white/5"
            >
              <pre class="whitespace-pre-wrap break-all m-0">{formatData(msg.data)}</pre>
            </div>
            <span class="text-[0.7rem] text-[#666] mt-1 px-1">
              {new Date(msg.timestamp / 1000000).toLocaleTimeString()}
            </span>
          </div>
        {/each}
      </div>

      <!-- Footer -->
      <div class="bg-[#252526] border-t border-[#3e3e3e] p-4 flex items-center justify-between text-[#888] text-xs">
        <div class="flex gap-4">
          <span class="flex items-center gap-1.5"><div class="w-2 h-2 rounded-full bg-[#094771]"></div> Client</span>
          <span class="flex items-center gap-1.5"><div class="w-2 h-2 rounded-full bg-[#2d2d2d]"></div> Server</span>
        </div>
        <div>Total Messages: {$selectedStream.length}</div>
      </div>
    </div>
  </div>
{/if}
