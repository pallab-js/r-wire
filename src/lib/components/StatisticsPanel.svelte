<script lang="ts">
  import { statistics } from '../stores';

  // Use memoized statistics store instead of recalculating
  $: stats = $statistics;

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
</script>

<div class="statistics-panel">
  <h3>Capture Statistics</h3>

  {#if stats.totalPackets === 0}
    <div class="empty-stats">
      No packets captured yet.
    </div>
  {:else}
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-label">Total Packets</div>
        <div class="stat-value">{stats.totalPackets.toLocaleString()}</div>
      </div>

      <div class="stat-card">
        <div class="stat-label">Total Bytes</div>
        <div class="stat-value">{formatBytes(stats.totalBytes)}</div>
      </div>

      <div class="stat-card">
        <div class="stat-label">Avg Packet Size</div>
        <div class="stat-value">{stats.averagePacketSize > 0 ? Math.round(stats.averagePacketSize) : 0} bytes</div>
      </div>
    </div>

    <div class="protocol-section">
      <h4>Protocol Distribution</h4>
      <div class="protocol-list">
        {#each stats.protocols as protocol}
          <div class="protocol-item">
            <div class="protocol-header">
              <span class="protocol-name">{protocol.protocol}</span>
              <span class="protocol-count">{protocol.count} ({protocol.percentage.toFixed(1)}%)</span>
            </div>
            <div class="protocol-bar">
              <div 
                class="protocol-bar-fill" 
                style="width: {protocol.percentage}%"
              ></div>
            </div>
            <div class="protocol-bytes">{formatBytes(protocol.totalBytes)}</div>
          </div>
        {/each}
      </div>
    </div>

    <div class="top-talkers">
      <div class="talker-column">
        <h4>Top Sources</h4>
        <div class="talker-list">
          {#each stats.topSources as talker}
            <div class="talker-item">
              <span class="talker-address">{talker.address}</span>
              <span class="talker-count">{talker.count}</span>
            </div>
          {/each}
        </div>
      </div>

      <div class="talker-column">
        <h4>Top Destinations</h4>
        <div class="talker-list">
          {#each stats.topDestinations as talker}
            <div class="talker-item">
              <span class="talker-address">{talker.address}</span>
              <span class="talker-count">{talker.count}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .statistics-panel {
    padding: 1rem;
    background: #1e1e1e;
    color: #d4d4d4;
    height: 100%;
    overflow-y: auto;
  }

  h3 {
    margin: 0 0 1rem 0;
    color: #4ec9b0;
    font-size: 1.2rem;
  }

  h4 {
    margin: 1rem 0 0.5rem 0;
    color: #dcdcaa;
    font-size: 1rem;
  }

  .empty-stats {
    text-align: center;
    color: #888;
    padding: 2rem;
    font-style: italic;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .stat-card {
    background: #252526;
    padding: 1rem;
    border-radius: 4px;
    border: 1px solid #3e3e3e;
  }

  .stat-label {
    font-size: 0.85rem;
    color: #888;
    margin-bottom: 0.5rem;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 600;
    color: #4ec9b0;
  }

  .protocol-section {
    margin-bottom: 1.5rem;
  }

  .protocol-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .protocol-item {
    background: #252526;
    padding: 0.75rem;
    border-radius: 4px;
    border: 1px solid #3e3e3e;
  }

  .protocol-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }

  .protocol-name {
    color: #dcdcaa;
    font-weight: 600;
  }

  .protocol-count {
    color: #888;
  }

  .protocol-bar {
    height: 6px;
    background: #1e1e1e;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 0.25rem;
  }

  .protocol-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #4a9eff, #007acc);
    transition: width 0.3s ease;
  }

  .protocol-bytes {
    font-size: 0.8rem;
    color: #888;
  }

  .top-talkers {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .talker-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .talker-item {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem;
    background: #252526;
    border-radius: 4px;
    border: 1px solid #3e3e3e;
    font-size: 0.85rem;
  }

  .talker-address {
    color: #4ec9b0;
    font-family: 'Courier New', monospace;
  }

  .talker-count {
    color: #dcdcaa;
    font-weight: 600;
  }
</style>
