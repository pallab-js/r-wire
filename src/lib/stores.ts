import { writable, derived, get } from 'svelte/store';
import { updateStatistics, resetStatistics, createEmptyStatistics, type Statistics } from './utils/statistics';
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
export const statistics = writable<Statistics>(createEmptyStatistics());
export const filteredPackets = writable<PacketSummary[]>([]);

// Filter store with debouncing
export const displayFilter = writable<string>('');
const _debouncedFilterInternal = writable<string>('');
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

// Handle filter changes efficiently
let currentFilter = '';
let currentPackets: PacketSummary[] = [];

_debouncedFilterInternal.subscribe(filter => {
    currentFilter = filter;
    filteredPackets.set(getFilteredPackets(currentPackets, filter));
});

// Helper to add packets with limit
export function addPackets(newPackets: PacketSummary[]) {
    if (!newPackets || newPackets.length === 0) {
        return;
    }
    
    _packetListInternal.update(list => {
        const updated = [...list, ...newPackets];
        if (updated.length > MAX_FRONTEND_PACKETS) {
            currentPackets = updated.slice(-MAX_FRONTEND_PACKETS);
        } else {
            currentPackets = updated;
        }
        return currentPackets;
    });

    // Update filtered packets incrementally
    const matchingNew = getFilteredPackets(newPackets, currentFilter);
    if (matchingNew.length > 0 || currentFilter === '') {
        filteredPackets.update(list => {
            const updated = [...list, ...matchingNew];
            if (updated.length > MAX_FRONTEND_PACKETS) {
                return updated.slice(-MAX_FRONTEND_PACKETS);
            }
            return updated;
        });
    }

    // Update statistics incrementally
    statistics.update(s => updateStatistics(s, newPackets));
}

// Helper to set packet list (for clear operation)
export function setPacketList(packets: PacketSummary[]) {
    const limited = packets.length > MAX_FRONTEND_PACKETS 
        ? packets.slice(-MAX_FRONTEND_PACKETS)
        : packets;
    currentPackets = limited;
    _packetListInternal.set(limited);
    filteredPackets.set(getFilteredPackets(limited, currentFilter));
    
    // Reset stats if clearing
    if (packets.length === 0) {
        resetStatistics();
        statistics.set(createEmptyStatistics());
    }
}

export const selectedPacket = writable<PacketDetail | null>(null);
export const captureError = writable<string | null>(null);

// Initialize debounced filter with current filter value
_debouncedFilterInternal.set('');

// Update debounced filter when displayFilter changes
let debounceUnsubscribe = displayFilter.subscribe(filter => {
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
