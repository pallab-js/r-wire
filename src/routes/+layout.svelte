<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';

  onMount(async () => {
    try {
      await import('@tauri-apps/api/tauri');
      console.log('Tauri API is available');
    } catch {
      console.warn('Tauri API not available - running in browser mode?');
    }
  });
</script>

<svelte:head>
  <style>
    :root {
      color-scheme: dark;
    }
  </style>
</svelte:head>

<div
  class="h-screen w-screen flex flex-col overflow-hidden"
  style="background-color: var(--bg-page); color: var(--text-primary);"
>
  <slot />
</div>
