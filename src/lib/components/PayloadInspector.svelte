<script lang="ts">
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
    } catch {
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
    } catch {}

    // Try JWT
    if (text.startsWith('ey') && text.split('.').length === 3) {
      try {
        const parts = text.split('.');
        const header = JSON.parse(atob(parts[0]));
        const payload = JSON.parse(atob(parts[1]));
        formattedContent = JSON.stringify({ header, payload }, null, 2);
        contentType = 'jwt';
        return;
      } catch {}
    }

    // Default to plain text
    formattedContent = text;
    contentType = 'text';
  }

  function copyToClipboard() {
    navigator.clipboard.writeText(formattedContent);
  }
</script>

<div
  class="flex flex-col h-full"
  style="background-color: var(--bg-page); font-family: var(--font-mono);"
>
  <div
    class="border-b p-2 flex items-center justify-between shrink-0"
    style="background-color: var(--border-standard); border-color: var(--border-standard);"
  >
    <div class="flex items-center gap-2">
      <span class="text-code px-2">DETECTED:</span>
      <span
        class="px-2 py-0.5 text-xs font-medium rounded uppercase"
        style="background-color: #60a5fa; color: var(--bg-button);"
      >
        {contentType}
      </span>
    </div>
    <button
      on:click={copyToClipboard}
      class="text-xs px-3 py-1 rounded cursor-pointer transition-colors flex items-center gap-1.5 btn-secondary"
      style="background-color: var(--bg-button);"
    >
      <svg
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        ><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path
          d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
        ></path></svg
      >
      Copy
    </button>
  </div>

  <div class="flex-1 overflow-auto p-4 text-sm">
    {#if contentType === 'json' || contentType === 'jwt'}
      <pre
        class="m-0 whitespace-pre-wrap leading-relaxed"
        style="color: var(--text-primary);">{formattedContent}</pre>
    {:else}
      <pre
        class="m-0 whitespace-pre-wrap break-all leading-relaxed"
        style="color: var(--text-primary);">{formattedContent}</pre>
    {/if}
  </div>
</div>
