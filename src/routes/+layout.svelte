<script lang="ts">
  // Root layout for SvelteKit
  import '../app.css';
  import { onMount } from 'svelte';

  onMount(async () => {
    // Verify Tauri API is available
    try {
      await import('@tauri-apps/api/tauri');
      console.log('Tauri API is available');
    } catch {
      console.warn('Tauri API not available - running in browser mode?');
    }
  });
</script>

<div class="h-screen w-screen flex flex-col overflow-hidden" style="background-color: var(--cursor-cream); color: var(--cursor-dark);">
  <slot />
</div>
