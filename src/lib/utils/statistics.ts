import type { PacketSummary } from '../stores';

export interface ProtocolStats {
  protocol: string;
  count: number;
  percentage: number;
  totalBytes: number;
}

export interface TimeSeriesData {
  timestamp: number;
  packets: number;
  bytes: number;
}

export interface Statistics {
  totalPackets: number;
  totalBytes: number;
  protocols: ProtocolStats[];
  topSources: Array<{ address: string; count: number }>;
  topDestinations: Array<{ address: string; count: number }>;
  averagePacketSize: number;
  timeSeries: TimeSeriesData[];
}

export function createEmptyStatistics(): Statistics {
  return {
    totalPackets: 0,
    totalBytes: 0,
    protocols: [],
    topSources: [],
    topDestinations: [],
    averagePacketSize: 0,
    timeSeries: [],
  };
}

// Internal state for incremental counting
const protocolMap = new Map<string, { count: number; bytes: number }>();
const sourceMap = new Map<string, number>();
const destMap = new Map<string, number>();
const timeSeriesMap = new Map<number, TimeSeriesData>();

export function resetStatistics() {
  protocolMap.clear();
  sourceMap.clear();
  destMap.clear();
  timeSeriesMap.clear();
}

export function updateStatistics(current: Statistics, newPackets: PacketSummary[]): Statistics {
  if (newPackets.length === 0) {
    return current;
  }

  let totalBytes = current.totalBytes;
  const totalPackets = current.totalPackets + newPackets.length;

  for (const packet of newPackets) {
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

    // Time series (bucket by second)
    const timeSec = Math.floor(packet.timestamp / 1_000_000_000);
    const tsEntry = timeSeriesMap.get(timeSec) || { timestamp: timeSec, packets: 0, bytes: 0 };
    tsEntry.packets += 1;
    tsEntry.bytes += packet.length;
    timeSeriesMap.set(timeSec, tsEntry);
  }

  // Convert maps to arrays and calculate percentages
  const protocols: ProtocolStats[] = Array.from(protocolMap.entries())
    .map(([protocol, data]) => ({
      protocol,
      count: data.count,
      percentage: (data.count / totalPackets) * 100,
      totalBytes: data.bytes,
    }))
    .sort((a, b) => b.count - a.count);

  const topSources = Array.from(sourceMap.entries())
    .map(([address, count]) => ({ address, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);

  const topDestinations = Array.from(destMap.entries())
    .map(([address, count]) => ({ address, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);

  // Keep last 60 seconds for time series to avoid infinite memory growth
  const sortedTimes = Array.from(timeSeriesMap.keys()).sort((a, b) => a - b);
  if (sortedTimes.length > 60) {
    const keysToRemove = sortedTimes.slice(0, sortedTimes.length - 60);
    for (const key of keysToRemove) {
      timeSeriesMap.delete(key);
    }
  }

  const timeSeries = Array.from(timeSeriesMap.values()).sort((a, b) => a.timestamp - b.timestamp);

  return {
    totalPackets,
    totalBytes,
    protocols,
    topSources,
    topDestinations,
    averagePacketSize: totalPackets > 0 ? totalBytes / totalPackets : 0,
    timeSeries,
  };
}
