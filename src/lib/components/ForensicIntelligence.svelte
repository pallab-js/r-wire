<script lang="ts">
  import type { ForensicIntelligence } from '../stores';

  export let intelligence: ForensicIntelligence;

  function getRiskColor(score: number) {
    if (score < 30) return 'text-[#4ec9b0]';
    if (score < 70) return 'text-[#d7ba7d]';
    return 'text-[#ff6b6b]';
  }

  function getEntropyDescription(entropy: number) {
    if (entropy > 7.5) return 'Very High (Likely Encrypted/Compressed)';
    if (entropy > 6.0) return 'High (Structured Data)';
    if (entropy > 4.0) return 'Medium (Normal Text/Headers)';
    return 'Low (Sparse Data)';
  }
</script>

<div class="flex flex-col h-full bg-[#1e1e1e] p-6 overflow-y-auto font-sans text-[#d4d4d4]">
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-4xl">
    <!-- Risk Score Gauge -->
    <div class="bg-[#252526] p-6 rounded border border-[#3e3e3e] flex flex-col items-center justify-center text-center">
      <h3 class="text-[#888] text-xs font-bold uppercase tracking-widest mb-4">Flow Risk Assessment</h3>
      <div class="text-5xl font-black mb-2 {getRiskColor(intelligence.risk_score)}">
        {intelligence.risk_score}<span class="text-xl opacity-50">/100</span>
      </div>
      <p class="text-sm font-medium opacity-80">Deterministic algorithmic score</p>
    </div>

    <!-- Entropy Analysis -->
    <div class="bg-[#252526] p-6 rounded border border-[#3e3e3e]">
      <h3 class="text-[#888] text-xs font-bold uppercase tracking-widest mb-4">Payload Entropy</h3>
      <div class="flex items-end gap-3 mb-3">
        <div class="text-3xl font-bold text-[#9cdcfe]">{intelligence.entropy.toFixed(4)}</div>
        <span class="text-xs pb-1 opacity-50">bits per byte</span>
      </div>
      <div class="w-full bg-[#1e1e1e] h-2 rounded-full overflow-hidden mb-3">
        <div class="h-full bg-[#007acc] transition-all duration-500" style="width: {(intelligence.entropy / 8) * 100}%"></div>
      </div>
      <p class="text-sm italic text-[#4ec9b0]">{getEntropyDescription(intelligence.entropy)}</p>
    </div>

    <!-- Fingerprints & Metadata -->
    <div class="bg-[#252526] p-6 rounded border border-[#3e3e3e] md:col-span-2">
      <h3 class="text-[#888] text-xs font-bold uppercase tracking-widest mb-4">Local Fingerprints</h3>
      <div class="flex flex-col gap-4">
        <div class="flex flex-col gap-1">
          <span class="text-xs font-bold text-[#569cd6] uppercase">TLS JA3 Fingerprint</span>
          <code class="bg-[#1e1e1e] p-3 rounded border border-white/5 font-mono text-sm break-all">
            {intelligence.ja3_hash || 'No TLS Handshake Detected'}
          </code>
        </div>
        <div class="flex flex-col gap-1">
          <span class="text-xs font-bold text-[#569cd6] uppercase">Device Manufacturer (OUI)</span>
          <div class="bg-[#1e1e1e] p-3 rounded border border-white/5 text-sm">
            {intelligence.manufacturer || 'Unknown / Local Address'}
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
