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
    Filler,
  } from 'chart.js';

  Chart.register(
    LineController,
    LineElement,
    PointElement,
    LinearScale,
    CategoryScale,
    DoughnutController,
    ArcElement,
    Tooltip,
    Legend,
    Filler,
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
    const labels = stats.timeSeries.map((d) => {
      const date = new Date(d.timestamp * 1000);
      return `${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}:${date.getSeconds().toString().padStart(2, '0')}`;
    });
    const packets = stats.timeSeries.map((d) => d.packets);
    const bytes = stats.timeSeries.map((d) => d.bytes);

    timeChart.data.labels = labels;
    timeChart.data.datasets[0].data = packets;
    timeChart.data.datasets[1].data = bytes;
    timeChart.update('none');
  }

  $: if (protocolChart && stats.protocols) {
    protocolChart.data.labels = stats.protocols.map((p) => p.protocol);
    protocolChart.data.datasets[0].data = stats.protocols.map((p) => p.count);
    protocolChart.update('none');
  }

  onMount(() => {
    const commonOptions = {
      responsive: true,
      maintainAspectRatio: false,
      animation: { duration: 0 },
      color: '#fafafa',
      plugins: {
        legend: { labels: { color: '#fafafa' } },
      },
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
              borderColor: '#3ecf8e',
              backgroundColor: 'rgba(62, 207, 142, 0.1)',
              yAxisID: 'y',
              fill: true,
              tension: 0.2,
              pointRadius: 0,
            },
            {
              label: 'Bytes/s',
              data: [],
              borderColor: '#00c573',
              backgroundColor: 'rgba(0, 197, 115, 0.1)',
              yAxisID: 'y1',
              fill: true,
              tension: 0.2,
              pointRadius: 0,
            },
          ],
        },
        options: {
          ...commonOptions,
          scales: {
            x: {
              ticks: { color: '#898989' },
              grid: { color: 'rgba(250, 250, 250, 0.1)' },
            },
            y: {
              type: 'linear',
              display: true,
              position: 'left',
              ticks: { color: '#3ecf8e' },
              grid: { color: 'rgba(250, 250, 250, 0.1)' },
            },
            y1: {
              type: 'linear',
              display: true,
              position: 'right',
              ticks: { color: '#00c573' },
              grid: { drawOnChartArea: false },
            },
          },
        },
      });
    }

    const colors = [
      '#3ecf8e',
      '#00c573',
      '#60a5fa',
      '#f472b6',
      '#a78bfa',
      '#fbbf24',
      '#f87171',
      '#34d399',
    ];

    if (protocolChartCanvas) {
      protocolChart = new Chart(protocolChartCanvas, {
        type: 'doughnut',
        data: {
          labels: [],
          datasets: [
            {
              data: [],
              backgroundColor: colors,
              borderColor: '#171717',
              borderWidth: 2,
            },
          ],
        },
        options: {
          ...commonOptions,
          cutout: '70%',
          plugins: {
            legend: { position: 'right', labels: { color: '#fafafa', boxWidth: 12 } },
          },
        },
      });
    }
  });

  onDestroy(() => {
    if (timeChart) timeChart.destroy();
    if (protocolChart) protocolChart.destroy();
  });
</script>

<div
  class="p-4 h-full overflow-y-auto box-border"
  style="background-color: var(--bg-page); color: var(--text-primary);"
