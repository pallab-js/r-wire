<script lang="ts">
  import CaptureControls from '../lib/components/CaptureControls.svelte';
  import PacketList from '../lib/components/PacketList.svelte';
  import PacketDetailTabs from '../lib/components/PacketDetailTabs.svelte';
  import StatisticsPanel from '../lib/components/StatisticsPanel.svelte';
  import { selectedPacket } from '../lib/stores';

  let showStats = false;
  
  // Resizing state
  let dragType: 'vertical' | 'horizontal' | null = null;
  let listHeightPercent = 60; 
  let statsWidthPx = 400;
  
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
</script>

<svelte:window on:mousemove={onDrag} on:mouseup={stopDrag} on:mouseleave={stopDrag} />

<div class="flex flex-col h-screen bg-[#1e1e1e] text-[#d4d4d4] font-sans overflow-hidden select-none">
  <!-- Top Toolbar -->
  <header class="flex-none bg-[#252526] border-b border-[#3e3e3e] shadow-sm z-20">
    <div class="flex items-center justify-between px-4 py-2">
      <div class="flex items-center gap-3">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#4ec9b0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M4 22h14a2 2 0 0 0 2-2V7.5L14.5 2H6a2 2 0 0 0-2 2v4"/>
          <polyline points="14 2 14 8 20 8"/>
          <path d="M2 15h10"/>
          <path d="m9 18 3-3-3-3"/>
        </svg>
        <h1 class="text-[#d4d4d4] font-semibold tracking-wide text-lg m-0">AuraCap</h1>
      </div>
      <button 
        class="px-3 py-1.5 border border-[#3e3e3e] rounded cursor-pointer text-sm font-medium transition-colors bg-transparent text-[#ccc] hover:bg-[#333] hover:text-white flex items-center gap-2"
        class:bg-[#333]={showStats}
        class:text-white={showStats}
        on:click={() => showStats = !showStats}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
    <div class="flex-1 flex flex-col min-w-0 bg-[#1e1e1e] relative">
      <div id="split-container" class="flex-1 flex flex-col h-full relative">
        <!-- Packet List (Top Pane) -->
        <div class="flex flex-col relative overflow-hidden" style="height: {$selectedPacket ? listHeightPercent + '%' : '100%'}">
          <PacketList />
        </div>
        
        <!-- Vertical Resizer -->
        {#if $selectedPacket}
          <button 
            aria-label="Resize panels"
            class="h-1.5 w-full cursor-row-resize bg-[#2d2d2d] border-y border-[#3e3e3e] hover:bg-[#007acc] transition-colors flex items-center justify-center group z-10 p-0 border-x-0 shrink-0"
            on:mousedown={() => startDrag('vertical')}
          >
             <div class="w-8 h-0.5 bg-[#555] rounded-full group-hover:bg-white transition-colors"></div>
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
        class="w-1.5 h-full cursor-col-resize bg-[#2d2d2d] border-x border-[#3e3e3e] hover:bg-[#007acc] transition-colors flex items-center justify-center group z-20 p-0 border-y-0 shrink-0"
        on:mousedown={() => startDrag('horizontal')}
      >
         <div class="h-8 w-0.5 bg-[#555] rounded-full group-hover:bg-white transition-colors"></div>
      </button>

      <!-- Right Panel (Statistics) -->
      <div 
        class="border-l border-[#3e3e3e] bg-[#252526] shadow-[-4px_0_15px_rgba(0,0,0,0.2)] z-10 overflow-y-auto shrink-0"
        style="width: {statsWidthPx}px"
      >
        <StatisticsPanel />
      </div>
    {/if}
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background-color: #1e1e1e;
    overflow: hidden;
  }
  
  /* Prevent text selection while dragging */
  :global(.dragging) {
    user-select: none !important;
  }
</style>