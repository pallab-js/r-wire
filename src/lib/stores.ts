import { writable, derived } from 'svelte/store';
import { calculateStatistics, type Statistics } from './utils/statistics';
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

export interface ProtocolLayer {
    name: string;
    fields: Array<[string, string]>; // (Field Name, Field Value)
}

export interface PacketDetail {
    summary: PacketSummary;
    layers: ProtocolLayer[];
    raw_bytes: number[]; // Vec<u8> as number[]
}

export const availableInterfaces = writable<string[]>([]);
export const selectedInterface = writable<string | null>(null);
export const isCapturing = writable<boolean>(false);

// Limit frontend packet list to prevent memory issues
const MAX_FRONTEND_PACKETS = 50_000;
const _packetListInternal = writable<PacketSummary[]>([]);

export const packetList = derived(_packetListInternal, ($list) => $list);

// Helper to add packets with limit
export function addPackets(newPackets: PacketSummary[]) {
    if (!newPackets || newPackets.length === 0) {
        return; // Early return for empty batches
    }
    
    _packetListInternal.update(list => {
        const updated = [...list, ...newPackets];
        // Keep only the most recent packets if over limit
        // Handle case where newPackets itself exceeds the limit
        if (updated.length > MAX_FRONTEND_PACKETS) {
            return updated.slice(-MAX_FRONTEND_PACKETS);
        }
        return updated;
    });
}

// Helper to set packet list (for clear operation)
export function setPacketList(packets: PacketSummary[]) {
    // Ensure we don't exceed the limit even when setting directly
    const limited = packets.length > MAX_FRONTEND_PACKETS 
        ? packets.slice(-MAX_FRONTEND_PACKETS)
        : packets;
    _packetListInternal.set(limited);
}

export const selectedPacket = writable<PacketDetail | null>(null);
export const captureError = writable<string | null>(null);

// Filter store with debouncing
export const displayFilter = writable<string>('');
const _debouncedFilterInternal = writable<string>('');
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let debounceUnsubscribe: (() => void) | null = null;

// Initialize debounced filter with current filter value
_debouncedFilterInternal.set('');

// Update debounced filter when displayFilter changes
// Note: This subscription persists for the app lifetime, which is acceptable
// The cleanup on beforeunload is a safety measure
debounceUnsubscribe = displayFilter.subscribe(filter => {
    if (debounceTimer) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
    }
    debounceTimer = setTimeout(() => {
        _debouncedFilterInternal.set(filter);
        debounceTimer = null;
    }, 200); // 200ms debounce
});

// Cleanup function (called when page unloads)
if (typeof window !== 'undefined') {
    window.addEventListener('beforeunload', () => {
        if (debounceTimer) {
            clearTimeout(debounceTimer);
            debounceTimer = null;
        }
        if (debounceUnsubscribe) {
            debounceUnsubscribe();
            debounceUnsubscribe = null;
        }
    });
}

export const debouncedFilter = derived(_debouncedFilterInternal, ($filter) => $filter);

// Memoized statistics - only recalculates when packetList changes
export const statistics = derived(packetList, ($packetList) => 
    calculateStatistics($packetList)
);

// Memoized filtered packets - only recalculates when packetList or filter changes
export const filteredPackets = derived(
    [packetList, debouncedFilter],
    ([$packetList, $debouncedFilter]) => 
        getFilteredPackets($packetList, $debouncedFilter)
);
