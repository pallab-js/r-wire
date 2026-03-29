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
        info: 'Test'
      },
      {
        id: 2,
        timestamp: Date.now() * 1000000,
        source_addr: '192.168.1.1',
        dest_addr: '1.1.1.1',
        protocol: 'UDP',
        length: 200,
        info: 'Test'
      }
    ];

    const updatedStats = updateStatistics(initialStats, mockPackets);

    expect(updatedStats.totalPackets).toBe(2);
    expect(updatedStats.totalBytes).toBe(300);
    
    const tcpStats = updatedStats.protocols.find(p => p.protocol === 'TCP');
    expect(tcpStats?.count).toBe(1);
    
    const udpStats = updatedStats.protocols.find(p => p.protocol === 'UDP');
    expect(udpStats?.count).toBe(1);
    
    const topSource = updatedStats.topSources.find(s => s.address === '192.168.1.1');
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
        info: 'Test'
      }
    ];

    const updatedStats = updateStatistics(initialStats, mockPackets);
    expect(updatedStats.timeSeries.length).toBeGreaterThan(0);
    expect(updatedStats.timeSeries[updatedStats.timeSeries.length - 1].bytes).toBe(1000);
  });
});
