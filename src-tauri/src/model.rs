use serde::{Deserialize, Serialize};

/// Cached packet data with timestamp for efficient storage and retrieval.
///
/// This structure holds the raw packet bytes along with their precise capture timestamp.
/// Used internally by the packet cache to avoid re-parsing packets.
#[derive(Debug, Clone)]
pub struct CachedPacket {
    /// Raw packet data as received from libpcap
    pub data: Vec<u8>,
    /// Capture timestamp in nanoseconds since Unix epoch
    pub timestamp_ns: i64,
}

/// Summary information for packets displayed in the main packet list.
///
/// This lightweight structure contains the essential information needed for
/// the packet list view, avoiding the overhead of full protocol dissection
/// for every packet in large captures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSummary {
    /// Unique packet identifier (sequential)
    pub id: u64,
    /// Packet capture timestamp in nanoseconds since Unix epoch
    pub timestamp: i64,
    /// Source address (IP or MAC depending on protocol)
    pub source_addr: String,
    /// Destination address (IP or MAC depending on protocol)
    pub dest_addr: String,
    /// Protocol name (TCP, UDP, ICMP, ARP, etc.)
    pub protocol: String,
    /// Packet length in bytes
    pub length: u32,
    /// Human-readable packet description/information
    pub info: String,
}

/// Detailed packet analysis with full protocol dissection.
///
/// Contains the complete analysis of a packet including all protocol layers
/// and the raw bytes for hex display. This is the full detail view shown
/// when a user selects a specific packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketDetail {
    /// Basic packet summary information
    pub summary: PacketSummary,
    /// Protocol layers with detailed field information
    pub layers: Vec<ProtocolLayer>,
    /// Raw packet bytes for hex view display
    pub raw_bytes: Vec<u8>,
    /// High-level expert analysis/warnings
    pub expert_summary: Vec<String>,
    /// Human-readable narrative explanation
    pub narrative: ForensicNarrative,
    /// Forensic intelligence metadata (Entropy, JA3, etc)
    pub intelligence: ForensicIntelligence,
    /// Extracted objects/files from this packet or its stream
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicNarrative {
    pub summary: String,
    pub technical_details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicIntelligence {
    pub entropy: f64,
    pub ja3_hash: Option<String>,
    pub manufacturer: Option<String>,
    pub risk_score: u8, // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub mime_type: String,
    pub size: usize,
    pub hash_sha256: String,
}

/// A protocol layer with its parsed fields.
///
/// Represents a single layer in the protocol stack (e.g., Ethernet, IP, TCP)
/// with all its parsed header fields. Used to build the protocol tree view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolLayer {
    /// Human-readable protocol name (e.g., "Transmission Control Protocol")
    pub name: String,
    /// List of parsed fields for this protocol layer
    pub fields: Vec<PacketField>,
}

/// A message in a reassembled stream (e.g., TCP or UDP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    /// The side that sent the message (true for client -> server, false for server -> client)
    pub is_client: bool,
    /// The reassembled payload data
    pub data: Vec<u8>,
    /// Timestamp of the first packet in this message
    pub timestamp: i64,
}

/// A single field within a protocol layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketField {
    /// Name of the field (e.g., "Destination IP")
    pub name: String,
    /// Formatted value of the field (e.g., "192.168.1.1")
    pub value: String,
    /// The byte range in the raw packet corresponding to this field (start, end)
    pub range: (usize, usize),
    /// Optional expert info for this specific field
    pub expert: Option<String>,
}
