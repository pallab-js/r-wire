import { describe, it, expect } from 'vitest';
import { matchesFilter } from './filter';
import type { PacketSummary } from '../stores';

describe('filter.ts', () => {
  const mockPacket: PacketSummary = {
    id: 1,
    timestamp: 1711706400000000,
    source_addr: '192.168.1.1',
    dest_addr: '8.8.8.8',
    protocol: 'TCP',
    length: 64,
    info: '443 > 54321 [SYN] Seq=0 Win=64240 Len=0',
    src_port: 54321,
    dst_port: 443,
  };

  const icmpPacket: PacketSummary = {
    id: 2,
    timestamp: 1711706400000001,
    source_addr: '192.168.1.1',
    dest_addr: '8.8.8.8',
    protocol: 'ICMP',
    length: 98,
    info: '192.168.1.1 -> 8.8.8.8 [ICMP]',
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
    expect(matchesFilter(mockPacket, 'port:54321')).toBe(true);
    expect(matchesFilter(mockPacket, 'port:80')).toBe(false);
  });

  it('should not match port substrings from the info text', () => {
    // Regression: `port:` used to substring-match `info`, so "443" matched
    // "44" and "54321" matched "5432".
    expect(matchesFilter(mockPacket, 'port:44')).toBe(false);
    expect(matchesFilter(mockPacket, 'port:5432')).toBe(false);
    expect(matchesFilter(mockPacket, 'port:192.168.1.1')).toBe(false);
  });

  it('should not match ports on packets without a transport layer', () => {
    expect(matchesFilter(icmpPacket, 'port:80')).toBe(false);
    expect(matchesFilter(icmpPacket, 'protocol:icmp')).toBe(true);
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
