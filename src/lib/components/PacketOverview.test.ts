import { render, screen } from '@testing-library/svelte/svelte5';
import { describe, it, expect } from 'vitest';
import PacketOverview from './PacketOverview.svelte';
import type { PacketDetail, PacketSummary, ProtocolLayer } from '../stores';

function makePacket(
  overrides: {
    summary?: Partial<PacketSummary>;
    layers?: ProtocolLayer[];
    riskScore?: number;
  } = {},
): PacketDetail {
  const summary: PacketSummary = {
    id: 1,
    timestamp: 1_760_000_000_000_000_000,
    source_addr: '192.168.1.1',
    dest_addr: '192.168.1.2',
    protocol: 'TCP',
    length: 74,
    info: 'syn',
    ...overrides.summary,
  };

  return {
    summary,
    layers: overrides.layers ?? [],
    raw_bytes: [],
    expert_summary: [],
    narrative: { summary: 'A test packet.', technical_details: [] },
    intelligence: {
      entropy: 4.2,
      manufacturer: null,
      risk_score: overrides.riskScore ?? 0,
    },
    artifacts: [],
  };
}

describe('PacketOverview.svelte', () => {
  it('shows transport ports from the summary, not from the address', () => {
    // Regression: addresses are bare, so parsing them for "ip:port" always
    // failed and the port tiles never rendered.
    render(PacketOverview, {
      packet: makePacket({ summary: { protocol: 'HTTP', src_port: 54321, dst_port: 80 } }),
    });

    expect(screen.getByText('192.168.1.1')).toBeInTheDocument();
    expect(screen.getByText('192.168.1.2')).toBeInTheDocument();
    expect(screen.getByText('54321')).toBeInTheDocument();
    expect(screen.getByText('80')).toBeInTheDocument();
  });

  it('shows the TCP header panel for app-layer protocols riding on TCP', () => {
    // Regression: gating on `protocol === 'TCP'` hid TCP details for HTTP.
    render(PacketOverview, {
      packet: makePacket({
        summary: { protocol: 'HTTP' },
        layers: [
          {
            name: 'Internet Protocol Version 4',
            fields: [{ name: 'TTL', value: '64', range: [22, 23], expert: null }],
          },
          {
            name: 'Transmission Control Protocol',
            fields: [{ name: 'Flags', value: 'SYN', range: [47, 48], expert: null }],
          },
        ],
      }),
    });

    expect(screen.getByText('TCP Details')).toBeInTheDocument();
    expect(screen.getByText('SYN')).toBeInTheDocument();
  });

  it('shows the IPv6 Hop Limit in the IP header panel', () => {
    // Regression: only "ttl"/"time to live" were searched, so IPv6 packets
    // lost the whole IP header panel.
    render(PacketOverview, {
      packet: makePacket({
        summary: { source_addr: 'fe80::1', dest_addr: 'fe80::2' },
        layers: [
          {
            name: 'Internet Protocol Version 6',
            fields: [{ name: 'Hop Limit', value: '64', range: [22, 23], expert: null }],
          },
        ],
      }),
    });

    expect(screen.getByText('IP Header')).toBeInTheDocument();
    expect(screen.getByText('TTL / Hop Limit')).toBeInTheDocument();
    expect(screen.getByText('64')).toBeInTheDocument();
  });

  it('shows the IPv4 header checksum from the IP layer', () => {
    render(PacketOverview, {
      packet: makePacket({
        layers: [
          {
            name: 'Internet Protocol Version 4',
            fields: [
              { name: 'TTL', value: '128', range: [22, 23], expert: null },
              {
                name: 'Header Checksum',
                value: '0xb861 (correct)',
                range: [24, 26],
                expert: null,
              },
            ],
          },
          {
            name: 'Transmission Control Protocol',
            fields: [
              { name: 'Checksum', value: '0x1234 (unverified)', range: [50, 52], expert: null },
            ],
          },
        ],
      }),
    });

    expect(screen.getByText('0xb861 (correct)')).toBeInTheDocument();
    // The transport checksum must not be shown as the IP header's.
    expect(screen.queryByText('0x1234 (unverified)')).not.toBeInTheDocument();
  });

  it('hides the risk badge when nothing was found', () => {
    // The backend scores ordinary packets 0; a non-zero score means the
    // dissector found something notable.
    render(PacketOverview, { packet: makePacket({ riskScore: 0 }) });

    expect(screen.queryByText(/Risk:/)).not.toBeInTheDocument();
  });

  it('shows a non-zero risk score', () => {
    render(PacketOverview, { packet: makePacket({ riskScore: 20 }) });

    expect(screen.getByText('Risk: 20/100')).toBeInTheDocument();
  });

  it('shows the risk badge when the score is elevated', () => {
    render(PacketOverview, { packet: makePacket({ riskScore: 70 }) });

    expect(screen.getByText('Risk: 70/100')).toBeInTheDocument();
  });

  it('hides the TCP and IP panels when those layers are absent', () => {
    render(PacketOverview, {
      packet: makePacket({
        summary: { protocol: 'ARP', source_addr: '', dest_addr: '' },
        layers: [
          {
            name: 'Ethernet',
            fields: [{ name: 'Type', value: '0x0806', range: [12, 14], expert: null }],
          },
        ],
      }),
    });

    expect(screen.queryByText('TCP Details')).not.toBeInTheDocument();
    expect(screen.queryByText('IP Header')).not.toBeInTheDocument();
  });
});
