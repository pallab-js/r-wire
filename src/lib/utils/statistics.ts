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

/** Maximum number of points on the traffic chart. */
const MAX_TIME_BUCKETS = 60;
/** Upper bound on bucket width, so one bogus timestamp cannot flatten the chart. */
const MAX_BUCKET_SECONDS = 86_400;

/**
 * Width of one time bucket in seconds.
 *
 * Buckets start at one second and double once the capture spans more than
 * `MAX_TIME_BUCKETS` of them. Widening re-buckets the aggregates already
 * collected instead of deleting the oldest ones, which keeps
 * `sum(timeSeries) === totalPackets` for captures of any length.
 */
let bucketSeconds = 1;

export function resetStatistics() {
  protocolMap.clear();
  sourceMap.clear();
  destMap.clear();
  timeSeriesMap.clear();
  bucketSeconds = 1;
}

/** Collapses the existing buckets into buckets twice as wide. */
function widenTimeBuckets(): boolean {
  if (bucketSeconds >= MAX_BUCKET_SECONDS) {
    return false;
  }

  const next = bucketSeconds * 2;
  const merged = new Map<number, TimeSeriesData>();

  for (const entry of timeSeriesMap.values()) {
    const key = Math.floor(entry.timestamp / next) * next;
    const existing = merged.get(key);
    if (existing) {
      existing.packets += entry.packets;
      existing.bytes += entry.bytes;
    } else {
      merged.set(key, { timestamp: key, packets: entry.packets, bytes: entry.bytes });
    }
  }

  timeSeriesMap.clear();
  for (const [key, value] of merged) {
    timeSeriesMap.set(key, value);
  }

  bucketSeconds = next;
  return true;
}

/**
 * Grows the bucket width until every bucket fits on the chart.
 *
 * This used to delete the oldest buckets instead, so any capture longer than
 * 60 seconds silently lost its earliest traffic: the chart summed to half of
 * `totalPackets`, which counted everything.
 */
function fitTimeBucketsToChart() {
  while (timeSeriesMap.size > 1) {
    const keys = Array.from(timeSeriesMap.keys()).sort((a, b) => a - b);
    const span = keys[keys.length - 1] - keys[0];
    if (span <= (MAX_TIME_BUCKETS - 1) * bucketSeconds) {
      break;
    }
    if (!widenTimeBuckets()) {
      break;
    }
  }
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

    // Time series: bucket at the current width so the whole capture stays on
    // the chart (see `bucketSeconds`).
    const timeSec = Math.floor(packet.timestamp / 1_000_000_000);
    const bucket = Math.floor(timeSec / bucketSeconds) * bucketSeconds;
    const tsEntry = timeSeriesMap.get(bucket) || { timestamp: bucket, packets: 0, bytes: 0 };
    tsEntry.packets += 1;
    tsEntry.bytes += packet.length;
    timeSeriesMap.set(bucket, tsEntry);
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

  // Widen buckets (rather than dropping old ones) until the chart fits.
  fitTimeBucketsToChart();

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
