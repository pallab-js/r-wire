<script lang="ts">
  import type { ForensicIntelligence } from '../stores';

  export let intelligence: ForensicIntelligence;

  function getRiskColor(score: number) {
    if (score < 30) return 'text-[var(--color-success)]';
    if (score < 70) return 'text-[var(--color-thinking)]';
    return 'text-[var(--color-error)]';
  }

  function getEntropyDescription(entropy: number) {
    if (entropy > 7.5) return 'Very High (Likely Encrypted/Compressed)';
    if (entropy > 6.0) return 'High (Structured Data)';
    if (entropy > 4.0) return 'Medium (Normal Text/Headers)';
    return 'Low (Sparse Data)';
  }
</script>

<div class="flex flex-col h-full p-6 overflow-y-auto" style="background-color: var(--cursor-cream); color: var(--cursor-dark);">
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-4xl">
    <!-- Risk Score Gauge -->
    <div class="p-6 rounded border flex flex-col items-center justify-center text-center" style="background-color: var(--surface-200); border-color: var(--border-primary);">
      <h3 class="text-xs font-bold uppercase tracking-widest mb-4" style="color: rgba(38, 37, 30, 0.55); font-family: var(--font-mono);">Flow Risk Assessment</h3>
      <div class="text-5xl font-black mb-2 {getRiskColor(intelligence.risk_score)}">
        {intelligence.risk_score}<span class="text-xl opacity-50">/100</span>
      </div>
      <p class="text-sm font-medium opacity-80" style="font-family: var(--font-gothic);">Deterministic algorithmic score</p>
    </div>

    <!-- Entropy Analysis -->
    <div class="p-6 rounded border" style="background-color: var(--surface-200); border-color: var(--border-primary);">
      <h3 class="text-xs font-bold uppercase tracking-widest mb-4" style="color: rgba(38, 37, 30, 0.55); font-family: var(--font-mono);">Payload Entropy</h3>
      <div class="flex items-end gap-3 mb-3">
        <div class="text-3xl font-bold" style="color: var(--cursor-dark); font-family: var(--font-gothic);">{intelligence.entropy.toFixed(4)}</div>
        <span class="text-xs pb-1 opacity-50" style="font-family: var(--font-mono);">bits per byte</span>
      </div>
      <div class="w-full h-2 rounded-full overflow-hidden mb-3" style="background-color: var(--surface-100);">
        <div class="h-full transition-all duration-500" style="width: {(intelligence.entropy / 8) * 100}%; background-color: var(--color-read);"></div>
      </div>
      <p class="text-sm italic" style="color: var(--color-success); font-family: var(--font-gothic);">{getEntropyDescription(intelligence.entropy)}</p>
    </div>

    <!-- Fingerprints & Metadata -->
    <div class="p-6 rounded border md:col-span-2" style="background-color: var(--surface-200); border-color: var(--border-primary);">
      <h3 class="text-xs font-bold uppercase tracking-widest mb-4" style="color: rgba(38, 37, 30, 0.55); font-family: var(--font-mono);">Local Fingerprints</h3>
      <div class="flex flex-col gap-4">
        <div class="flex flex-col gap-1">
          <span class="text-xs font-bold uppercase" style="color: var(--color-read); font-family: var(--font-mono);">TLS JA3 Fingerprint</span>
          <code class="p-3 rounded font-mono text-sm break-all" style="background-color: var(--surface-100); border: 1px solid var(--border-primary); font-family: var(--font-mono);">
            {intelligence.ja3_hash || 'No TLS Handshake Detected'}
          </code>
        </div>
        <div class="flex flex-col gap-1">
          <span class="text-xs font-bold uppercase" style="color: var(--color-read); font-family: var(--font-mono);">Device Manufacturer (OUI)</span>
          <div class="p-3 rounded text-sm" style="background-color: var(--surface-100); border: 1px solid var(--border-primary); font-family: var(--font-gothic);">
            {intelligence.manufacturer || 'Unknown / Local Address'}
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
