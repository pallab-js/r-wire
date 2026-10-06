<script lang="ts">
  import type { PacketDetail } from '../stores';

  export let packet: PacketDetail;

  function findFieldInLayers(fieldName: string, caseInsensitive = true): string | null {
    for (const layer of packet.layers) {
      for (const field of layer.fields) {
        if (caseInsensitive) {
          if (field.name.toLowerCase().includes(fieldName.toLowerCase())) {
            return field.value;
          }
        } else {
          if (field.name === fieldName) {
            return field.value;
          }
        }
      }
    }
    return null;
  }

  function findFieldInLayer(layerName: string, fieldName: string): string | null {
    const layer = packet.layers.find((l) => l.name === layerName);
    if (!layer) return null;
    const field = layer.fields.find((f) => f.name.toLowerCase().includes(fieldName.toLowerCase()));
    return field?.value ?? null;
  }

  function parseIpPort(field: string): { ip: string; port: string } | null {
    const match = field.match(/^([^:]+):(\d+)$/);
    if (match) {
      return { ip: match[1], port: match[2] };
    }
    const portMatch = field.match(/^(\d+\.\d+\.\d+\.\d+):(\d+)$/);
    if (portMatch) {
      return { ip: portMatch[1], port: portMatch[2] };
    }
    return null;
  }

  function formatPort(port?: number | null): string {
    return port == null ? '' : String(port);
  }

  $: sourceAddr = packet.summary.source_addr;
  $: destAddr = packet.summary.dest_addr;
  $: protocol = packet.summary.protocol;
  $: length = packet.summary.length;
  $: timestamp = packet.summary.timestamp;
  $: info = packet.summary.info;

  // `source_addr`/`dest_addr` are bare addresses, so they never carry a port to
  // parse. The ports come from the summary (filled in by the dissector).
  $: srcParsed = parseIpPort(sourceAddr);
  $: srcIp = srcParsed?.ip ?? sourceAddr;
  $: srcPort = formatPort(packet.summary.src_port) || srcParsed?.port || '';

  $: dstParsed = parseIpPort(destAddr);
  $: dstIp = dstParsed?.ip ?? destAddr;
  $: dstPort = formatPort(packet.summary.dst_port) || dstParsed?.port || '';

  $: tcpFlags = findFieldInLayers('flags') ?? '';
  $: seqNum = findFieldInLayers('seq') ?? '';
  $: ackNum = findFieldInLayers('ack') ?? '';
  $: window = findFieldInLayers('window') ?? '';
  // IPv6 calls this field "Hop Limit"; without that fallback the whole IP
  // header panel disappeared for IPv6 packets.
  $: ttl =
    findFieldInLayers('ttl') ??
    findFieldInLayers('time to live') ??
    findFieldInLayers('hop limit') ??
    '';
  $: id = findFieldInLayers('identification') ?? '';
  // Scoped to the IP layer so a transport checksum cannot be shown as the IP
  // header's. IPv6 has no header checksum, so the tile is correctly absent.
  $: checksum = findFieldInLayer('Internet Protocol Version 4', 'checksum') ?? '';

  // `summary.protocol` names the application protocol for port-based traffic
  // (HTTP, HTTPS, DNS), so gating on `protocol === 'TCP'` hid the TCP header
  // for every packet that rides on TCP.
  $: hasTcpLayer = packet.layers.some(
    (layer) => layer.name === 'Transmission Control Protocol' || layer.name === 'TCP',
  );
  $: hasIpLayer = packet.layers.some(
    (layer) =>
      layer.name === 'Internet Protocol Version 4' || layer.name === 'Internet Protocol Version 6',
  );

  // The backend scores ordinary packets 0, so `> 0` means the dissector
  // actually found something (bad checksum, low TTL, high-entropy payload,
  // file transfer...). It used to score every packet 10, which made any
  // non-zero gate light up on all packets.
  $: riskScore = packet.intelligence.risk_score;
  $: showRisk = riskScore > 0;
  $: riskStyle =
    riskScore >= 70
      ? 'background-color: #fee2e2; color: #dc2626;'
      : 'background-color: #fef3c7; color: #b45309;';

  // `ts` is nanoseconds since the epoch (exceeds Number.MAX_SAFE_INTEGER, so
  // only millisecond precision survives the IPC round-trip).
  function formatTimestamp(ts: number): string {
    if (!Number.isFinite(ts) || ts <= 0) return '-';
    const date = new Date(ts / 1_000_000);
    if (isNaN(date.getTime())) return '-';
    return date.toISOString().replace('T', ' ').replace('Z', '');
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function getProtocolColor(proto: string): string {
    const colors: Record<string, string> = {
      TCP: '#3b82f6',
      UDP: '#22c55e',
      ICMP: '#f59e0b',
      DNS: '#a855f7',
      HTTP: '#ec4899',
      HTTPS: '#ef4444',
      ARP: '#06b6d4',
    };
    return colors[proto.toUpperCase()] ?? '#6b7280';
  }
</script>

<div class="h-full overflow-y-auto" style="background-color: var(--bg-page);">
  <div class="max-w-5xl p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-start justify-between">
      <div class="flex items-center gap-3">
        <div
          class="px-3 py-1.5 rounded-md text-sm font-semibold"
          style="background-color: {getProtocolColor(protocol)}; color: white;"
        >
          {protocol}
        </div>
        <span class="text-lg font-semibold" style="color: var(--text-primary);"
          >Packet #{packet.summary.id}</span
        >
      </div>
      <div class="text-right">
        <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
          Captured
        </div>
        <div class="text-sm font-mono" style="color: var(--text-primary);">
          {formatTimestamp(timestamp)}
        </div>
      </div>
    </div>

    <!-- Connection Flow - Primary Info -->
    <div
      class="rounded-lg border p-4"
      style="background-color: var(--bg-card); border-color: var(--border-standard);"
    >
      <div class="flex items-center justify-between">
        <div class="flex-1">
          <div class="text-xs uppercase tracking-wide mb-1" style="color: var(--text-muted);">
            Source
          </div>
          <div class="text-lg font-mono font-semibold" style="color: var(--text-primary);">
            {srcIp}
          </div>
          {#if srcPort}
            <div class="text-sm font-mono" style="color: var(--brand-blue);">{srcPort}</div>
          {/if}
        </div>
        <div class="flex flex-col items-center px-4">
          <svg
            width="32"
            height="32"
            viewBox="0 0 24 24"
            fill="none"
            stroke="var(--brand-green)"
            stroke-width="2"
          >
            <path d="M5 12h14M12 5l7 7-7 7" />
          </svg>
          <div class="text-xs mt-1 uppercase tracking-wide" style="color: var(--text-muted);">
            {protocol}
          </div>
        </div>
        <div class="flex-1 text-right">
          <div class="text-xs uppercase tracking-wide mb-1" style="color: var(--text-muted);">
            Destination
          </div>
          <div class="text-lg font-mono font-semibold" style="color: var(--text-primary);">
            {dstIp}
          </div>
          {#if dstPort}
            <div class="text-sm font-mono" style="color: var(--brand-blue);">{dstPort}</div>
          {/if}
        </div>
      </div>
    </div>

    <!-- Packet Info Bar -->
    <div
      class="rounded-lg border p-3 flex items-center justify-between"
      style="background-color: var(--bg-card); border-color: var(--border-standard);"
    >
      <div class="flex items-center gap-6">
        <div>
          <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
            Length
          </div>
          <div class="text-lg font-mono font-semibold" style="color: var(--text-primary);">
            {length} bytes
          </div>
        </div>
        {#if info}
          <div>
            <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
              Info
            </div>
            <div class="text-sm max-w-md truncate" style="color: var(--text-primary);">{info}</div>
          </div>
        {/if}
      </div>
      {#if showRisk}
        <div class="flex items-center gap-2">
          <div class="px-3 py-1 rounded text-sm font-medium" style={riskStyle}>
            Risk: {riskScore}/100
          </div>
        </div>
      {/if}
    </div>

    <!-- Protocol-Specific Details -->
    {#if hasTcpLayer && (tcpFlags || seqNum || ackNum || window)}
      <div
        class="rounded-lg border p-4"
        style="background-color: var(--bg-card); border-color: var(--border-standard);"
      >
        <div
          class="text-sm font-semibold uppercase tracking-wide mb-3"
          style="color: var(--text-muted);"
        >
          TCP Details
        </div>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          {#if tcpFlags}
            <div>
              <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
                Flags
              </div>
              <div class="text-sm font-mono font-semibold" style="color: var(--brand-blue);">
                {tcpFlags}
              </div>
            </div>
          {/if}
          {#if seqNum}
            <div>
              <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
                Sequence
              </div>
              <div class="text-sm font-mono" style="color: var(--text-primary);">{seqNum}</div>
            </div>
          {/if}
          {#if ackNum}
            <div>
              <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
                Acknowledgment
              </div>
              <div class="text-sm font-mono" style="color: var(--text-primary);">{ackNum}</div>
            </div>
          {/if}
          {#if window}
            <div>
              <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
                Window
              </div>
              <div class="text-sm font-mono" style="color: var(--text-primary);">{window}</div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- IP Header Details -->
    {#if hasIpLayer && (ttl || id || checksum)}
      <div
        class="rounded-lg border p-4"
        style="background-color: var(--bg-card); border-color: var(--border-standard);"
      >
        <div
          class="text-sm font-semibold uppercase tracking-wide mb-3"
          style="color: var(--text-muted);"
        >
          IP Header
        </div>
        <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
          {#if ttl}
            <div>
              <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
                TTL / Hop Limit
              </div>
              <div class="text-sm font-mono" style="color: var(--text-primary);">{ttl}</div>
            </div>
          {/if}
          {#if id}
            <div>
              <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
                Identification
              </div>
              <div class="text-sm font-mono" style="color: var(--text-primary);">{id}</div>
            </div>
          {/if}
          {#if checksum}
            <div>
              <div class="text-xs uppercase tracking-wide" style="color: var(--text-muted);">
                Checksum
              </div>
              <div class="text-sm font-mono" style="color: var(--text-primary);">{checksum}</div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Expert Info / Warnings -->
    {#if packet.expert_summary && packet.expert_summary.length > 0}
      <div class="rounded-lg border p-4" style="background-color: #fef2f2; border-color: #fecaca;">
        <div class="flex items-center gap-2 mb-3">
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="#dc2626"
            stroke-width="2"
          >
            <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
            <path d="M12 9v4" />
            <path d="M12 17h.01" />
          </svg>
          <div class="text-sm font-semibold" style="color: #dc2626;">Expert Information</div>
        </div>
        <div class="flex flex-col gap-2">
          {#each packet.expert_summary as expert}
            <div class="text-sm" style="color: #b91c1c;">{expert}</div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Forensic Context -->
    {#if packet.narrative.summary}
      <div
        class="rounded-lg border p-4"
        style="background-color: var(--bg-card); border-color: var(--border-standard);"
      >
        <div
          class="text-sm font-semibold uppercase tracking-wide mb-3"
          style="color: var(--brand-green);"
        >
          Analysis
        </div>
        <p class="text-base leading-relaxed mb-4" style="color: var(--text-primary);">
          {packet.narrative.summary}
        </p>
        {#if packet.narrative.technical_details.length > 0}
          <div class="space-y-2">
            {#each packet.narrative.technical_details as detail}
              <div class="flex items-start gap-2 text-sm" style="color: var(--text-muted);">
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  class="mt-0.5 shrink-0"
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
                <span>{detail}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Artifacts -->
    {#if packet.artifacts && packet.artifacts.length > 0}
      <div
        class="rounded-lg border p-4"
        style="background-color: var(--bg-card); border-color: var(--border-standard);"
      >
        <div
          class="text-sm font-semibold uppercase tracking-wide mb-3"
          style="color: var(--text-muted);"
        >
          Detected Artifacts
        </div>
        <div class="space-y-2">
          {#each packet.artifacts as artifact}
            <div class="flex items-center justify-between text-sm">
              <div>
                <span class="font-medium" style="color: var(--text-primary);">{artifact.name}</span>
                <span
                  class="ml-2 px-2 py-0.5 rounded text-xs"
                  style="background-color: var(--bg-button); color: var(--text-muted);"
                >
                  {artifact.mime_type}
                </span>
              </div>
              <span class="font-mono" style="color: var(--text-muted);"
                >{formatSize(artifact.size)}</span
              >
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
