use serde::{Serialize, Deserialize};

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
}

/// A protocol layer with its parsed fields.
///
/// Represents a single layer in the protocol stack (e.g., Ethernet, IP, TCP)
/// with all its parsed header fields. Used to build the protocol tree view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolLayer {
    /// Human-readable protocol name (e.g., "Transmission Control Protocol")
    pub name: String,
    /// List of (field_name, field_value) pairs for this protocol layer
    pub fields: Vec<(String, String)>,
}
