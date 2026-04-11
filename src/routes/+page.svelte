<script lang="ts">
  import CaptureControls from '../lib/components/CaptureControls.svelte';
  import PacketList from '../lib/components/PacketList.svelte';
  import PacketDetailTabs from '../lib/components/PacketDetailTabs.svelte';
  import StatisticsPanel from '../lib/components/StatisticsPanel.svelte';
  import StreamDialog from '../lib/components/StreamDialog.svelte';
  import OnboardingTour from '../lib/components/OnboardingTour.svelte';
  import { selectedPacket } from '../lib/stores';
  import { onMount } from 'svelte';

  let showStats = false;
  let filterInputRef: HTMLInputElement | null = null;
  let showOnboarding = false;

  // Resizing state
  let dragType: 'vertical' | 'horizontal' | null = null;
  let listHeightPercent = 60;
  let statsWidthPx = 400;

  onMount(() => {
    const completed = localStorage.getItem('auracap_onboarding_complete');
    showOnboarding = completed !== 'true';
  });

  function startDrag(type: 'vertical' | 'horizontal') {
    dragType = type;
    document.body.style.cursor = type === 'vertical' ? 'row-resize' : 'col-resize';
    document.body.style.userSelect = 'none';
  }

  function onDrag(e: MouseEvent) {
    if (!dragType) return;

    if (dragType === 'vertical') {
      const container = document.getElementById('split-container');
      if (!container) return;
      const rect = container.getBoundingClientRect();
      const offsetY = e.clientY - rect.top;
      let newPercent = (offsetY / rect.height) * 100;
      listHeightPercent = Math.max(20, Math.min(80, newPercent));
    } else if (dragType === 'horizontal') {
      const newWidth = window.innerWidth - e.clientX;
      statsWidthPx = Math.max(300, Math.min(800, newWidth));
    }
  }

  function stopDrag() {
    if (dragType) {
      dragType = null;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    const isCtrlOrCmd = e.ctrlKey || e.metaKey;

    // Ctrl/Cmd + F: Focus filter
    if (isCtrlOrCmd && e.key === 'f') {
      e.preventDefault();
      filterInputRef?.focus();
      return;
    }

    // Ctrl/Cmd + E: Toggle statistics
    if (isCtrlOrCmd && e.key === 'e') {
      e.preventDefault();
      showStats = !showStats;
      return;
    }

    // Ctrl/Cmd + S: Export PCAP (handled in CaptureControls)

    // Ctrl/Cmd + O: Import PCAP (handled in CaptureControls)

    // Escape: Clear packet selection or close dialogs
    if (e.key === 'Escape') {
      if ($selectedPacket) {
        selectedPacket.set(null);
      }
      return;
    }
  }
</script>

<svelte:window
  on:mousemove={onDrag}
  on:mouseup={stopDrag}
  on:mouseleave={stopDrag}
  on:keydown={handleKeydown}
/>

<div
  class="flex flex-col h-screen overflow-hidden select-none"
  style="background-color: var(--cursor-cream); color: var(--cursor-dark);"
>
  <!-- Top Toolbar -->
  <header
    class="flex-none border-b z-20 responsive-toolbar"
    style="background-color: var(--surface-100); border-color: var(--border-primary);"
  >
    <div class="flex items-center justify-between px-4 py-3 responsive-toolbar-inner">
      <div class="flex items-center gap-3">
        <svg
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="var(--color-success)"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M4 22h14a2 2 0 0 0 2-2V7.5L14.5 2H6a2 2 0 0 0-2 2v4" />
          <polyline points="14 2 14 8 20 8" />
          <path d="M2 15h10" />
          <path d="m9 18 3-3-3-3" />
        </svg>
        <h1
          class="font-semibold tracking-wide text-lg m-0"
          style="font-family: var(--font-gothic); letter-spacing: -0.72px; color: var(--cursor-dark);"
        >
          AuraCap
        </h1>
      </div>
      <button
        class="cursor-btn-secondary-pill flex items-center gap-2"
        class:active={showStats}
        on:click={() => (showStats = !showStats)}
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <line x1="18" y1="20" x2="18" y2="10"></line>
          <line x1="12" y1="20" x2="12" y2="4"></line>
          <line x1="6" y1="20" x2="6" y2="14"></line>
        </svg>
        {showStats ? 'Hide Stats' : 'Show Stats'}
      </button>
    </div>
    <CaptureControls />
  </header>

  <!-- Main Content Area -->
  <main class="flex flex-1 overflow-hidden relative">
    <!-- Left Panel (Packets & Details) -->
    <div
      class="flex-1 flex flex-col min-w-0 relative"
      style="background-color: var(--cursor-cream);"
    >
      <div id="split-container" class="flex-1 flex flex-col h-full relative">
        <!-- Packet List (Top Pane) -->
        <div
          class="flex flex-col relative overflow-hidden"
          style="height: {$selectedPacket ? listHeightPercent + '%' : '100%'}"
        >
          <PacketList />
        </div>

        <!-- Vertical Resizer -->
        {#if $selectedPacket}
          <button
            aria-label="Resize panels"
            class="h-1.5 w-full cursor-row-resize border-y hover:bg-[var(--color-accent)] transition-colors flex items-center justify-center group z-10 p-0 border-x-0 shrink-0"
            style="background-color: var(--surface-200); border-color: var(--border-primary);"
            on:mousedown={() => startDrag('vertical')}
          >
            <div
              class="w-8 h-0.5 rounded-full transition-colors group-hover:bg-[var(--cursor-dark)]"
              style="background-color: rgba(38, 37, 30, 0.3);"
            ></div>
          </button>

          <!-- Packet Details (Bottom Pane) -->
          <div class="flex flex-col overflow-hidden" style="height: {100 - listHeightPercent}%">
            <PacketDetailTabs />
          </div>
        {/if}
      </div>
    </div>

    <!-- Stats Panel Horizontal Resizer -->
    {#if showStats}
      <button
        aria-label="Resize statistics"
        class="w-1.5 h-full cursor-col-resize border-x hover:bg-[var(--color-accent)] transition-colors flex items-center justify-center group z-20 p-0 border-y-0 shrink-0"
        style="background-color: var(--surface-200); border-color: var(--border-primary);"
        on:mousedown={() => startDrag('horizontal')}
      >
        <div
          class="h-8 w-0.5 rounded-full transition-colors group-hover:bg-[var(--cursor-dark)]"
          style="background-color: rgba(38, 37, 30, 0.3);"
        ></div>
      </button>

      <!-- Right Panel (Statistics) -->
      <div
        class="border-l z-10 overflow-y-auto shrink-0"
        style="width: {statsWidthPx}px; border-color: var(--border-primary); background-color: var(--surface-100);"
      >
        <StatisticsPanel />
      </div>
    {/if}
  </main>

  <!-- Modals & Overlays -->
  <StreamDialog />
  {#if showOnboarding}
    <OnboardingTour onComplete={() => (showOnboarding = false)} />
  {/if}
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background-color: var(--cursor-cream);
    overflow: hidden;
  }

  /* Prevent text selection while dragging */
  :global(.dragging) {
    user-select: none !important;
  }
</style>
