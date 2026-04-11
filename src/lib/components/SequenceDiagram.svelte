<script lang="ts">
  import { selectedStream, selectedPacket } from '../stores';

  $: messages = $selectedStream || [];
  $: packet = $selectedPacket;

  // Use addresses from the selected packet if no stream is followed yet
  $: clientAddr = packet?.summary.source_addr || 'Client';
  $: serverAddr = packet?.summary.dest_addr || 'Server';

  function getInfo(msg: { data: number[] }) {
    const data = msg.data;
    try {
      const text = new TextDecoder().decode(new Uint8Array(data.slice(0, 50)));
      if (text.startsWith('GET')) return 'HTTP GET';
      if (text.startsWith('POST')) return 'HTTP POST';
      if (text.startsWith('HTTP/1.')) return `HTTP Response (${text.split(' ')[1]})`;
      return text.trim().substring(0, 30) + (text.length > 30 ? '...' : '');
    } catch {
      return `${data.length} bytes`;
    }
  }
</script>

<div
  class="flex flex-col h-full p-6 overflow-y-auto"
  style="background-color: var(--cursor-cream); font-family: var(--font-mono);"
>
  {#if messages.length === 0}
    <div
      class="flex flex-col items-center justify-center h-full gap-4"
      style="color: rgba(38, 37, 30, 0.55);"
    >
      <svg
        width="48"
        height="48"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1"
        class="opacity-20"
        ><path d="M7 11V7a5 5 0 0 1 10 0v4" /><rect
          x="3"
          y="11"
          width="18"
          height="11"
          rx="2"
        /><circle cx="12" cy="16" r="2" /></svg
      >
      <p class="italic text-center max-w-xs">
        Follow a stream to visualize the conversation as a sequence diagram.
      </p>
    </div>
  {:else}
    <div class="relative flex justify-between px-20 min-h-full pb-20">
      <!-- Lifelines -->
      <div
        class="absolute top-0 bottom-0 left-[150px] w-px"
        style="background-color: var(--border-primary);"
      ></div>
      <div
        class="absolute top-0 bottom-0 right-[150px] w-px"
        style="background-color: var(--border-primary);"
      ></div>

      <!-- Headers -->
      <div
        class="fixed top-0 left-0 right-0 z-10 px-20 flex justify-between py-4 border-b"
        style="background-color: rgba(242, 241, 237, 0.9); backdrop-filter: blur(8px); border-color: var(--border-primary);"
      >
        <div
          class="w-[200px] text-center px-4 py-2 rounded text-sm truncate font-mono"
          style="background-color: var(--surface-200); border: 1px solid var(--color-read); color: var(--color-read);"
          title={clientAddr}
        >
          {clientAddr}
        </div>
        <div
          class="w-[200px] text-center px-4 py-2 rounded text-sm truncate font-mono"
          style="background-color: var(--surface-200); border: 1px solid var(--color-thinking); color: var(--color-thinking);"
          title={serverAddr}
        >
          {serverAddr}
        </div>
      </div>

      <!-- Messages -->
      <div class="w-full mt-20 flex flex-col gap-8">
        {#each messages as msg}
          <div class="relative w-full flex items-center h-8">
            {#if msg.is_client}
              <!-- Client -> Server -->
              <div class="absolute left-[150px] right-[150px] flex items-center group">
                <div class="flex-1 h-0.5 relative" style="background-color: var(--color-read);">
                  <div
                    class="absolute right-0 top-1/2 -translate-y-1/2 border-y-4 border-y-transparent"
                    style="border-left: 8px solid var(--color-read);"
                  ></div>
                </div>
                <div
                  class="absolute left-1/2 -translate-x-1/2 -top-6 px-3 text-xs font-mono whitespace-nowrap opacity-80 group-hover:opacity-100 transition-opacity"
                  style="background-color: var(--cursor-cream); color: var(--color-read);"
                >
                  {getInfo(msg)}
                </div>
              </div>
            {:else}
              <!-- Server -> Client -->
              <div class="absolute left-[150px] right-[150px] flex items-center group">
                <div class="flex-1 h-0.5 relative" style="background-color: var(--color-thinking);">
                  <div
                    class="absolute left-0 top-1/2 -translate-y-1/2 border-y-4 border-y-transparent"
                    style="border-right: 8px solid var(--color-thinking);"
                  ></div>
                </div>
                <div
                  class="absolute left-1/2 -translate-x-1/2 -top-6 px-3 text-xs font-mono whitespace-nowrap opacity-80 group-hover:opacity-100 transition-opacity"
                  style="background-color: var(--cursor-cream); color: var(--color-thinking);"
                >
                  {getInfo(msg)}
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
