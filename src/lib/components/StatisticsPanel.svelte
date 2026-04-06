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
    timeChart.update('none');
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
      animation: { duration: 0 },
      color: '#26251e',
      plugins: {
        legend: { labels: { color: '#26251e' } }
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
              borderColor: '#1f8a65',
              backgroundColor: 'rgba(31, 138, 101, 0.1)',
              yAxisID: 'y',
              fill: true,
              tension: 0.2,
              pointRadius: 0
            },
            {
              label: 'Bytes/s',
              data: [],
              borderColor: '#f54e00',
              backgroundColor: 'rgba(245, 78, 0, 0.1)',
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
            x: { ticks: { color: 'rgba(38, 37, 30, 0.55)' }, grid: { color: 'rgba(38, 37, 30, 0.1)' } },
            y: {
              type: 'linear', display: true, position: 'left',
              ticks: { color: '#1f8a65' }, grid: { color: 'rgba(38, 37, 30, 0.1)' }
            },
            y1: {
              type: 'linear', display: true, position: 'right',
              ticks: { color: '#f54e00' }, grid: { drawOnChartArea: false }
            }
          }
        }
      });
    }

    const colors = ['#f54e00', '#c08532', '#1f8a65', '#cf2d56', '#9fbbe0', '#9fc9a2', '#c0a8dd', '#dfa88f'];

    if (protocolChartCanvas) {
      protocolChart = new Chart(protocolChartCanvas, {
        type: 'doughnut',
        data: {
          labels: [],
          datasets: [{
            data: [],
            backgroundColor: colors,
            borderColor: '#f2f1ed',
            borderWidth: 2
          }]
        },
        options: {
          ...commonOptions,
          cutout: '70%',
          plugins: {
            legend: { position: 'right', labels: { color: '#26251e', boxWidth: 12 } }
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

<div class="p-4 h-full overflow-y-auto box-border" style="background-color: var(--surface-100); color: var(--cursor-dark);">
  {#if stats.totalPackets === 0}
    <div class="text-center p-8 italic flex h-full items-center justify-center" style="color: rgba(38, 37, 30, 0.55);">
      No packets captured yet.
    </div>
  {:else}
    <div class="flex flex-col gap-4">
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        <div class="p-4 rounded border shadow-sm flex flex-col justify-center min-w-0 overflow-hidden cursor-card" style="background-color: var(--surface-200); border-color: var(--border-primary);">
          <div class="text-micro mb-1 truncate">Total Packets</div>
          <div class="text-2xl font-bold truncate" style="color: var(--color-success);">{stats.totalPackets.toLocaleString()}</div>
        </div>
        <div class="p-4 rounded border shadow-sm flex flex-col justify-center min-w-0 overflow-hidden cursor-card" style="background-color: var(--surface-200); border-color: var(--border-primary);">
          <div class="text-micro mb-1 truncate">Total Bytes</div>
          <div class="text-2xl font-bold truncate" style="color: var(--color-success);">{formatBytes(stats.totalBytes)}</div>
        </div>
        <div class="p-4 rounded border shadow-sm flex flex-col justify-center min-w-0 overflow-hidden cursor-card" style="background-color: var(--surface-200); border-color: var(--border-primary);">
          <div class="text-micro mb-1 truncate">Avg Packet Size</div>
          <div class="text-2xl font-bold truncate" style="color: var(--color-success);">{stats.averagePacketSize > 0 ? Math.round(stats.averagePacketSize) : 0} bytes</div>
        </div>
      </div>

      <div class="grid grid-cols-1 xl:grid-cols-[2fr_1fr] gap-4">
        <div class="p-4 rounded border flex flex-col min-h-[300px] cursor-card" style="background-color: var(--surface-200); border-color: var(--border-primary);">
          <h4 class="m-0 mb-4 text-micro">Traffic Rate</h4>
          <div class="flex-1 relative min-h-0">
            <canvas bind:this={timeChartCanvas}></canvas>
          </div>
        </div>

        <div class="p-4 rounded border flex flex-col min-h-[300px] cursor-card" style="background-color: var(--surface-200); border-color: var(--border-primary);">
          <h4 class="m-0 mb-4 text-micro">Protocols</h4>
          <div class="flex-1 relative min-h-0">
            <canvas bind:this={protocolChartCanvas}></canvas>
          </div>
        </div>
      </div>

      <!-- Network Topology Widget -->
      <div class="p-4 rounded border flex flex-col min-h-[400px] cursor-card" style="background-color: var(--surface-200); border-color: var(--border-primary);">
        <div class="flex items-center justify-between mb-4">
          <h4 class="m-0 text-micro">Network Topology</h4>
          <span class="text-micro px-2 py-0.5 rounded border" style="color: rgba(38, 37, 30, 0.55); background-color: var(--surface-100); border-color: var(--border-primary);">TOP 20 CONVERSATIONS</span>
        </div>
        <div class="flex-1 rounded border relative overflow-hidden group" style="background-color: var(--surface-100); border-color: var(--border-primary);">
          <div class="absolute inset-0 flex items-center justify-center pointer-events-none opacity-10">
             <svg width="100%" height="100%" viewBox="0 0 100 100" preserveAspectRatio="none">
               <circle cx="50" cy="50" r="40" fill="none" stroke="currentColor" stroke-width="0.5" stroke-dasharray="2 2" />
               <circle cx="50" cy="50" r="20" fill="none" stroke="currentColor" stroke-width="0.5" stroke-dasharray="2 2" />
             </svg>
          </div>
          <div class="relative z-10 p-4 h-full overflow-y-auto">
             <div class="flex flex-col gap-2">
                {#each stats.topSources.slice(0, 10) as src}
                   {#each stats.topDestinations.slice(0, 2) as dst}
                      <div class="flex items-center justify-between p-2 rounded border hover:border-[var(--color-success)]/30 transition-colors text-xs" style="background-color: var(--surface-200); border-color: var(--border-primary);">
                         <span class="truncate w-[120px] font-mono" style="color: var(--color-success);">{src.address}</span>
                         <div class="flex-1 flex items-center px-4 opacity-40">
                            <div class="h-px flex-1" style="background: linear-gradient(to right, var(--color-success), rgba(38, 37, 30, 0.55));"></div>
                            <svg width="8" height="8" viewBox="0 0 24 24" fill="currentColor" style="color: rgba(38, 37, 30, 0.55);"><path d="M8.59 16.59L13.17 12 8.59 7.41 10 6l6 6-6 6-1.41-1.41z"/></svg>
                         </div>
                         <span class="truncate w-[120px] text-right font-mono" style="color: rgba(38, 37, 30, 0.55);">{dst.address}</span>
                      </div>
                   {/each}
                {/each}
             </div>
          </div>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="p-4 rounded border cursor-card" style="background-color: var(--surface-200); border-color: var(--border-primary);">
          <h4 class="m-0 mb-4 text-micro">Top Sources</h4>
          <div class="flex flex-col gap-1.5">
            {#each stats.topSources as talker}
              <div class="flex justify-between px-3 py-1.5 rounded hover:bg-[var(--surface-300)] transition-colors text-sm" style="background-color: var(--surface-100);">
                <span class="font-mono" style="color: var(--color-success);">{talker.address}</span>
                <span class="font-medium" style="color: rgba(38, 37, 30, 0.55);">{talker.count.toLocaleString()}</span>
              </div>
            {/each}
          </div>
        </div>

        <div class="p-4 rounded border cursor-card" style="background-color: var(--surface-200); border-color: var(--border-primary);">
          <h4 class="m-0 mb-4 text-micro">Top Destinations</h4>
          <div class="flex flex-col gap-1.5">
            {#each stats.topDestinations as talker}
              <div class="flex justify-between px-3 py-1.5 rounded hover:bg-[var(--surface-300)] transition-colors text-sm" style="background-color: var(--surface-100);">
                <span class="font-mono" style="color: var(--color-success);">{talker.address}</span>
                <span class="font-medium" style="color: rgba(38, 37, 30, 0.55);">{talker.count.toLocaleString()}</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
