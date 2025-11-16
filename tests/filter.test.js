// Basic test utilities for TypeScript code
// This is a simple test runner - in a real project you'd use Jest, Vitest, etc.

interface PacketSummary {
    id: number;
    timestamp: number;
    source_addr: string;
    dest_addr: string;
    protocol: string;
    length: number;
    info: string;
}

function matchesFilter(packet: PacketSummary, filter: string): boolean {
    if (!filter || filter.trim() === '') {
        return true;
    }

    const lowerFilter = filter.toLowerCase().trim();

    // Protocol filter
    if (lowerFilter.startsWith('protocol:')) {
        const protocol = lowerFilter.replace('protocol:', '').trim();
        if (!protocol) return false;
        return packet.protocol.toLowerCase().includes(protocol);
    }

    // IP address filter
    if (lowerFilter.startsWith('ip:')) {
        const ip = lowerFilter.replace('ip:', '').trim();
        if (!ip) return false;
        return packet.source_addr.includes(ip) || packet.dest_addr.includes(ip);
    }

    // Port filter
    if (lowerFilter.startsWith('port:')) {
        const port = lowerFilter.replace('port:', '').trim();
        if (!port) return false;
        return packet.info.includes(port);
    }

    // Source filter
    if (lowerFilter.startsWith('src:')) {
        const src = lowerFilter.replace('src:', '').trim();
        if (!src) return false;
        return packet.source_addr.toLowerCase().includes(src);
    }

    // Destination filter
    if (lowerFilter.startsWith('dst:')) {
        const dst = lowerFilter.replace('dst:', '').trim();
        if (!dst) return false;
        return packet.dest_addr.toLowerCase().includes(dst);
    }

    // General search
    return (
        packet.protocol.toLowerCase().includes(lowerFilter) ||
        packet.source_addr.toLowerCase().includes(lowerFilter) ||
        packet.dest_addr.toLowerCase().includes(lowerFilter) ||
        packet.info.toLowerCase().includes(lowerFilter) ||
        packet.length.toString().includes(lowerFilter)
    );
}

// Simple test runner
function assert(condition: boolean, message: string) {
    if (!condition) {
        throw new Error(`Test failed: ${message}`);
    }
}

function testFilter() {
    console.log('Running filter tests...');

    const packet: PacketSummary = {
        id: 1,
        timestamp: 1000000000,
        source_addr: '192.168.1.1',
        dest_addr: '192.168.1.2',
        protocol: 'TCP',
        length: 100,
        info: '80'
    };

    // Test protocol filter
    assert(matchesFilter(packet, 'protocol:tcp'), 'Protocol filter should match TCP');
    assert(!matchesFilter(packet, 'protocol:udp'), 'Protocol filter should not match UDP');
    assert(!matchesFilter(packet, 'protocol:'), 'Empty protocol filter should not match');

    // Test IP filter
    assert(matchesFilter(packet, 'ip:192.168.1.1'), 'IP filter should match source IP');
    assert(matchesFilter(packet, 'ip:192.168.1.2'), 'IP filter should match dest IP');
    assert(!matchesFilter(packet, 'ip:10.0.0.1'), 'IP filter should not match different IP');
    assert(!matchesFilter(packet, 'ip:'), 'Empty IP filter should not match');

    // Test port filter
    assert(matchesFilter(packet, 'port:80'), 'Port filter should match port 80');
    assert(!matchesFilter(packet, 'port:443'), 'Port filter should not match port 443');
    assert(!matchesFilter(packet, 'port:'), 'Empty port filter should not match');

    // Test src/dst filters
    assert(matchesFilter(packet, 'src:192.168.1.1'), 'Source filter should match');
    assert(matchesFilter(packet, 'dst:192.168.1.2'), 'Dest filter should match');
    assert(!matchesFilter(packet, 'src:10.0.0.1'), 'Source filter should not match different IP');
    assert(!matchesFilter(packet, 'dst:10.0.0.1'), 'Dest filter should not match different IP');

    // Test general search
    assert(matchesFilter(packet, 'tcp'), 'General search should match protocol');
    assert(matchesFilter(packet, '192.168'), 'General search should match IP');
    assert(matchesFilter(packet, '80'), 'General search should match port');
    assert(!matchesFilter(packet, 'udp'), 'General search should not match different protocol');

    // Test empty/whitespace
    assert(matchesFilter(packet, ''), 'Empty filter should match');
    assert(matchesFilter(packet, '   '), 'Whitespace filter should match');

    console.log('All filter tests passed!');
}

// Run tests
if (typeof window === 'undefined') {
    // Node.js environment
    testFilter();
} else {
    // Browser environment - expose for manual testing
    (window as any).testFilter = testFilter;
    console.log('Filter tests loaded. Run testFilter() to execute tests.');
}