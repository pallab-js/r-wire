import { describe, it, expect } from 'vitest';
import { matchesFilter } from './filter';
import type { PacketSummary } from '../stores';

describe('filter.ts', () => {
  const mockPacket: PacketSummary = {
    id: 1,
    timestamp: '2024-03-29T10:00:00Z',
    source_addr: '192.168.1.1',
    dest_addr: '8.8.8.8',
    protocol: 'TCP',
    length: 64,
    info: '443 > 54321 [SYN] Seq=0 Win=64240 Len=0',
  };

  it('should return true if filter is empty', () => {
    expect(matchesFilter(mockPacket, '')).toBe(true);
    expect(matchesFilter(mockPacket, '  ')).toBe(true);
  });

  it('should filter by protocol', () => {
    expect(matchesFilter(mockPacket, 'protocol:tcp')).toBe(true);
    expect(matchesFilter(mockPacket, 'protocol:udp')).toBe(false);
  });

  it('should filter by IP address', () => {
    expect(matchesFilter(mockPacket, 'ip:192.168.1.1')).toBe(true);
    expect(matchesFilter(mockPacket, 'ip:8.8.8.8')).toBe(true);
    expect(matchesFilter(mockPacket, 'ip:1.1.1.1')).toBe(false);
  });

  it('should filter by port', () => {
    expect(matchesFilter(mockPacket, 'port:443')).toBe(true);
    expect(matchesFilter(mockPacket, 'port:80')).toBe(false);
  });

  it('should filter by source address', () => {
    expect(matchesFilter(mockPacket, 'src:192.168.1.1')).toBe(true);
    expect(matchesFilter(mockPacket, 'src:8.8.8.8')).toBe(false);
  });

  it('should filter by destination address', () => {
    expect(matchesFilter(mockPacket, 'dst:8.8.8.8')).toBe(true);
    expect(matchesFilter(mockPacket, 'dst:192.168.1.1')).toBe(false);
  });

  it('should perform a general search', () => {
    expect(matchesFilter(mockPacket, 'tcp')).toBe(true);
    expect(matchesFilter(mockPacket, '192.168')).toBe(true);
    expect(matchesFilter(mockPacket, 'SYN')).toBe(true);
    expect(matchesFilter(mockPacket, '64')).toBe(true);
    expect(matchesFilter(mockPacket, 'google')).toBe(false);
  });
});
