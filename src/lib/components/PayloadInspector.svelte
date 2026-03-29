<script lang="ts">
  import { onMount } from 'svelte';

  export let rawBytes: number[];

  let contentType: 'json' | 'jwt' | 'text' | 'hex' = 'text';
  let formattedContent = '';

  $: {
    if (rawBytes) {
      inspectPayload();
    }
  }

  function inspectPayload() {
    if (!rawBytes || rawBytes.length === 0) {
      formattedContent = 'No payload data';
      contentType = 'text';
      return;
    }

    const uint8 = new Uint8Array(rawBytes);
    const decoder = new TextDecoder('utf-8');
    let text = '';
    
    try {
      text = decoder.decode(uint8);
    } catch (e) {
      contentType = 'hex';
      formattedContent = 'Binary data';
      return;
    }

    // Try JSON
    try {
      const parsed = JSON.parse(text);
      formattedContent = JSON.stringify(parsed, null, 2);
      contentType = 'json';
      return;
    } catch (e) {}

    // Try JWT
    if (text.startsWith('ey') && text.split('.').length === 3) {
      try {
        const parts = text.split('.');
        const header = JSON.parse(atob(parts[0]));
        const payload = JSON.parse(atob(parts[1]));
        formattedContent = JSON.stringify({ header, payload }, null, 2);
        contentType = 'jwt';
        return;
      } catch (e) {}
    }

    // Default to plain text
    formattedContent = text;
    contentType = 'text';
  }

  function copyToClipboard() {
    navigator.clipboard.writeText(formattedContent);
  }
</script>

<div class="flex flex-col h-full bg-[#1e1e1e] font-sans">
  <div class="bg-[#252526] border-b border-[#3e3e3e] p-2 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-2">
      <span class="text-xs font-bold text-[#888] uppercase tracking-widest px-2">Detected:</span>
      <span class="px-2 py-0.5 bg-[#007acc] text-white text-[0.7rem] font-bold rounded uppercase">
        {contentType}
      </span>
    </div>
    <button 
      on:click={copyToClipboard}
      class="text-xs bg-[#333] hover:bg-[#444] text-[#ccc] border-none px-3 py-1 rounded cursor-pointer transition-colors flex items-center gap-1.5"
    >
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
      Copy
    </button>
  </div>

  <div class="flex-1 overflow-auto p-4 font-mono text-sm">
    {#if contentType === 'json' || contentType === 'jwt'}
      <pre class="text-[#9cdcfe] m-0 whitespace-pre-wrap leading-relaxed">{formattedContent}</pre>
    {:else}
      <pre class="text-[#d4d4d4] m-0 whitespace-pre-wrap break-all leading-relaxed">{formattedContent}</pre>
    {/if}
  </div>
</div>
