use std::fs::File;
use std::io::Write;
use std::path::Path;

// PCAP Global Header (24 bytes)
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

/// Appends one PCAP record.
///
/// `original_len` is the length the frame had on the wire. With a snaplen the
/// stored bytes are usually shorter than that, and a record that reports the
/// captured length as the original one (as this used to) tells every downstream
/// tool that an untruncated 9000-byte frame was only ever 1600 bytes long.
pub fn write_packet(
    file: &mut File,
    packet_data: &[u8],
    timestamp_sec: u32,
    timestamp_usec: u32,
    original_len: u32,
) -> std::io::Result<()> {
    let captured_len = packet_data.len() as u32;
    // PCAP requires orig_len >= incl_len; clamp rather than emit a record that
    // claims the frame was shorter than what we saved of it.
    let original_len = original_len.max(captured_len);

    // Packet header (16 bytes)
    file.write_all(&timestamp_sec.to_le_bytes())?;
    file.write_all(&timestamp_usec.to_le_bytes())?;
    file.write_all(&captured_len.to_le_bytes())?;
    file.write_all(&original_len.to_le_bytes())?;

    // Packet data
    file.write_all(packet_data)?;

    Ok(())
}

/// Streams PCAP records to disk one packet at a time.
///
/// This replaces `export_pcap_db`, which materialised every packet — summary
/// plus full payload — into a `Vec` before writing the first byte. For a large
/// capture that meant holding the entire database in RAM alongside SQLite's
/// own page cache, which is exactly how an export took the app down with it.
pub struct PcapWriter {
    file: File,
    packets_written: usize,
}

impl PcapWriter {
    /// Creates the file and writes the 24-byte PCAP global header.
    pub fn create(path: &Path) -> Result<Self, String> {
        let mut file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
        write_pcap_header(&mut file).map_err(|e| format!("Failed to write PCAP header: {}", e))?;
        Ok(Self {
            file,
            packets_written: 0,
        })
    }

    /// Appends one packet. `timestamp_ns` is nanoseconds since the epoch and
    /// `original_len` is the frame's length on the wire (it may exceed
    /// `data.len()` when the capture snaplen truncated the packet).
    pub fn write(
        &mut self,
        timestamp_ns: i64,
        data: &[u8],
        original_len: u32,
    ) -> Result<(), String> {
        let timestamp_sec = (timestamp_ns / 1_000_000_000) as u32;
        let timestamp_usec = ((timestamp_ns % 1_000_000_000) / 1_000).min(999_999) as u32;

        write_packet(
            &mut self.file,
            data,
            timestamp_sec,
            timestamp_usec,
            original_len,
        )
        .map_err(|e| format!("Failed to write packet {}: {}", self.packets_written + 1, e))?;
        self.packets_written += 1;
        Ok(())
    }

    pub fn packets_written(&self) -> usize {
        self.packets_written
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Unique-per-test name: parallel tests must not share the scratch file.
    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("auracap-{}-{}.pcap", std::process::id(), name))
    }

    #[test]
    fn writer_emits_global_header_then_packed_records() {
        let path = scratch("writer");
        let _ = fs::remove_file(&path);

        let mut writer = PcapWriter::create(&path).unwrap();
        writer
            .write(1_600_000_000_123_456_789, &[0u8; 10], 10)
            .unwrap();
        writer
            .write(1_600_000_001_000_000_000, &[1u8; 5], 5)
            .unwrap();
        assert_eq!(writer.packets_written(), 2);

        let bytes = fs::read(&path).unwrap();

        // 24-byte global header
        assert_eq!(bytes.len(), 24 + (16 + 10) + (16 + 5));
        assert_eq!(&bytes[0..4], &PCAP_MAGIC.to_le_bytes());

        // First record at offset 24
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            1_600_000_000
        );
        assert_eq!(
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
            123_456
        );
        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 10);

        // Second record immediately after the first payload
        let offset = 24 + 16 + 10;
        assert_eq!(
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
            1_600_000_001
        );
        assert_eq!(
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()),
            0
        );
        assert_eq!(
            u32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()),
            5
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn truncated_packets_keep_their_wire_length() {
        let path = scratch("truncated");
        let _ = fs::remove_file(&path);

        // 10 bytes survived the snaplen, the frame was 9000 bytes long.
        let mut writer = PcapWriter::create(&path).unwrap();
        writer
            .write(1_600_000_000_000_000_000, &[0u8; 10], 9000)
            .unwrap();

        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
            10,
            "incl_len is what was actually captured"
        );
        assert_eq!(
            u32::from_le_bytes(bytes[36..40].try_into().unwrap()),
            9000,
            "orig_len records the length on the wire"
        );
        assert_eq!(bytes.len(), 24 + 16 + 10);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn orig_len_never_claims_fewer_bytes_than_were_captured() {
        let path = scratch("clamped");
        let _ = fs::remove_file(&path);

        // Missing/short original length (0) must not produce a record that
        // claims the frame was shorter than the bytes stored after it.
        let mut writer = PcapWriter::create(&path).unwrap();
        writer.write(0, &[7u8; 16], 0).unwrap();

        let bytes = fs::read(&path).unwrap();
        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 16);
        assert_eq!(u32::from_le_bytes(bytes[36..40].try_into().unwrap()), 16);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn create_fails_for_missing_directory() {
        let path = std::env::temp_dir().join("no-such-dir-xyz").join("x.pcap");
        assert!(PcapWriter::create(&path).is_err());
    }
}
