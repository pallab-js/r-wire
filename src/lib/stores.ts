import { writable, derived } from 'svelte/store';
import {
  updateStatistics,
  resetStatistics,
  createEmptyStatistics,
  type Statistics,
} from './utils/statistics';
import { getFilteredPackets } from './utils/filter';

export interface PacketSummary {
  id: number;
  timestamp: number; // i64 as number
  source_addr: string;
  dest_addr: string;
  protocol: string;
  length: number;
  info: string;
}

export interface PacketField {
  name: string;
  value: string;
  range: [number, number]; // (start, end)
  expert: string | null;
}

export interface ProtocolLayer {
  name: string;
  fields: PacketField[];
}

export interface ForensicNarrative {
  summary: string;
  technical_details: string[];
}

export interface ForensicIntelligence {
  entropy: number;
  ja3_hash: string | null;
  manufacturer: string | null;
  risk_score: number;
}

export interface Artifact {
  name: string;
  mime_type: string;
  size: number;
  hash_sha256: string;
}

export interface PacketDetail {
  summary: PacketSummary;
  layers: ProtocolLayer[];
  raw_bytes: number[]; // Vec<u8> as number[]
  expert_summary: string[];
  narrative: ForensicNarrative;
  intelligence: ForensicIntelligence;
  artifacts: Artifact[];
}

export const availableInterfaces = writable<string[]>([]);
export const selectedInterface = writable<string | null>(null);
export const isCapturing = writable<boolean>(false);

const _packetListInternal = writable<PacketSummary[]>([]);

export const packetList = derived(_packetListInternal, ($list) => $list);
export const statistics = writable<Statistics>(createEmptyStatistics());
export const filteredPackets = writable<PacketSummary[]>([]);
export const totalFilteredCount = writable<number>(0);

// Filter store with debouncing
export const displayFilter = writable<string>('');
export const bpfFilter = writable<string>('');
const _debouncedFilterInternal = writable<string>('');
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

// Handle filter changes efficiently
let currentFilter = '';

_debouncedFilterInternal.subscribe((filter) => {
  currentFilter = filter;
  // We don't filter client-side anymore, the component will fetch from backend
});

// Helper to add packets with limit
export function addPackets(newPackets: PacketSummary[]) {
  if (!newPackets || newPackets.length === 0) {
    return;
  }

  // We still update statistics incrementally
  statistics.update((s) => updateStatistics(s, newPackets));

  // For the UI, we increment the total count based on matching filter
  const matchingNew = getFilteredPackets(newPackets, currentFilter);
  if (matchingNew.length > 0 || currentFilter === '') {
    totalFilteredCount.update((n) => n + matchingNew.length);
  }
}

// Helper to set packet list (for clear operation)
export function setPacketList(packets: PacketSummary[]) {
  if (packets.length === 0) {
    _packetListInternal.set([]);
    filteredPackets.set([]);
    totalFilteredCount.set(0);
    resetStatistics();
    statistics.set(createEmptyStatistics());
  } else {
    // If we are setting a list (e.g. from an import/load), we might still want it in memory for now
    // or just set the count.
    _packetListInternal.set(packets);
    totalFilteredCount.set(packets.length);
  }
}

export interface StreamMessage {
  is_client: boolean;
  data: number[]; // Vec<u8> as number[]
  timestamp: number;
}

export const selectedPacket = writable<PacketDetail | null>(null);
export const selectedStream = writable<StreamMessage[] | null>(null);
export const captureError = writable<string | null>(null);
export const highlightedRange = writable<[number, number] | null>(null);

// Initialize debounced filter with current filter value
_debouncedFilterInternal.set('');

// Update debounced filter when displayFilter changes
const debounceUnsubscribe = displayFilter.subscribe((filter) => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
  debounceTimer = setTimeout(() => {
    _debouncedFilterInternal.set(filter);
    debounceTimer = null;
  }, 200); // 200ms debounce
});

// Cleanup function
if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', () => {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    if (debounceUnsubscribe) {
      debounceUnsubscribe();
    }
  });
}

export const debouncedFilter = derived(_debouncedFilterInternal, ($filter) => $filter);
