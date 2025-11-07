use serde::{Serialize, Deserialize};

// Cached packet data with timestamp
#[derive(Debug, Clone)]
pub struct CachedPacket {
    pub data: Vec<u8>,
    pub timestamp_ns: i64,
}

// Summary for the main packet list (F3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSummary {
    pub id: u64,
    pub timestamp: i64, // Unix nanoseconds
    pub source_addr: String,
    pub dest_addr: String,
    pub protocol: String,
    pub length: u32,
    pub info: String,
}

// Detailed view for protocol tree (F4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketDetail {
    pub summary: PacketSummary,
    pub layers: Vec<ProtocolLayer>,
    pub raw_bytes: Vec<u8>, // For the hex view (F5)
}

// A generic, serializable representation of a parsed protocol layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolLayer {
    pub name: String,
    pub fields: Vec<(String, String)>, // (Field Name, Field Value)
}
