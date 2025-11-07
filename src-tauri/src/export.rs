use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use crate::model::{PacketSummary, CachedPacket};
use std::collections::BTreeMap;

// PCAP Global Header (24 bytes)
// Magic number: 0xA1B2C3D4 (nanosecond timestamps) or 0xA1B2C3D4 (microsecond timestamps)
// We'll use microsecond timestamps (0xA1B2C3D4)
const PCAP_MAGIC: u32 = 0xA1B2C3D4;
const PCAP_VERSION_MAJOR: u16 = 2;
const PCAP_VERSION_MINOR: u16 = 4;
const PCAP_THISZONE: i32 = 0; // GMT to local correction
const PCAP_SIGFIGS: u32 = 0; // Accuracy of timestamps
const PCAP_SNAPLEN: u32 = 65535; // Max length of captured packets
const PCAP_NETWORK: u32 = 1; // Ethernet (DLT_EN10MB)

pub fn write_pcap_header(file: &mut File) -> std::io::Result<()> {
    file.write_all(&PCAP_MAGIC.to_le_bytes())?;
    file.write_all(&PCAP_VERSION_MAJOR.to_le_bytes())?;
    file.write_all(&PCAP_VERSION_MINOR.to_le_bytes())?;
    file.write_all(&PCAP_THISZONE.to_le_bytes())?;
    file.write_all(&PCAP_SIGFIGS.to_le_bytes())?;
    file.write_all(&PCAP_SNAPLEN.to_le_bytes())?;
    file.write_all(&PCAP_NETWORK.to_le_bytes())?;
    Ok(())
}

pub fn write_packet(
    file: &mut File,
    packet_data: &[u8],
    timestamp_sec: u32,
    timestamp_usec: u32,
) -> std::io::Result<()> {
    let captured_len = packet_data.len() as u32;
    let original_len = captured_len;

    // Packet header (16 bytes)
    file.write_all(&timestamp_sec.to_le_bytes())?;
    file.write_all(&timestamp_usec.to_le_bytes())?;
    file.write_all(&captured_len.to_le_bytes())?;
    file.write_all(&original_len.to_le_bytes())?;

    // Packet data
    file.write_all(packet_data)?;

    Ok(())
}

pub fn export_pcap(
    packet_cache: &BTreeMap<u64, CachedPacket>,
    packet_list: &[PacketSummary],
    file_path: PathBuf,
) -> Result<(), String> {
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    // Write PCAP global header
    write_pcap_header(&mut file)
        .map_err(|e| format!("Failed to write PCAP header: {}", e))?;

    // Write packets in order
    for packet in packet_list {
        if let Some(cached) = packet_cache.get(&packet.id) {
            // Convert nanoseconds to seconds and microseconds
            // Handle potential overflow: u32 max is ~4.2 billion seconds (year 2106)
            let timestamp_ns = cached.timestamp_ns;
            let timestamp_sec = (timestamp_ns / 1_000_000_000) as u32;
            // Ensure microseconds don't exceed 999,999
            let timestamp_usec = ((timestamp_ns % 1_000_000_000) / 1_000).min(999_999) as u32;

            write_packet(&mut file, &cached.data, timestamp_sec, timestamp_usec)
                .map_err(|e| format!("Failed to write packet {}: {}", packet.id, e))?;
        }
    }

    Ok(())
}
