<script lang="ts">
  import { selectedStream, selectedPacket } from '../stores';

  $: messages = $selectedStream || [];
  $: packet = $selectedPacket;

  // Use addresses from the selected packet if no stream is followed yet
  $: clientAddr = packet?.summary.source_addr || 'Client';
  $: serverAddr = packet?.summary.dest_addr || 'Server';

  function getInfo(msg: any) {
    // Try to extract a useful summary of the data
    const data = msg.data;
    try {
      const text = new TextDecoder().decode(new Uint8Array(data.slice(0, 50)));
      if (text.startsWith('GET')) return 'HTTP GET';
      if (text.startsWith('POST')) return 'HTTP POST';
      if (text.startsWith('HTTP/1.')) return `HTTP Response (${text.split(' ')[1]})`;
      return text.trim().substring(0, 30) + (text.length > 30 ? '...' : '');
    } catch (e) {
      return `${data.length} bytes`;
    }
  }
</script>

<div class="flex flex-col h-full bg-[#1e1e1e] p-6 overflow-y-auto font-sans">
  {#if messages.length === 0}
    <div class="flex flex-col items-center justify-center h-full text-[#888] gap-4">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" class="opacity-20"><path d="M7 11V7a5 5 0 0 1 10 0v4"/><rect x="3" y="11" width="18" height="11" rx="2"/><circle cx="12" cy="16" r="2"/></svg>
      <p class="italic text-center max-w-xs">Follow a stream to visualize the conversation as a sequence diagram.</p>
    </div>
  {:else}
    <div class="relative flex justify-between px-20 min-h-full pb-20">
      <!-- Lifelines -->
      <div class="absolute top-0 bottom-0 left-[150px] w-px bg-[#3e3e3e]"></div>
      <div class="absolute top-0 bottom-0 right-[150px] w-px bg-[#3e3e3e]"></div>

      <!-- Headers -->
      <div class="fixed top-0 left-0 right-0 z-10 px-20 flex justify-between bg-[#1e1e1e]/80 backdrop-blur-md py-4 border-b border-[#3e3e3e]">
         <div class="w-[200px] text-center px-4 py-2 bg-[#252526] border border-[#4ec9b0] rounded text-[#4ec9b0] font-mono text-sm truncate" title={clientAddr}>{clientAddr}</div>
         <div class="w-[200px] text-center px-4 py-2 bg-[#252526] border border-[#dcdcaa] rounded text-[#dcdcaa] font-mono text-sm truncate" title={serverAddr}>{serverAddr}</div>
      </div>

      <!-- Messages -->
      <div class="w-full mt-20 flex flex-col gap-8">
        {#each messages as msg}
          <div class="relative w-full flex items-center h-8">
            {#if msg.is_client}
              <!-- Client -> Server -->
              <div class="absolute left-[150px] right-[150px] flex items-center group">
                <div class="flex-1 h-0.5 bg-[#4ec9b0] relative">
                  <div class="absolute right-0 top-1/2 -translate-y-1/2 border-y-4 border-y-transparent border-l-8 border-l-[#4ec9b0]"></div>
                </div>
                <div class="absolute left-1/2 -translate-x-1/2 -top-6 bg-[#1e1e1e] px-3 text-[#4ec9b0] text-[0.75rem] font-mono whitespace-nowrap opacity-80 group-hover:opacity-100 transition-opacity">
                  {getInfo(msg)}
                </div>
              </div>
            {:else}
              <!-- Server -> Client -->
              <div class="absolute left-[150px] right-[150px] flex items-center group">
                <div class="flex-1 h-0.5 bg-[#dcdcaa] relative">
                  <div class="absolute left-0 top-1/2 -translate-y-1/2 border-y-4 border-y-transparent border-r-8 border-r-[#dcdcaa]"></div>
                </div>
                <div class="absolute left-1/2 -translate-x-1/2 -top-6 bg-[#1e1e1e] px-3 text-[#dcdcaa] text-[0.75rem] font-mono whitespace-nowrap opacity-80 group-hover:opacity-100 transition-opacity">
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
