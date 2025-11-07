<script lang="ts">
  // Root layout for SvelteKit
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

<div class="root-wrapper">
  <slot />
</div>

<style>
  :global(html) {
    height: 100%;
    width: 100%;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    overflow: hidden;
    height: 100%;
    width: 100%;
    background: #1e1e1e;
    color: #d4d4d4;
  }

  .root-wrapper {
    height: 100%;
    width: 100%;
    display: flex;
    flex-direction: column;
  }
</style>
