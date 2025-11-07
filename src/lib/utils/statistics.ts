import type { PacketSummary } from '../stores';

export interface ProtocolStats {
  protocol: string;
  count: number;
  percentage: number;
  totalBytes: number;
}

export interface Statistics {
  totalPackets: number;
  totalBytes: number;
  protocols: ProtocolStats[];
  topSources: Array<{ address: string; count: number }>;
  topDestinations: Array<{ address: string; count: number }>;
  averagePacketSize: number;
}

export function calculateStatistics(packets: PacketSummary[]): Statistics {
  if (packets.length === 0) {
    return {
      totalPackets: 0,
      totalBytes: 0,
      protocols: [],
      topSources: [],
      topDestinations: [],
      averagePacketSize: 0,
    };
  }

  // Protocol statistics
  const protocolMap = new Map<string, { count: number; bytes: number }>();
  const sourceMap = new Map<string, number>();
  const destMap = new Map<string, number>();
  let totalBytes = 0;

  for (const packet of packets) {
    // Protocol stats
    const proto = packet.protocol;
    const entry = protocolMap.get(proto) || { count: 0, bytes: 0 };
    entry.count += 1;
    entry.bytes += packet.length;
    protocolMap.set(proto, entry);

    // Source stats
    const srcCount = sourceMap.get(packet.source_addr) || 0;
    sourceMap.set(packet.source_addr, srcCount + 1);

    // Destination stats
    const dstCount = destMap.get(packet.dest_addr) || 0;
    destMap.set(packet.dest_addr, dstCount + 1);

    totalBytes += packet.length;
  }

  // Convert protocol map to array and calculate percentages
  const protocols: ProtocolStats[] = Array.from(protocolMap.entries())
    .map(([protocol, data]) => ({
      protocol,
      count: data.count,
      percentage: (data.count / packets.length) * 100,
      totalBytes: data.bytes,
    }))
    .sort((a, b) => b.count - a.count);

  // Top sources
  const topSources = Array.from(sourceMap.entries())
    .map(([address, count]) => ({ address, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);

  // Top destinations
  const topDestinations = Array.from(destMap.entries())
    .map(([address, count]) => ({ address, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);

  return {
    totalPackets: packets.length,
    totalBytes,
    protocols,
    topSources,
    topDestinations,
    averagePacketSize: packets.length > 0 ? totalBytes / packets.length : 0,
  };
}
