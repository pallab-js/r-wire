<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { statistics } from '../stores';
  import {
    Chart,
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    CategoryScale,
    DoughnutController,
    ArcElement,
    Tooltip,
    Legend,
    Filler
  } from 'chart.js';

  Chart.register(
    LineController, LineElement, PointElement, LinearScale, CategoryScale,
    DoughnutController, ArcElement, Tooltip, Legend, Filler
  );

  let timeChartCanvas: HTMLCanvasElement;
  let protocolChartCanvas: HTMLCanvasElement;
  let timeChart: Chart | null = null;
  let protocolChart: Chart | null = null;

  $: stats = $statistics;

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  // Reactive updates to charts
  $: if (timeChart && stats.timeSeries) {
    const labels = stats.timeSeries.map(d => {
      const date = new Date(d.timestamp * 1000);
      return `${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}:${date.getSeconds().toString().padStart(2, '0')}`;
    });
    const packets = stats.timeSeries.map(d => d.packets);
    const bytes = stats.timeSeries.map(d => d.bytes);

    timeChart.data.labels = labels;
    timeChart.data.datasets[0].data = packets;
    timeChart.data.datasets[1].data = bytes;
    timeChart.update('none'); // Update without animation for performance
  }

  $: if (protocolChart && stats.protocols) {
    protocolChart.data.labels = stats.protocols.map(p => p.protocol);
    protocolChart.data.datasets[0].data = stats.protocols.map(p => p.count);
    protocolChart.update('none');
  }

  onMount(() => {
    const commonOptions = {
      responsive: true,
      maintainAspectRatio: false,
      animation: { duration: 0 }, // Disable animation for real-time updates
      color: '#d4d4d4',
      plugins: {
        legend: { labels: { color: '#ccc' } }
      }
    };

    if (timeChartCanvas) {
      timeChart = new Chart(timeChartCanvas, {
        type: 'line',
        data: {
          labels: [],
          datasets: [
            {
              label: 'Packets/s',
              data: [],
              borderColor: '#4ec9b0',
              backgroundColor: 'rgba(78, 201, 176, 0.1)',
              yAxisID: 'y',
              fill: true,
              tension: 0.2,
              pointRadius: 0
            },
            {
              label: 'Bytes/s',
              data: [],
              borderColor: '#4a9eff',
              backgroundColor: 'rgba(74, 158, 255, 0.1)',
              yAxisID: 'y1',
              fill: true,
              tension: 0.2,
              pointRadius: 0
            }
          ]
        },
        options: {
          ...commonOptions,
          scales: {
            x: { ticks: { color: '#888' }, grid: { color: '#333' } },
            y: { 
              type: 'linear', display: true, position: 'left',
              ticks: { color: '#4ec9b0' }, grid: { color: '#333' }
            },
            y1: { 
              type: 'linear', display: true, position: 'right',
              ticks: { color: '#4a9eff' }, grid: { drawOnChartArea: false }
            }
          }
        }
      });
    }

    const colors = ['#4a9eff', '#ffa500', '#00ff00', '#ff6b6b', '#9b59b6', '#3498db', '#f1c40f', '#e74c3c'];
    
    if (protocolChartCanvas) {
      protocolChart = new Chart(protocolChartCanvas, {
        type: 'doughnut',
        data: {
          labels: [],
          datasets: [{
            data: [],
            backgroundColor: colors,
            borderColor: '#1e1e1e',
            borderWidth: 2
          }]
        },
        options: {
          ...commonOptions,
          cutout: '70%',
          plugins: {
            legend: { position: 'right', labels: { color: '#ccc', boxWidth: 12 } }
          }
        }
      });
    }
  });

  onDestroy(() => {
    if (timeChart) timeChart.destroy();
    if (protocolChart) protocolChart.destroy();
  });
</script>

<div class="p-4 bg-[#1e1e1e] text-[#d4d4d4] h-full overflow-y-auto box-border">
  {#if stats.totalPackets === 0}
    <div class="text-center text-[#888] p-8 italic">
      No packets captured yet.
    </div>
  {:else}
    <div class="grid grid-cols-3 gap-4 mb-6">
      <div class="bg-[#252526] p-4 rounded-md border border-[#3e3e3e] shadow-sm">
        <div class="text-[0.85rem] text-[#888] mb-2 uppercase tracking-wide">Total Packets</div>
        <div class="text-[1.8rem] font-semibold text-[#4ec9b0]">{stats.totalPackets.toLocaleString()}</div>
      </div>
      <div class="bg-[#252526] p-4 rounded-md border border-[#3e3e3e] shadow-sm">
        <div class="text-[0.85rem] text-[#888] mb-2 uppercase tracking-wide">Total Bytes</div>
        <div class="text-[1.8rem] font-semibold text-[#4ec9b0]">{formatBytes(stats.totalBytes)}</div>
      </div>
      <div class="bg-[#252526] p-4 rounded-md border border-[#3e3e3e] shadow-sm">
        <div class="text-[0.85rem] text-[#888] mb-2 uppercase tracking-wide">Avg Packet Size</div>
        <div class="text-[1.8rem] font-semibold text-[#4ec9b0]">{stats.averagePacketSize > 0 ? Math.round(stats.averagePacketSize) : 0} bytes</div>
      </div>
    </div>

    <div class="grid grid-cols-[2fr_1fr] grid-rows-[auto_auto] gap-4">
      <div class="bg-[#252526] p-4 rounded-md border border-[#3e3e3e] flex flex-col col-start-1 col-end-2 row-start-1 row-end-2 min-h-[300px]">
        <h4 class="m-0 mb-4 text-[#dcdcaa] text-base font-semibold">Traffic Rate (Last 60s)</h4>
        <div class="flex-1 relative min-h-0">
          <canvas bind:this={timeChartCanvas}></canvas>
        </div>
      </div>
      
      <div class="bg-[#252526] p-4 rounded-md border border-[#3e3e3e] flex flex-col col-start-2 col-end-3 row-start-1 row-end-2 min-h-[300px]">
        <h4 class="m-0 mb-4 text-[#dcdcaa] text-base font-semibold">Protocols</h4>
        <div class="flex-1 relative min-h-0">
          <canvas bind:this={protocolChartCanvas}></canvas>
        </div>
      </div>

      <div class="bg-[#252526] p-4 rounded-md border border-[#3e3e3e] row-start-2 row-end-3">
        <h4 class="m-0 mb-4 text-[#dcdcaa] text-base font-semibold">Top Sources</h4>
        <div class="flex flex-col gap-2">
          {#each stats.topSources as talker}
            <div class="flex justify-between px-3 py-2 bg-[#1e1e1e] rounded border border-[#333] text-[0.85rem]">
              <span class="text-[#4ec9b0] font-mono">{talker.address}</span>
              <span class="text-[#dcdcaa] font-semibold">{talker.count}</span>
            </div>
          {/each}
        </div>
      </div>

      <div class="bg-[#252526] p-4 rounded-md border border-[#3e3e3e] row-start-2 row-end-3">
        <h4 class="m-0 mb-4 text-[#dcdcaa] text-base font-semibold">Top Destinations</h4>
        <div class="flex flex-col gap-2">
          {#each stats.topDestinations as talker}
            <div class="flex justify-between px-3 py-2 bg-[#1e1e1e] rounded border border-[#333] text-[0.85rem]">
              <span class="text-[#4ec9b0] font-mono">{talker.address}</span>
              <span class="text-[#dcdcaa] font-semibold">{talker.count}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>
