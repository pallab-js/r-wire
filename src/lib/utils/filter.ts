import type { PacketSummary } from '../stores';

export function matchesFilter(packet: PacketSummary, filter: string): boolean {
  if (!filter || filter.trim() === '') {
    return true;
  }

  const lowerFilter = filter.toLowerCase().trim();

  // Protocol filter
  if (lowerFilter.startsWith('protocol:')) {
    const protocol = lowerFilter.replace('protocol:', '').trim();
    if (!protocol) return false; // Empty protocol filter
    return packet.protocol.toLowerCase().includes(protocol);
  }

  // IP address filter
  if (lowerFilter.startsWith('ip:')) {
    const ip = lowerFilter.replace('ip:', '').trim();
    if (!ip) return false; // Empty IP filter
    return packet.source_addr.includes(ip) || packet.dest_addr.includes(ip);
  }

  // Port filter
  if (lowerFilter.startsWith('port:')) {
    const port = lowerFilter.replace('port:', '').trim();
    if (!port) return false; // Empty port filter
    return packet.info.includes(port);
  }

  // Source filter
  if (lowerFilter.startsWith('src:')) {
    const src = lowerFilter.replace('src:', '').trim();
    if (!src) return false; // Empty source filter
    return packet.source_addr.toLowerCase().includes(src);
  }

  // Destination filter
  if (lowerFilter.startsWith('dst:')) {
    const dst = lowerFilter.replace('dst:', '').trim();
    if (!dst) return false; // Empty destination filter
    return packet.dest_addr.toLowerCase().includes(dst);
  }

  // General search (searches in all fields)
  return (
    packet.protocol.toLowerCase().includes(lowerFilter) ||
    packet.source_addr.toLowerCase().includes(lowerFilter) ||
    packet.dest_addr.toLowerCase().includes(lowerFilter) ||
    packet.info.toLowerCase().includes(lowerFilter) ||
    packet.length.toString().includes(lowerFilter)
  );
}

export function getFilteredPackets(packets: PacketSummary[], filter: string): PacketSummary[] {
  if (!filter || filter.trim() === '') {
    return packets;
  }
  return packets.filter(packet => matchesFilter(packet, filter));
}