>
  {#if stats.totalPackets === 0}
    <div
      class="text-center p-8 flex h-full items-center justify-center"
      style="color: var(--text-muted);"
    >
      No packets captured yet.
    </div>
  {:else}
    <div class="flex flex-col gap-4">
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          class="p-4 rounded border flex flex-col justify-center min-w-0 overflow-hidden card"
          style="background-color: var(--border-standard); border-color: var(--border-standard);"
        >
          <div class="text-code mb-1 truncate">TOTAL PACKETS</div>
          <div class="text-2xl font-medium truncate" style="color: var(--brand-green);">
            {stats.totalPackets.toLocaleString()}
          </div>
        </div>
        <div
          class="p-4 rounded border flex flex-col justify-center min-w-0 overflow-hidden card"
          style="background-color: var(--border-standard); border-color: var(--border-standard);"
        >
          <div class="text-code mb-1 truncate">TOTAL BYTES</div>
          <div class="text-2xl font-medium truncate" style="color: var(--brand-green);">
            {formatBytes(stats.totalBytes)}
          </div>
        </div>
        <div
          class="p-4 rounded border flex flex-col justify-center min-w-0 overflow-hidden card"
          style="background-color: var(--border-standard); border-color: var(--border-standard);"
        >
          <div class="text-code mb-1 truncate">AVG PACKET SIZE</div>
          <div class="text-2xl font-medium truncate" style="color: var(--brand-green);">
            {stats.averagePacketSize > 0 ? Math.round(stats.averagePacketSize) : 0} BYTES
          </div>
        </div>
      </div>

      <div class="grid grid-cols-1 xl:grid-cols-[2fr_1fr] gap-4">
        <div
          class="p-4 rounded border flex flex-col min-h-[300px] card"
          style="background-color: var(--border-standard); border-color: var(--border-standard);"
        >
          <h4 class="m-0 mb-4 text-code">TRAFFIC RATE</h4>
          <div class="flex-1 relative min-h-0">
            <canvas bind:this={timeChartCanvas}></canvas>
          </div>
        </div>

        <div
          class="p-4 rounded border flex flex-col min-h-[300px] card"
          style="background-color: var(--border-standard); border-color: var(--border-standard);"
        >
          <h4 class="m-0 mb-4 text-code">PROTOCOLS</h4>
          <div class="flex-1 relative min-h-0">
            <canvas bind:this={protocolChartCanvas}></canvas>
          </div>
        </div>
      </div>

      <!-- Network Topology Widget -->
      <div
        class="p-4 rounded border flex flex-col min-h-[400px] card"
        style="background-color: var(--border-standard); border-color: var(--border-standard);"
      >
        <div class="flex items-center justify-between mb-4">
          <h4 class="m-0 text-code">NETWORK TOPOLOGY</h4>
          <span
            class="text-code px-2 py-0.5 rounded border"
            style="color: var(--text-muted); background-color: var(--bg-button); border-color: var(--border-standard);"
            >TOP 20 CONVERSATIONS</span
          >
        </div>
        <div
          class="flex-1 rounded border relative overflow-hidden group"
          style="background-color: var(--bg-button); border-color: var(--border-standard);"
        >
          <div
            class="absolute inset-0 flex items-center justify-center pointer-events-none opacity-10"
          >
            <svg width="100%" height="100%" viewBox="0 0 100 100" preserveAspectRatio="none">
              <circle
                cx="50"
                cy="50"
                r="40"
                fill="none"
                stroke="currentColor"
                stroke-width="0.5"
                stroke-dasharray="2 2"
              />
              <circle
                cx="50"
                cy="50"
                r="20"
                fill="none"
                stroke="currentColor"
                stroke-width="0.5"
                stroke-dasharray="2 2"
              />
            </svg>
          </div>
          <div class="relative z-10 p-4 h-full overflow-y-auto">
            <div class="flex flex-col gap-2">
              {#each stats.topSources.slice(0, 10) as src}
                {#each stats.topDestinations.slice(0, 2) as dst}
                  <div
                    class="flex items-center justify-between p-2 rounded border hover:border-[rgba(62,207,142,0.3)] transition-colors text-xs"
                    style="background-color: var(--border-standard); border-color: var(--border-subtle);"
                  >
                    <span class="truncate w-[120px] font-mono" style="color: var(--brand-green);"
                      >{src.address}</span
                    >
                    <div class="flex-1 flex items-center px-4 opacity-40">
                      <div
                        class="h-px flex-1"
                        style="background: linear-gradient(to right, var(--brand-green), var(--text-muted));"
                      ></div>
                      <svg
                        width="8"
                        height="8"
                        viewBox="0 0 24 24"
                        fill="currentColor"
                        style="color: var(--text-muted);"
                      >
                        <path d="M8.59 16.59L13.17 12 8.59 7.41 10 6l6 6-6 6-1.41-1.41z" />
                      </svg>
                    </div>
                    <span
                      class="truncate w-[120px] text-right font-mono"
                      style="color: var(--text-muted);">{dst.address}</span
                    >
                  </div>
                {/each}
              {/each}
            </div>
          </div>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div
          class="p-4 rounded border card"
          style="background-color: var(--border-standard); border-color: var(--border-standard);"
        >
          <h4 class="m-0 mb-4 text-code">TOP SOURCES</h4>
          <div class="flex flex-col gap-1.5">
            {#each stats.topSources as talker}
              <div
                class="flex justify-between px-3 py-1.5 rounded hover:bg-[var(--border-prominent)] transition-colors text-sm"
                style="background-color: var(--bg-button);"
              >
                <span class="font-mono" style="color: var(--brand-green);">{talker.address}</span>
                <span class="font-medium" style="color: var(--text-muted);"
                  >{talker.count.toLocaleString()}</span
                >
              </div>
            {/each}
          </div>
        </div>

        <div
          class="p-4 rounded border card"
          style="background-color: var(--border-standard); border-color: var(--border-standard);"
        >
          <h4 class="m-0 mb-4 text-code">TOP DESTINATIONS</h4>
          <div class="flex flex-col gap-1.5">
            {#each stats.topDestinations as talker}
              <div
                class="flex justify-between px-3 py-1.5 rounded hover:bg-[var(--border-prominent)] transition-colors text-sm"
                style="background-color: var(--bg-button);"
              >
                <span class="font-mono" style="color: var(--brand-green);">{talker.address}</span>
                <span class="font-medium" style="color: var(--text-muted);"
                  >{talker.count.toLocaleString()}</span
                >
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
