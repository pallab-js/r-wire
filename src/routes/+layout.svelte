<script lang="ts">
  // Root layout for SvelteKit
  import '../app.css';
  import { onMount } from 'svelte';
  
  onMount(() => {
    // Verify Tauri API is available
    if (typeof window !== 'undefined' && window.__TAURI__) {
      console.log('Tauri API is available');
    } else {
      console.warn('Tauri API not available - running in browser mode?');
    }
  });
</script>

<div class="h-screen w-screen flex flex-col overflow-hidden bg-[#1e1e1e] text-[#d4d4d4]">
  <slot />
</div>
