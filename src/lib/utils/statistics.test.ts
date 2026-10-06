import { describe, it, expect, beforeEach } from 'vitest';
import { updateStatistics, createEmptyStatistics, resetStatistics } from './statistics';
import type { PacketSummary } from '../stores';

describe('statistics.ts', () => {
  beforeEach(() => {
    resetStatistics();
  });

  it('should initialize with empty statistics', () => {
    const stats = createEmptyStatistics();
    expect(stats.totalPackets).toBe(0);
    expect(stats.totalBytes).toBe(0);
    expect(stats.topSources).toHaveLength(0);
  });

  it('should update statistics correctly for a batch of packets', () => {
    const initialStats = createEmptyStatistics();
    const mockPackets: PacketSummary[] = [
      {
        id: 1,
        timestamp: Date.now() * 1000000,
        source_addr: '192.168.1.1',
        dest_addr: '8.8.8.8',
        protocol: 'TCP',
        length: 100,
        info: 'Test',
      },
      {
        id: 2,
        timestamp: Date.now() * 1000000,
        source_addr: '192.168.1.1',
        dest_addr: '1.1.1.1',
        protocol: 'UDP',
        length: 200,
        info: 'Test',
      },
    ];

    const updatedStats = updateStatistics(initialStats, mockPackets);

    expect(updatedStats.totalPackets).toBe(2);
    expect(updatedStats.totalBytes).toBe(300);

    const tcpStats = updatedStats.protocols.find((p) => p.protocol === 'TCP');
    expect(tcpStats?.count).toBe(1);

    const udpStats = updatedStats.protocols.find((p) => p.protocol === 'UDP');
    expect(udpStats?.count).toBe(1);

    const topSource = updatedStats.topSources.find((s) => s.address === '192.168.1.1');
    expect(topSource?.count).toBe(2);
  });

  it('should maintain time-based traffic data points', () => {
    const initialStats = createEmptyStatistics();
    const now_ns = 1711706400000000000; // Fixed timestamp in ns
    const mockPackets: PacketSummary[] = [
      {
        id: 1,
        timestamp: now_ns,
        source_addr: 'A',
        dest_addr: 'B',
        protocol: 'TCP',
        length: 1000,
        info: 'Test',
      },
    ];

    const updatedStats = updateStatistics(initialStats, mockPackets);
    expect(updatedStats.timeSeries.length).toBeGreaterThan(0);
    expect(updatedStats.timeSeries[updatedStats.timeSeries.length - 1].bytes).toBe(1000);
  });

  const packetAt = (id: number, seconds: number): PacketSummary => ({
    id,
    timestamp: seconds * 1_000_000_000,
    source_addr: '10.0.0.1',
    dest_addr: '10.0.0.2',
    protocol: 'TCP',
    length: 100,
    info: 'x',
  });

  const sumPackets = (stats: { timeSeries: { packets: number }[] }) =>
    stats.timeSeries.reduce((n, bucket) => n + bucket.packets, 0);

  it('keeps every packet in the time series for captures longer than 60s', () => {
    // Regression: the chart used to delete the oldest buckets, so this run
    // reported 120 total packets while the chart summed to 60.
    const base = 1_760_000_000;
    let stats = createEmptyStatistics();
    for (let s = 0; s < 120; s++) {
      stats = updateStatistics(stats, [packetAt(s + 1, base + s)]);
    }

    expect(stats.totalPackets).toBe(120);
    expect(sumPackets(stats)).toBe(120);
    expect(stats.timeSeries.length).toBeLessThanOrEqual(60);
  });

  it('covers the whole capture span instead of only the newest 60s', () => {
    const base = 1_760_000_000;
    let stats = createEmptyStatistics();
    for (let s = 0; s < 120; s++) {
      stats = updateStatistics(stats, [packetAt(s + 1, base + s)]);
    }

    expect(stats.timeSeries[0].timestamp).toBe(base);
    expect(stats.timeSeries[stats.timeSeries.length - 1].timestamp).toBeGreaterThanOrEqual(
      base + 118,
    );
  });

  it('leaves short captures at one-second granularity', () => {
    const base = 1_760_000_000;
    let stats = createEmptyStatistics();
    stats = updateStatistics(stats, [packetAt(1, base)]);
    stats = updateStatistics(stats, [packetAt(2, base)]);
    stats = updateStatistics(stats, [packetAt(3, base + 2)]);

    expect(stats.timeSeries).toHaveLength(2);
    expect(sumPackets(stats)).toBe(3);
    expect(stats.timeSeries.map((b) => b.timestamp)).toEqual([base, base + 2]);
  });

  it('restores one-second buckets after a reset', () => {
    const base = 1_760_000_000;
    let stats = createEmptyStatistics();
    for (let s = 0; s < 120; s++) {
      stats = updateStatistics(stats, [packetAt(s + 1, base + s)]);
    }
    expect(stats.timeSeries.length).toBeLessThan(120);

    resetStatistics();
    stats = createEmptyStatistics();
    stats = updateStatistics(stats, [packetAt(1, base)]);

    expect(stats.timeSeries).toHaveLength(1);
    expect(stats.timeSeries[0].timestamp).toBe(base);
    expect(stats.totalPackets).toBe(1);
  });

  it('preserves totals even when bucket width hits its cap', () => {
    // One packet at the epoch next to a modern one: the span is absurd, but
    // nothing may be dropped from the series.
    let stats = createEmptyStatistics();
    stats = updateStatistics(stats, [packetAt(1, 0)]);
    for (let s = 0; s < 70; s++) {
      stats = updateStatistics(stats, [packetAt(s + 2, 1_760_000_000 + s)]);
    }

    expect(sumPackets(stats)).toBe(stats.totalPackets);
    expect(stats.totalPackets).toBe(71);
  });
});
