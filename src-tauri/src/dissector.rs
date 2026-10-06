use crate::model::{
    Artifact, ForensicIntelligence, ForensicNarrative, PacketDetail, PacketField, PacketSummary,
    ProtocolLayer,
};
use crate::state::FlowKey;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;
use sha2::{Digest, Sha256};
use std::net::IpAddr;

// Protocol name constants to avoid repeated string allocations
const PROTO_TCP: &str = "TCP";
const PROTO_UDP: &str = "UDP";
const PROTO_ICMP: &str = "ICMP";
const PROTO_ICMPV6: &str = "ICMPv6";
const PROTO_IPV4: &str = "IPv4";
const PROTO_IPV6: &str = "IPv6";
const PROTO_ARP: &str = "ARP";
const PROTO_HTTP: &str = "HTTP";
const PROTO_HTTPS: &str = "HTTPS";
const PROTO_UNKNOWN: &str = "Unknown";

/// Local OUI database for standalone manufacturer identification
fn get_manufacturer(mac: &str) -> Option<String> {
    let prefix = mac.replace(':', "").to_uppercase();
    if prefix.len() < 6 {
        return None;
    }
    let oui = &prefix[0..6];

    match oui {
        "00000C" => Some("Cisco".to_string()),
        "0005CD" => Some("Apple".to_string()),
        "000C29" => Some("VMware".to_string()),
        "00155D" => Some("Microsoft".to_string()),
        "005056" => Some("VMware".to_string()),
        "00163E" => Some("Xen".to_string()),
        "080027" => Some("Oracle (VirtualBox)".to_string()),
        "3C22FB" => Some("Apple".to_string()),
        "B827EB" => Some("Raspberry Pi".to_string()),
        "DCA632" => Some("Raspberry Pi".to_string()),
        _ => None,
    }
}

/// Detects file types based on local magic byte signatures (Standalone/Deterministic)
fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// File signatures recognised at the start of a transport payload:
/// `(magic, description, MIME type)`.
const ARTIFACT_SIGNATURES: &[(&[u8], &str, &str)] = &[
    (b"%PDF-", "PDF document", "application/pdf"),
    (&[0xFF, 0xD8, 0xFF], "JPEG image", "image/jpeg"),
    (
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        "PNG image",
        "image/png",
    ),
];

/// Detects a file that begins in this packet's transport payload.
///
/// Detection runs against the payload, not the frame. Run against the raw
/// frame the magic bytes could only ever match a destination MAC address
/// (`25:50:44:46:2d:xx` for `%PDF-`), so this reported nothing on real traffic
/// and could invent an artifact out of an unrelated MAC. The name is a type
/// description because no filename is carried anywhere in the packet; size and
/// hash cover the bytes actually present here, which for TCP is usually one
/// fragment of the file rather than the whole thing.
fn detect_artifacts(payload: &[u8]) -> Vec<Artifact> {
    ARTIFACT_SIGNATURES
        .iter()
        .find(|(magic, _, _)| payload.starts_with(magic))
        .map(|(_, name, mime)| Artifact {
            name: (*name).to_string(),
            mime_type: (*mime).to_string(),
            size: payload.len(),
            hash_sha256: compute_sha256(payload),
        })
        .into_iter()
        .collect()
}

/// Combines the packet's observable signals into a 0-100 risk score.
///
/// Evidence-based: an ordinary packet scores 0, so a non-zero score always
/// means the dissector actually found something. (The previous rule —
/// `entropy > 7.5 ? 70 : 10` — gave every packet a non-zero score, which made
/// the number meaningless.)
///
/// - +20 per expert finding (bad checksum, low TTL, zero window, ...), capped
///   at 60.
/// - +30 for a high-entropy payload, which usually means encrypted,
///   compressed, or obfuscated data we cannot inspect.
/// - +20 when the packet begins a file.
fn compute_risk_score(entropy: f64, expert_summary: &[String], artifacts: &[Artifact]) -> u8 {
    let findings = (expert_summary.len() as u32 * 20).min(60) as u8;
    let high_entropy = if entropy > 7.5 { 30 } else { 0 };
    let file_transfer = if artifacts.is_empty() { 0 } else { 20 };
    (findings + high_entropy + file_transfer).min(100)
}

/// Extracts a flow key from a raw packet if it's an IP packet with a transport layer.
pub fn get_flow_key(raw_data: &[u8]) -> Option<FlowKey> {
    let ethernet = EthernetPacket::new(raw_data)?;
    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ipv4 = Ipv4Packet::new(ethernet.payload())?;
            let src_ip = IpAddr::V4(ipv4.get_source());
            let dst_ip = IpAddr::V4(ipv4.get_destination());
            let protocol = ipv4.get_next_level_protocol().0;

            let (src_port, dst_port) = match ipv4.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp = TcpPacket::new(ipv4.payload())?;
                    (tcp.get_source(), tcp.get_destination())
                }
                IpNextHeaderProtocols::Udp => {
                    let udp = UdpPacket::new(ipv4.payload())?;
                    (udp.get_source(), udp.get_destination())
                }
                _ => (0, 0),
            };
            Some(FlowKey::new(src_ip, dst_ip, protocol, src_port, dst_port))
        }
        EtherTypes::Ipv6 => {
            let ipv6 = Ipv6Packet::new(ethernet.payload())?;
            let src_ip = IpAddr::V6(ipv6.get_source());
            let dst_ip = IpAddr::V6(ipv6.get_destination());
            let protocol = ipv6.get_next_header().0;

            let (src_port, dst_port) = match ipv6.get_next_header() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp = TcpPacket::new(ipv6.payload())?;
                    (tcp.get_source(), tcp.get_destination())
                }
                IpNextHeaderProtocols::Udp => {
                    let udp = UdpPacket::new(ipv6.payload())?;
                    (udp.get_source(), udp.get_destination())
                }
                _ => (0, 0),
            };
            Some(FlowKey::new(src_ip, dst_ip, protocol, src_port, dst_port))
        }
        _ => None,
    }
}

// Application layer protocol detection based on ports
/// Identifies an application protocol from a well-known port alone.
fn service_on_port(port: u16, payload: &[u8]) -> Option<(String, String)> {
    match port {
        53 => Some((
            "DNS".to_string(),
            if dns_is_response(payload) {
                "DNS Response"
            } else {
                "DNS Query"
            }
            .to_string(),
        )),
        67 | 68 => Some((
            "DHCP".to_string(),
            if payload.first() == Some(&1) {
                "DHCP Discover"
            } else {
                "DHCP"
            }
            .to_string(),
        )),
        69 => Some(("TFTP".to_string(), "TFTP".to_string())),
        123 => Some(("NTP".to_string(), "NTP Request".to_string())),
        137 | 138 => Some((
            "NetBIOS".to_string(),
            if port == 137 {
                "NetBIOS Name Query"
            } else {
                "NetBIOS Datagram"
            }
            .to_string(),
        )),
        161 | 162 => Some((
            "SNMP".to_string(),
            if port == 161 { "SNMP Get" } else { "SNMP Trap" }.to_string(),
        )),
        389 => Some(("LDAP".to_string(), "LDAP Query".to_string())),
        443 => Some(("HTTPS".to_string(), "TLS/SSL".to_string())),
        445 => Some(("SMB".to_string(), "SMB".to_string())),
        514 => Some(("Syslog".to_string(), "Syslog".to_string())),
        631 => Some(("IPP".to_string(), "Printer (IPP)".to_string())),
        1900 => Some(("SSDP".to_string(), "UPnP Discovery".to_string())),
        5353 => Some(("mDNS".to_string(), "Multicast DNS".to_string())),
        7070 => Some(("SIP".to_string(), "SIP Invite".to_string())),
        8080 => Some(("HTTP".to_string(), "HTTP Proxy".to_string())),
        8443 => Some(("HTTPS".to_string(), "TLS Alt".to_string())),
        9200 => Some(("Elasticsearch".to_string(), "ES Query".to_string())),
        27017 => Some(("MongoDB".to_string(), "MongoDB Query".to_string())),
        _ => None,
    }
}

/// Identifies an application protocol from the payload alone.
fn sniff_payload(payload: &[u8]) -> Option<(String, String)> {
    if payload.len() < 4 {
        return None;
    }
    if payload.starts_with(b"GET ")
        || payload.starts_with(b"POST ")
        || payload.starts_with(b"PUT ")
        || payload.starts_with(b"DELETE ")
        || payload.starts_with(b"HEAD ")
        || payload.starts_with(b"HTTP/")
    {
        Some(("HTTP".to_string(), "HTTP".to_string()))
    } else if payload.starts_with(b"{\"") || payload.starts_with(b"[{\"") {
        Some(("JSON".to_string(), "JSON Data".to_string()))
    } else {
        None
    }
}

/// Identifies the application protocol of a datagram.
///
/// Both ports are consulted — destination first, because that is the service
/// being contacted — instead of the old "destination if below 1024, else
/// source" rule. That rule ignored well-known services at or above 1024 (mDNS
/// 5353, SSDP 1900, 8080, 27017, ...): whenever a client's ephemeral port was
/// the destination the service was never looked up, so an mDNS *query* was
/// labelled plain UDP while the reply in the same conversation was labelled
/// mDNS.
fn detect_app_protocol(src_port: u16, dst_port: u16, payload: &[u8]) -> Option<(String, String)> {
    service_on_port(dst_port, payload)
        .or_else(|| service_on_port(src_port, payload))
        .or_else(|| sniff_payload(payload))
}

/// True when a DNS message is a response (QR bit set).
///
/// Handles the two-byte length prefix DNS uses over TCP.
fn dns_is_response(payload: &[u8]) -> bool {
    let tcp_prefixed = payload.len() >= 4
        && u16::from_be_bytes([payload[0], payload[1]]) as usize == payload.len() - 2;
    let flags_at = if tcp_prefixed { 4 } else { 2 };
    payload
        .get(flags_at..flags_at + 2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]) & 0x8000 != 0)
        .unwrap_or(false)
}

/// True for a TLS record: content type 0x14-0x17 carrying a 0x03xx version.
fn is_tls_record(payload: &[u8]) -> bool {
    payload.len() >= 3
        && matches!(payload[0], 0x14..=0x17)
        && payload[1] == 0x03
        && matches!(payload[2], 0x00..=0x04)
}

/// Classifies a TCP segment.
///
/// Payload evidence wins — a TLS record on port 80 is TLS, an HTTP request on
/// port 443 is HTTP — and a segment that carries no application data (SYN, pure
/// ACK, FIN, window probe) is never claimed to be HTTP or HTTPS. The old rule
/// labelled every segment with 80 or 443 in either port as HTTP/HTTPS, which
/// mislabelled the majority of a connection's packets, all of which are
/// control segments. Payload we cannot identify falls back to the well-known
/// port, the same convention the UDP table above uses.
fn classify_tcp(src_port: u16, dst_port: u16, payload: &[u8]) -> Option<(String, String)> {
    if let Some(app) = sniff_payload(payload) {
        return Some(app);
    }
    if is_tls_record(payload) {
        return Some((PROTO_HTTPS.to_string(), "TLS/SSL".to_string()));
    }
    if payload.is_empty() {
        return None;
    }

    match (src_port, dst_port) {
        (80, _) | (_, 80) | (8080, _) | (_, 8080) => {
            Some((PROTO_HTTP.to_string(), "HTTP".to_string()))
        }
        (443, _) | (_, 443) | (8443, _) | (_, 8443) => {
            Some((PROTO_HTTPS.to_string(), "TLS/SSL".to_string()))
        }
        // DNS runs over TCP too (zone transfers, and increasingly by default).
        (53, _) | (_, 53) => service_on_port(53, payload),
        _ => None,
    }
}

/// Description of an ICMP/ICMPv6 message for the packet list.
fn icmp_description(icmp: &[u8], v6: bool) -> String {
    let type_name = icmp
        .first()
        .map(|&t| {
            if v6 {
                icmpv6_type_name(t)
            } else {
                icmp_type_name(t)
            }
        })
        .unwrap_or(None);

    match type_name {
        Some(name) => name.to_string(),
        None => match icmp.first() {
            Some(&t) if v6 => format!("ICMPv6 type {t}"),
            Some(&t) => format!("ICMP type {t}"),
            None if v6 => "ICMPv6".to_string(),
            None => "ICMP".to_string(),
        },
    }
}

// Lightweight parser for the packet list view
/// Extracts source/destination transport ports for TCP and UDP packets.
///
/// Ports are stored alongside the summary so `port:` filters can match exact
/// port numbers instead of substring-searching the human-readable `info` text
/// (where `port:80` would also match 8080, 1080, and `192.168.1.80`).
pub fn extract_ports(raw_data: &[u8]) -> Option<(u16, u16)> {
    let ethernet = EthernetPacket::new(raw_data)?;

    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ipv4 = Ipv4Packet::new(ethernet.payload())?;
            match ipv4.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp = TcpPacket::new(ipv4.payload())?;
                    Some((tcp.get_source(), tcp.get_destination()))
                }
                IpNextHeaderProtocols::Udp => {
                    let udp = UdpPacket::new(ipv4.payload())?;
                    Some((udp.get_source(), udp.get_destination()))
                }
                _ => None,
            }
        }
        EtherTypes::Ipv6 => {
            let ipv6 = Ipv6Packet::new(ethernet.payload())?;
            match ipv6.get_next_header() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp = TcpPacket::new(ipv6.payload())?;
                    Some((tcp.get_source(), tcp.get_destination()))
                }
                IpNextHeaderProtocols::Udp => {
                    let udp = UdpPacket::new(ipv6.payload())?;
                    Some((udp.get_source(), udp.get_destination()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn parse_summary(raw_data: &[u8], id: u64, timestamp_ns: i64) -> Option<PacketSummary> {
    let ethernet = EthernetPacket::new(raw_data)?;

    let (source_addr, dest_addr, protocol, info) = match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ipv4 = Ipv4Packet::new(ethernet.payload())?;
            let src = ipv4.get_source().to_string();
            let dst = ipv4.get_destination().to_string();

            match ipv4.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => {
                    if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                        let dst_port = tcp.get_destination();
                        let src_port = tcp.get_source();
                        let (proto, info_str) =
                            match classify_tcp(src_port, dst_port, tcp.payload()) {
                                Some(app) => app,
                                None => (
                                    PROTO_TCP.to_string(),
                                    format!("{}:{} → {}:{}", src, src_port, dst, dst_port),
                                ),
                            };
                        (src.clone(), dst.clone(), proto, info_str)
                    } else {
                        (
                            src.clone(),
                            dst.clone(),
                            PROTO_TCP.to_string(),
                            format!("{} → {}", src, dst),
                        )
                    }
                }
                IpNextHeaderProtocols::Udp => {
                    if let Some(udp) = UdpPacket::new(ipv4.payload()) {
                        let dst_port = udp.get_destination();
                        let src_port = udp.get_source();
                        let payload = udp.payload();

                        let (proto, info_str) = if let Some((app_proto, app_info)) =
                            detect_app_protocol(src_port, dst_port, payload)
                        {
                            (app_proto, app_info)
                        } else {
                            (
                                PROTO_UDP.to_string(),
                                format!("{}:{} → {}:{}", src, src_port, dst, dst_port),
                            )
                        };
                        (src.clone(), dst.clone(), proto, info_str)
                    } else {
                        (
                            src.clone(),
                            dst.clone(),
                            PROTO_UDP.to_string(),
                            format!("{} → {}", src, dst),
                        )
                    }
                }
                IpNextHeaderProtocols::Icmp => (
                    src.clone(),
                    dst.clone(),
                    PROTO_ICMP.to_string(),
                    format!(
                        "{} → {} [{}]",
                        src,
                        dst,
                        icmp_description(ipv4.payload(), false)
                    ),
                ),
                _ => (
                    src.clone(),
                    dst.clone(),
                    PROTO_IPV4.to_string(),
                    format!("{} → {}", src, dst),
                ),
            }
        }
        EtherTypes::Ipv6 => {
            let ipv6 = Ipv6Packet::new(ethernet.payload())?;
            let src = ipv6.get_source().to_string();
            let dst = ipv6.get_destination().to_string();

            match ipv6.get_next_header() {
                IpNextHeaderProtocols::Tcp => {
                    if let Some(tcp) = TcpPacket::new(ipv6.payload()) {
                        let dst_port = tcp.get_destination();
                        let src_port = tcp.get_source();
                        let (proto, info_str) =
                            match classify_tcp(src_port, dst_port, tcp.payload()) {
                                Some(app) => app,
                                None => (
                                    PROTO_TCP.to_string(),
                                    format!("{}:{} → {}:{}", src, src_port, dst, dst_port),
                                ),
                            };
                        (src.clone(), dst.clone(), proto, info_str)
                    } else {
                        (
                            src.clone(),
                            dst.clone(),
                            PROTO_TCP.to_string(),
                            format!("{} → {}", src, dst),
                        )
                    }
                }
                IpNextHeaderProtocols::Udp => {
                    if let Some(udp) = UdpPacket::new(ipv6.payload()) {
                        let dst_port = udp.get_destination();
                        let src_port = udp.get_source();
                        let payload = udp.payload();

                        let (proto, info_str) = if let Some((app_proto, app_info)) =
                            detect_app_protocol(src_port, dst_port, payload)
                        {
                            (app_proto, app_info)
                        } else {
                            (
                                PROTO_UDP.to_string(),
                                format!("{}:{} → {}:{}", src, src_port, dst, dst_port),
                            )
                        };
                        (src.clone(), dst.clone(), proto, info_str)
                    } else {
                        (
                            src.clone(),
                            dst.clone(),
                            PROTO_UDP.to_string(),
                            format!("{} → {}", src, dst),
                        )
                    }
                }
                IpNextHeaderProtocols::Icmpv6 => (
                    src.clone(),
                    dst.clone(),
                    PROTO_ICMPV6.to_string(),
                    format!(
                        "{} → {} [{}]",
                        src,
                        dst,
                        icmp_description(ipv6.payload(), true)
                    ),
                ),
                _ => (
                    src.clone(),
                    dst.clone(),
                    PROTO_IPV6.to_string(),
                    format!("{} → {}", src, dst),
                ),
            }
        }
        EtherTypes::Arp => {
            let src = ethernet.get_source().to_string();
            let dst = ethernet.get_destination().to_string();
            (src, dst, PROTO_ARP.to_string(), "ARP Request".to_string())
        }
        _ => {
            let src = ethernet.get_source().to_string();
            let dst = ethernet.get_destination().to_string();
            (
                src,
                dst,
                PROTO_UNKNOWN.to_string(),
                PROTO_UNKNOWN.to_string(),
            )
        }
    };

    let (src_port, dst_port) = extract_ports(raw_data).unzip();

    Some(PacketSummary {
        id,
        timestamp: timestamp_ns,
        source_addr,
        dest_addr,
        protocol,
        length: raw_data.len() as u32,
        info,
        src_port,
        dst_port,
    })
}

/// Extracts the transport layer payload from a raw packet.
pub fn get_transport_payload(raw_data: &[u8]) -> Option<Vec<u8>> {
    let ethernet = EthernetPacket::new(raw_data)?;
    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ipv4 = Ipv4Packet::new(ethernet.payload())?;
            match ipv4.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp = TcpPacket::new(ipv4.payload())?;
                    Some(tcp.payload().to_vec())
                }
                IpNextHeaderProtocols::Udp => {
                    let udp = UdpPacket::new(ipv4.payload())?;
                    Some(udp.payload().to_vec())
                }
                _ => None,
            }
        }
        EtherTypes::Ipv6 => {
            let ipv6 = Ipv6Packet::new(ethernet.payload())?;
            match ipv6.get_next_header() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp = TcpPacket::new(ipv6.payload())?;
                    Some(tcp.payload().to_vec())
                }
                IpNextHeaderProtocols::Udp => {
                    let udp = UdpPacket::new(ipv6.payload())?;
                    Some(udp.payload().to_vec())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// One TCP segment's contribution to one direction of a stream.
#[derive(Debug, Clone)]
pub struct TcpSegment {
    /// Sequence number of the segment's first payload byte.
    pub seq: u32,
    /// Capture time of the segment, in nanoseconds since the epoch.
    pub timestamp_ns: i64,
    /// Payload bytes carried by the segment. Never empty.
    pub payload: Vec<u8>,
}

/// A contiguous run of stream bytes produced by [`reassemble_tcp`].
#[derive(Debug, Clone)]
pub struct StreamBlock {
    pub data: Vec<u8>,
    /// Stream bytes that were never captured immediately before this block.
    /// 0 when the stream is contiguous up to this point.
    pub missing_before: u64,
    /// Capture time of the first packet that contributed to the block.
    pub timestamp_ns: i64,
}

/// Extracts the sequence number and payload of a TCP frame.
///
/// Returns `None` for non-TCP frames and for segments that carry no payload
/// (SYN, FIN and pure ACKs): those consume sequence numbers but contribute no
/// bytes to the stream.
pub fn get_tcp_segment(raw_data: &[u8], timestamp_ns: i64) -> Option<TcpSegment> {
    let ethernet = EthernetPacket::new(raw_data)?;
    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ipv4 = Ipv4Packet::new(ethernet.payload())?;
            if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Tcp {
                return None;
            }
            segment_from_tcp(ipv4.payload(), timestamp_ns)
        }
        EtherTypes::Ipv6 => {
            let ipv6 = Ipv6Packet::new(ethernet.payload())?;
            if ipv6.get_next_header() != IpNextHeaderProtocols::Tcp {
                return None;
            }
            segment_from_tcp(ipv6.payload(), timestamp_ns)
        }
        _ => None,
    }
}

/// Reads the sequence number and payload out of a TCP header plus its data.
fn segment_from_tcp(tcp_bytes: &[u8], timestamp_ns: i64) -> Option<TcpSegment> {
    let tcp = TcpPacket::new(tcp_bytes)?;
    let payload = tcp.payload();
    if payload.is_empty() {
        return None;
    }

    Some(TcpSegment {
        seq: tcp.get_sequence(),
        timestamp_ns,
        payload: payload.to_vec(),
    })
}

/// Reassembles one direction of a TCP stream from captured segments.
///
/// Segments are placed by **sequence number**, never by arrival order, so a
/// segment that was captured out of order still lands where it belongs, and a
/// retransmission (or a partial overlap) never contributes a byte twice — the
/// first segment to claim a byte keeps it. Bytes that were never captured, be
/// it a segment lost before it reached the interface or a hole in our own
/// capture, split the stream into separate blocks and are reported as
/// `missing_before`: gluing the halves together would present bytes that never
/// existed on the wire as if they had.
///
/// TCP sequence numbers wrap at 2^32, so positions are wrapping distances from
/// the lowest sequence seen — exact for any stream below 2 GiB.
pub fn reassemble_tcp(mut segments: Vec<TcpSegment>) -> Vec<StreamBlock> {
    if segments.is_empty() {
        return Vec::new();
    }

    // Anchor on the lowest sequence number (wrapping comparison), so every
    // offset is a non-negative distance from the earliest byte we captured.
    let mut base = segments[0].seq;
    for segment in &segments {
        if (segment.seq.wrapping_sub(base) as i32) < 0 {
            base = segment.seq;
        }
    }

    // Ascending by stream position. `sort_by_key` is stable, so segments that
    // claim the same byte keep capture order and the first one wins.
    segments.sort_by_key(|s| s.seq.wrapping_sub(base) as i32);

    let mut blocks: Vec<StreamBlock> = Vec::new();
    let mut data: Vec<u8> = Vec::new();
    let mut block_started_at = segments[0].timestamp_ns;
    let mut missing_before = 0u64;
    // First stream offset that is not in `data` yet (absolute, not relative to
    // the current block: it only restarts at 0 implicitly when `data` does).
    let mut next = 0u64;

    for segment in &segments {
        let offset = segment.seq.wrapping_sub(base) as i32 as u64;
        let end = offset + segment.payload.len() as u64;

        if end <= next {
            continue; // retransmission: every byte is already stored
        }

        if offset > next {
            // A hole: close the block and start counting what is missing.
            if !data.is_empty() {
                blocks.push(StreamBlock {
                    data: std::mem::take(&mut data),
                    missing_before,
                    timestamp_ns: block_started_at,
                });
            }
            missing_before = offset - next;
            block_started_at = segment.timestamp_ns;
        }

        // Skip bytes an earlier segment already claimed (overlap).
        let skip = next.saturating_sub(offset) as usize;
        data.extend_from_slice(&segment.payload[skip..]);
        next = end;
    }

    if !data.is_empty() {
        blocks.push(StreamBlock {
            data,
            missing_before,
            timestamp_ns: block_started_at,
        });
    }

    blocks
}

fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }
    let mut entropy = 0.0;
    let len = data.len() as f64;
    for &count in &counts {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// RFC 1071 one's-complement checksum over `header`.
fn internet_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let (chunks, remainder) = header.as_chunks::<2>();
    for chunk in chunks {
        sum += u16::from_be_bytes(*chunk) as u32;
    }
    if !remainder.is_empty() {
        sum += (remainder[0] as u32) << 8;
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !sum as u16
}

/// Verifies an RFC 1071 checksum stored at `checksum_offset`.
///
/// Returns `(stored, recomputed)`; `None` when the data is too short to hold
/// the checksum field.
fn verify_checksum(data: &[u8], checksum_offset: usize) -> Option<(u16, u16)> {
    if data.len() < checksum_offset + 2 {
        return None;
    }
    let stored = u16::from_be_bytes([data[checksum_offset], data[checksum_offset + 1]]);
    let mut zeroed = data.to_vec();
    zeroed[checksum_offset] = 0;
    zeroed[checksum_offset + 1] = 0;
    Some((stored, internet_checksum(&zeroed)))
}

/// Verifies the IPv4 header checksum in place (RFC 1071).
///
/// Returns `(stored, recomputed)` where `stored` is the value the sender put
/// in bytes 10-11 and `recomputed` is what the header actually sums to.
/// `None` when the header is truncated and cannot be checked.
fn verify_ipv4_checksum(header: &[u8]) -> Option<(u16, u16)> {
    if header.len() < 20 {
        return None;
    }
    verify_checksum(header, 10)
}

/// ICMP (RFC 792) type names.
fn icmp_type_name(icmp_type: u8) -> Option<&'static str> {
    Some(match icmp_type {
        0 => "Echo Reply",
        3 => "Destination Unreachable",
        4 => "Source Quench",
        5 => "Redirect",
        8 => "Echo Request",
        9 => "Router Advertisement",
        10 => "Router Solicitation",
        11 => "Time Exceeded",
        12 => "Parameter Problem",
        13 => "Timestamp Request",
        14 => "Timestamp Reply",
        _ => return None,
    })
}

/// ICMPv6 (RFC 4443, RFC 4861) type names.
fn icmpv6_type_name(icmp_type: u8) -> Option<&'static str> {
    Some(match icmp_type {
        1 => "Destination Unreachable",
        2 => "Packet Too Big",
        3 => "Time Exceeded",
        4 => "Parameter Problem",
        128 => "Echo Request",
        129 => "Echo Reply",
        130 => "Multicast Listener Query",
        131 => "Multicast Listener Report",
        132 => "Multicast Listener Done",
        133 => "Router Solicitation",
        134 => "Router Advertisement",
        135 => "Neighbor Solicitation",
        136 => "Neighbor Advertisement",
        137 => "Redirect",
        _ => return None,
    })
}

/// Builds the ICMP/ICMPv6 protocol layer (and its expert notes).
///
/// The ICMP checksum covers the whole message and is verified here. The ICMPv6
/// checksum additionally covers the IPv6 pseudo-header, so it is reported but
/// not verified.
fn parse_icmp_layer(
    layers: &mut Vec<ProtocolLayer>,
    expert_summary: &mut Vec<String>,
    offset: usize,
    icmp: &[u8],
    v6: bool,
) {
    if icmp.len() < 4 {
        return;
    }

    let icmp_type = icmp[0];
    let code = icmp[1];
    let type_label = match if v6 {
        icmpv6_type_name(icmp_type)
    } else {
        icmp_type_name(icmp_type)
    } {
        Some(name) => format!("{icmp_type} ({name})"),
        None => icmp_type.to_string(),
    };

    let stored = u16::from_be_bytes([icmp[2], icmp[3]]);
    let (checksum_value, checksum_expert) = if v6 {
        (format!("0x{stored:04x} (not verified)"), None)
    } else {
        match verify_checksum(icmp, 2) {
            None => (format!("0x{stored:04x}"), None),
            Some((stored, computed)) => {
                let correct = stored == computed;
                (
                    format!(
                        "0x{stored:04x} ({})",
                        if correct { "correct" } else { "incorrect" }
                    ),
                    if correct {
                        None
                    } else {
                        Some("Checksum incorrect".to_string())
                    },
                )
            }
        }
    };

    if checksum_expert.is_some() {
        expert_summary.push(
            "ICMP checksum incorrect. The message was modified in transit, or captured before the sender finished computing it."
                .to_string(),
        );
    }

    let mut fields = vec![
        PacketField {
            name: "Type".to_string(),
            value: type_label,
            range: (offset, offset + 1),
            expert: None,
        },
        PacketField {
            name: "Code".to_string(),
            value: code.to_string(),
            range: (offset + 1, offset + 2),
            expert: None,
        },
        PacketField {
            name: "Checksum".to_string(),
            value: checksum_value,
            range: (offset + 2, offset + 4),
            expert: checksum_expert,
        },
    ];

    // Echo request/reply carry an identifier and sequence number.
    let is_echo = if v6 {
        matches!(icmp_type, 128 | 129)
    } else {
        matches!(icmp_type, 0 | 8)
    };
    if is_echo && icmp.len() >= 8 {
        fields.push(PacketField {
            name: "Identifier".to_string(),
            value: u16::from_be_bytes([icmp[4], icmp[5]]).to_string(),
            range: (offset + 4, offset + 6),
            expert: None,
        });
        fields.push(PacketField {
            name: "Sequence Number".to_string(),
            value: u16::from_be_bytes([icmp[6], icmp[7]]).to_string(),
            range: (offset + 6, offset + 8),
            expert: None,
        });
    }

    layers.push(ProtocolLayer {
        name: if v6 {
            "Internet Control Message Protocol v6".to_string()
        } else {
            "Internet Control Message Protocol".to_string()
        },
        fields,
    });
}

fn generate_narrative(summary: &PacketSummary, layers: &[ProtocolLayer]) -> ForensicNarrative {
    let mut details = Vec::new();
    let mut narrative_summary = format!(
        "This is a {} packet from {} to {}.",
        summary.protocol, summary.source_addr, summary.dest_addr
    );

    for layer in layers {
        match layer.name.as_str() {
            "Ethernet" => {
                details.push("Frame delivered via Ethernet physical layer.".to_string());
            }
            "Internet Protocol Version 4" => {
                narrative_summary = format!(
                    "IPv4 communication identified between {} and {}.",
                    summary.source_addr, summary.dest_addr
                );
                details.push("Standard IPv4 routing used for this exchange.".to_string());
            }
            "Transmission Control Protocol" => {
                let flags = layer
                    .fields
                    .iter()
                    .find(|f| f.name == "Flags")
                    .map(|f| f.value.as_str())
                    .unwrap_or("");
                if flags.contains("0x02") {
                    // SYN
                    narrative_summary = format!(
                        "Connection attempt initiated by {} to {}.",
                        summary.source_addr, summary.dest_addr
                    );
                    details.push(
                        "The source host is requesting to open a new TCP session.".to_string(),
                    );
                } else if flags.contains("0x12") {
                    // SYN-ACK
                    narrative_summary = format!(
                        "Connection request acknowledged by {} to {}.",
                        summary.source_addr, summary.dest_addr
                    );
                    details.push("The destination host has accepted the connection request and is ready to establish a session.".to_string());
                }
            }
            _ => {}
        }
    }

    ForensicNarrative {
        summary: narrative_summary,
        technical_details: details,
    }
}

// Full packet dissection for detail view
pub fn dissect_packet(raw_data: &[u8], id: u64, timestamp_ns: i64) -> Option<PacketDetail> {
    let mut layers = Vec::new();
    let mut expert_summary = Vec::new();

    // Parse Ethernet layer (L2)
    let ethernet = EthernetPacket::new(raw_data)?;
    let src_mac = ethernet.get_source().to_string();
    let manufacturer = get_manufacturer(&src_mac);

    let ethernet_fields = vec![
        PacketField {
            name: "Destination".to_string(),
            value: ethernet.get_destination().to_string(),
            range: (0, 6),
            expert: None,
        },
        PacketField {
            name: "Source".to_string(),
            value: src_mac.clone(),
            range: (6, 12),
            expert: manufacturer
                .clone()
                .map(|m| format!("Hardware detected as {}", m)),
        },
        PacketField {
            name: "Type".to_string(),
            value: format!("0x{:04x}", ethernet.get_ethertype().0),
            range: (12, 14),
            expert: None,
        },
    ];
    layers.push(ProtocolLayer {
        name: "Ethernet".to_string(),
        fields: ethernet_fields,
    });

    let current_offset = 14;

    // Parse IP layer (L3)
    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            if let Some(ipv4) = Ipv4Packet::new(ethernet.payload()) {
                let header_len = (ipv4.get_header_length() as usize) * 4;
                let ttl = ipv4.get_ttl();

                // Recompute the header checksum so the overview panel has a real
                // value to show instead of an empty tile.
                let ip_checksum = ipv4
                    .packet()
                    .get(..header_len)
                    .and_then(verify_ipv4_checksum);
                if let Some((stored, computed)) = ip_checksum {
                    if stored != computed {
                        expert_summary.push(
                            "IPv4 header checksum invalid. Checksum offload on the capturing host, or modification in transit, can cause this."
                                .to_string(),
                        );
                    }
                }
                let (checksum_value, checksum_expert) = match ip_checksum {
                    Some((stored, computed)) => {
                        let correct = stored == computed;
                        (
                            format!(
                                "0x{:04x} ({})",
                                stored,
                                if correct { "correct" } else { "incorrect" }
                            ),
                            if correct {
                                None
                            } else {
                                Some("Header checksum incorrect".to_string())
                            },
                        )
                    }
                    None => ("not verifiable (truncated header)".to_string(), None),
                };
                if ttl < 10 {
                    expert_summary.push("Suspiciously low TTL (Time To Live). Possible traceroute or network manipulation.".to_string());
                }

                let ip_fields = vec![
                    PacketField {
                        name: "Version".to_string(),
                        value: "4".to_string(),
                        // Version (high nibble) and IHL (low nibble) share byte 0.
                        range: (current_offset, current_offset + 1),
                        expert: None,
                    },
                    PacketField {
                        name: "Header Length".to_string(),
                        value: format!("{} bytes", header_len),
                        range: (current_offset, current_offset + 1),
                        expert: None,
                    },
                    PacketField {
                        name: "Total Length".to_string(),
                        value: format!("{} bytes", ipv4.get_total_length()),
                        range: (current_offset + 2, current_offset + 4),
                        expert: None,
                    },
                    PacketField {
                        name: "Identification".to_string(),
                        value: format!("0x{:04x}", ipv4.get_identification()),
                        range: (current_offset + 4, current_offset + 6),
                        expert: None,
                    },
                    PacketField {
                        name: "TTL".to_string(),
                        value: ttl.to_string(),
                        range: (current_offset + 8, current_offset + 9),
                        expert: if ttl < 10 {
                            Some("Very Low TTL".to_string())
                        } else {
                            None
                        },
                    },
                    PacketField {
                        name: "Protocol".to_string(),
                        value: format!(
                            "{} ({})",
                            ipv4.get_next_level_protocol().0,
                            ipv4.get_next_level_protocol()
                        ),
                        range: (current_offset + 9, current_offset + 10),
                        expert: None,
                    },
                    PacketField {
                        name: "Header Checksum".to_string(),
                        value: checksum_value,
                        range: (current_offset + 10, current_offset + 12),
                        expert: checksum_expert,
                    },
                    PacketField {
                        name: "Source".to_string(),
                        value: ipv4.get_source().to_string(),
                        range: (current_offset + 12, current_offset + 16),
                        expert: None,
                    },
                    PacketField {
                        name: "Destination".to_string(),
                        value: ipv4.get_destination().to_string(),
                        range: (current_offset + 16, current_offset + 20),
                        expert: None,
                    },
                ];
                layers.push(ProtocolLayer {
                    name: "Internet Protocol Version 4".to_string(),
                    fields: ip_fields,
                });

                let transport_offset = current_offset + header_len;
                match ipv4.get_next_level_protocol() {
                    IpNextHeaderProtocols::Tcp => {
                        if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                            let tcp_header_len = (tcp.get_data_offset() as usize) * 4;
                            let window = tcp.get_window();
                            if window == 0 {
                                expert_summary.push(
                                    "TCP Zero Window: Connection flow may be stalled.".to_string(),
                                );
                            }

                            let tcp_fields = vec![
                                PacketField {
                                    name: "Source Port".to_string(),
                                    value: tcp.get_source().to_string(),
                                    range: (transport_offset, transport_offset + 2),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Destination Port".to_string(),
                                    value: tcp.get_destination().to_string(),
                                    range: (transport_offset + 2, transport_offset + 4),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Sequence Number".to_string(),
                                    value: tcp.get_sequence().to_string(),
                                    range: (transport_offset + 4, transport_offset + 8),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Acknowledgment Number".to_string(),
                                    value: tcp.get_acknowledgement().to_string(),
                                    range: (transport_offset + 8, transport_offset + 12),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Flags".to_string(),
                                    value: format!("0x{:02x}", tcp.get_flags()),
                                    range: (transport_offset + 13, transport_offset + 14),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Window Size".to_string(),
                                    value: window.to_string(),
                                    range: (transport_offset + 14, transport_offset + 16),
                                    expert: if window == 0 {
                                        Some("TCP Window Zero")
                                    } else {
                                        None
                                    }
                                    .map(|s| s.to_string()),
                                },
                            ];
                            layers.push(ProtocolLayer {
                                name: "Transmission Control Protocol".to_string(),
                                fields: tcp_fields,
                            });

                            let app_offset = transport_offset + tcp_header_len;
                            if !tcp.payload().is_empty() {
                                parse_application_layer(
                                    &mut layers,
                                    tcp.get_source(),
                                    tcp.get_destination(),
                                    tcp.payload(),
                                    true,
                                    app_offset,
                                );
                            }
                        }
                    }
                    IpNextHeaderProtocols::Udp => {
                        if let Some(udp) = UdpPacket::new(ipv4.payload()) {
                            let udp_fields = vec![
                                PacketField {
                                    name: "Source Port".to_string(),
                                    value: udp.get_source().to_string(),
                                    range: (transport_offset, transport_offset + 2),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Destination Port".to_string(),
                                    value: udp.get_destination().to_string(),
                                    range: (transport_offset + 2, transport_offset + 4),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Length".to_string(),
                                    value: format!("{} bytes", udp.get_length()),
                                    range: (transport_offset + 4, transport_offset + 6),
                                    expert: None,
                                },
                            ];
                            layers.push(ProtocolLayer {
                                name: "User Datagram Protocol".to_string(),
                                fields: udp_fields,
                            });

                            let app_offset = transport_offset + 8;
                            if !udp.payload().is_empty() {
                                parse_application_layer(
                                    &mut layers,
                                    udp.get_source(),
                                    udp.get_destination(),
                                    udp.payload(),
                                    false,
                                    app_offset,
                                );
                            }
                        }
                    }
                    IpNextHeaderProtocols::Icmp => {
                        parse_icmp_layer(
                            &mut layers,
                            &mut expert_summary,
                            transport_offset,
                            ipv4.payload(),
                            false,
                        );
                    }
                    _ => {}
                }
            }
        }
        EtherTypes::Ipv6 => {
            if let Some(ipv6) = Ipv6Packet::new(ethernet.payload()) {
                let payload_length = ipv6.get_payload_length();
                let hop_limit = ipv6.get_hop_limit();
                if hop_limit < 10 {
                    expert_summary.push(
                        "Suspiciously low Hop Limit. Possible traceroute or network manipulation."
                            .to_string(),
                    );
                }

                let ip_fields = vec![
                    PacketField {
                        name: "Version".to_string(),
                        value: "6".to_string(),
                        range: (current_offset, current_offset + 1),
                        expert: None,
                    },
                    PacketField {
                        name: "Payload Length".to_string(),
                        value: format!("{} bytes", payload_length),
                        range: (current_offset + 4, current_offset + 6),
                        expert: None,
                    },
                    PacketField {
                        name: "Hop Limit".to_string(),
                        value: hop_limit.to_string(),
                        range: (current_offset + 7, current_offset + 8),
                        expert: if hop_limit < 10 {
                            Some("Very Low Hop Limit".to_string())
                        } else {
                            None
                        },
                    },
                    PacketField {
                        name: "Source".to_string(),
                        value: ipv6.get_source().to_string(),
                        range: (current_offset + 8, current_offset + 24),
                        expert: None,
                    },
                    PacketField {
                        name: "Destination".to_string(),
                        value: ipv6.get_destination().to_string(),
                        range: (current_offset + 24, current_offset + 40),
                        expert: None,
                    },
                ];
                layers.push(ProtocolLayer {
                    name: "Internet Protocol Version 6".to_string(),
                    fields: ip_fields,
                });

                let transport_offset = current_offset + 40; // IPv6 header is always 40 bytes
                match ipv6.get_next_header() {
                    IpNextHeaderProtocols::Tcp => {
                        if let Some(tcp) = TcpPacket::new(ipv6.payload()) {
                            let tcp_header_len = (tcp.get_data_offset() as usize) * 4;
                            let window = tcp.get_window();
                            if window == 0 {
                                expert_summary.push(
                                    "TCP Zero Window: Connection flow may be stalled.".to_string(),
                                );
                            }

                            let tcp_fields = vec![
                                PacketField {
                                    name: "Source Port".to_string(),
                                    value: tcp.get_source().to_string(),
                                    range: (transport_offset, transport_offset + 2),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Destination Port".to_string(),
                                    value: tcp.get_destination().to_string(),
                                    range: (transport_offset + 2, transport_offset + 4),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Sequence Number".to_string(),
                                    value: tcp.get_sequence().to_string(),
                                    range: (transport_offset + 4, transport_offset + 8),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Acknowledgment Number".to_string(),
                                    value: tcp.get_acknowledgement().to_string(),
                                    range: (transport_offset + 8, transport_offset + 12),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Flags".to_string(),
                                    value: format!("0x{:02x}", tcp.get_flags()),
                                    range: (transport_offset + 13, transport_offset + 14),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Window Size".to_string(),
                                    value: window.to_string(),
                                    range: (transport_offset + 14, transport_offset + 16),
                                    expert: if window == 0 {
                                        Some("TCP Window Zero".to_string())
                                    } else {
                                        None
                                    },
                                },
                            ];
                            layers.push(ProtocolLayer {
                                name: "Transmission Control Protocol".to_string(),
                                fields: tcp_fields,
                            });

                            let app_offset = transport_offset + tcp_header_len;
                            if !tcp.payload().is_empty() {
                                parse_application_layer(
                                    &mut layers,
                                    tcp.get_source(),
                                    tcp.get_destination(),
                                    tcp.payload(),
                                    true,
                                    app_offset,
                                );
                            }
                        }
                    }
                    IpNextHeaderProtocols::Udp => {
                        if let Some(udp) = UdpPacket::new(ipv6.payload()) {
                            let udp_fields = vec![
                                PacketField {
                                    name: "Source Port".to_string(),
                                    value: udp.get_source().to_string(),
                                    range: (transport_offset, transport_offset + 2),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Destination Port".to_string(),
                                    value: udp.get_destination().to_string(),
                                    range: (transport_offset + 2, transport_offset + 4),
                                    expert: None,
                                },
                                PacketField {
                                    name: "Length".to_string(),
                                    value: format!("{} bytes", udp.get_length()),
                                    range: (transport_offset + 4, transport_offset + 6),
                                    expert: None,
                                },
                            ];
                            layers.push(ProtocolLayer {
                                name: "User Datagram Protocol".to_string(),
                                fields: udp_fields,
                            });

                            let app_offset = transport_offset + 8;
                            if !udp.payload().is_empty() {
                                parse_application_layer(
                                    &mut layers,
                                    udp.get_source(),
                                    udp.get_destination(),
                                    udp.payload(),
                                    false,
                                    app_offset,
                                );
                            }
                        }
                    }
                    IpNextHeaderProtocols::Icmpv6 => {
                        parse_icmp_layer(
                            &mut layers,
                            &mut expert_summary,
                            transport_offset,
                            ipv6.payload(),
                            true,
                        );
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    // Get summary
    let summary = parse_summary(raw_data, id, timestamp_ns)?;

    // The payload — not the frame — is what entropy, artifact detection and
    // the risk score should describe: IP/TCP headers dilute the entropy, and
    // file signatures in the frame could only ever match a MAC address.
    let payload = get_transport_payload(raw_data).unwrap_or_default();
    let entropy = calculate_entropy(if payload.is_empty() {
        raw_data
    } else {
        &payload
    });
    let narrative = generate_narrative(&summary, &layers);
    let artifacts = detect_artifacts(&payload);
    let risk_score = compute_risk_score(entropy, &expert_summary, &artifacts);

    Some(PacketDetail {
        summary,
        layers,
        raw_bytes: raw_data.to_vec(),
        expert_summary,
        narrative,
        intelligence: ForensicIntelligence {
            entropy,
            manufacturer,
            risk_score,
        },
        artifacts,
    })
}

// Application layer parsing

/// HTTP request methods recognised in a request line.
const HTTP_METHODS: &[&str] = &[
    "GET ", "POST ", "PUT ", "DELETE ", "HEAD ", "PATCH ", "OPTIONS ", "TRACE ", "CONNECT ",
];

fn field(name: &str, value: impl Into<String>, range: (usize, usize)) -> PacketField {
    PacketField {
        name: name.to_string(),
        value: value.into(),
        range,
        expert: None,
    }
}

/// Value of a header in an HTTP head, searching past the request/status line
/// and stopping at the blank line that ends the header block.
fn http_header_value(head: &str, wanted: &str) -> Option<String> {
    let mut lines = head.lines();
    lines.next();
    for line in lines {
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case(wanted) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Parses an HTTP request or response head into a layer.
///
/// HTTP is recognised from the payload rather than the port: the old port-80
/// check gave every UTF-8 payload on port 80 an HTTP layer with a single
/// "Data" field, while HTTP on a proxy port got none. Returns false when the
/// payload is not HTTP, so no layer is fabricated.
fn parse_http_layer(layers: &mut Vec<ProtocolLayer>, payload: &[u8], offset: usize) -> bool {
    // Only the head matters; the body can be arbitrarily large.
    let head = String::from_utf8_lossy(&payload[..payload.len().min(1024)]);
    let end = offset + payload.len();
    let mut fields = Vec::new();

    if HTTP_METHODS.iter().any(|method| head.starts_with(method)) {
        // Request line: METHOD SP request-target SP HTTP-version
        let mut parts = head.splitn(3, ' ');
        let method = parts.next().unwrap_or_default().trim().to_string();
        let uri = parts.next().unwrap_or_default().trim().to_string();
        let version = parts
            .next()
            .and_then(|rest| rest.lines().next())
            .unwrap_or_default()
            .trim()
            .to_string();

        let method_end = offset + method.len();
        fields.push(field("Method", method, (offset, method_end)));
        if !uri.is_empty() {
            let uri_end = (method_end + 1 + uri.len()).min(end);
            fields.push(field("Request URI", uri, (method_end + 1, uri_end)));
        }
        if !version.is_empty() {
            fields.push(field("Version", version, (offset, end)));
        }
        if let Some(host) = http_header_value(&head, "Host") {
            fields.push(field("Host", host, (offset, end)));
        }
    } else if head.starts_with("HTTP/") {
        // Status line: HTTP-version SP status-code SP reason-phrase
        let mut parts = head.splitn(3, ' ');
        let version = parts.next().unwrap_or_default().trim().to_string();
        let code = parts.next().unwrap_or_default().trim().to_string();
        let reason = parts
            .next()
            .and_then(|rest| rest.lines().next())
            .unwrap_or_default()
            .trim()
            .to_string();

        fields.push(field("Version", version, (offset, end)));
        if !code.is_empty() {
            fields.push(field("Status Code", code, (offset, end)));
        }
        if !reason.is_empty() {
            fields.push(field("Reason Phrase", reason, (offset, end)));
        }
        if let Some(content_type) = http_header_value(&head, "Content-Type") {
            fields.push(field("Content-Type", content_type, (offset, end)));
        }
    } else {
        return false;
    }

    fields.push(field(
        "Payload Length",
        format!("{} bytes", payload.len()),
        (offset, end),
    ));
    layers.push(ProtocolLayer {
        name: "Hypertext Transfer Protocol".to_string(),
        fields,
    });
    true
}

/// Reads a DNS name (RFC 1035 §3.1), following compression pointers.
///
/// Returns the name and the offset just past it in the original message.
fn read_dns_name(buf: &[u8], mut pos: usize) -> Option<(String, usize)> {
    let mut labels: Vec<String> = Vec::new();
    let mut next = pos;
    let mut hops = 0u8;

    loop {
        let len = *buf.get(pos)? as usize;
        if len == 0 {
            if hops == 0 {
                next = pos + 1;
            }
            break;
        }
        if len & 0xC0 == 0xC0 {
            // Compression pointer: the top two bits are set.
            if hops == 0 {
                next = pos + 2;
            }
            let low = *buf.get(pos + 1)? as usize;
            pos = ((len & 0x3F) << 8) | low;
            hops += 1;
            if hops > 16 {
                return None; // malformed message must not loop forever
            }
            continue;
        }
        if len > 63 {
            return None;
        }
        let label = buf.get(pos + 1..pos + 1 + len)?;
        labels.push(String::from_utf8_lossy(label).into_owned());
        pos += 1 + len;
        if hops == 0 {
            next = pos;
        }
    }

    if labels.is_empty() {
        Some((".".to_string(), next))
    } else {
        Some((labels.join("."), next))
    }
}

fn dns_type_name(code: u16) -> String {
    match code {
        1 => "A".to_string(),
        2 => "NS".to_string(),
        5 => "CNAME".to_string(),
        6 => "SOA".to_string(),
        12 => "PTR".to_string(),
        15 => "MX".to_string(),
        16 => "TXT".to_string(),
        28 => "AAAA".to_string(),
        33 => "SRV".to_string(),
        41 => "OPT".to_string(),
        65 => "HTTPS".to_string(),
        other => format!("TYPE{other}"),
    }
}

fn dns_class_name(code: u16) -> String {
    match code {
        1 => "IN".to_string(),
        3 => "CH".to_string(),
        4 => "HS".to_string(),
        other => format!("CLASS{other}"),
    }
}

/// Parses a DNS message into a layer: header fields plus the first question.
///
/// Over TCP a DNS message is prefixed with a two-byte length; that prefix is
/// skipped when it matches the bytes actually present.
fn parse_dns_layer(layers: &mut Vec<ProtocolLayer>, payload: &[u8], is_tcp: bool, offset: usize) {
    let prefix = if is_tcp
        && payload.len() >= 4
        && u16::from_be_bytes([payload[0], payload[1]]) as usize == payload.len() - 2
    {
        2
    } else {
        0
    };
    let msg = &payload[prefix..];
    let base = offset + prefix;
    let end = offset + payload.len();

    // Short capture: show what we have rather than nothing.
    if msg.len() < 12 {
        layers.push(ProtocolLayer {
            name: "Domain Name System".to_string(),
            fields: vec![field(
                "Payload Length",
                format!("{} bytes", payload.len()),
                (offset, end),
            )],
        });
        return;
    }

    let id = u16::from_be_bytes([msg[0], msg[1]]);
    let flags = u16::from_be_bytes([msg[2], msg[3]]);
    let questions = u16::from_be_bytes([msg[4], msg[5]]);
    let answers = u16::from_be_bytes([msg[6], msg[7]]);
    let authority = u16::from_be_bytes([msg[8], msg[9]]);
    let additional = u16::from_be_bytes([msg[10], msg[11]]);

    let is_response = flags & 0x8000 != 0;
    let opcode = (flags >> 11) & 0x0F;
    let rcode = flags & 0x0F;

    let mut fields = vec![
        field("Transaction ID", format!("0x{id:04x}"), (base, base + 2)),
        field(
            "Message Type",
            if is_response { "Response" } else { "Query" },
            (base + 2, base + 4),
        ),
        field(
            "Opcode",
            match opcode {
                0 => "Query".to_string(),
                1 => "Inverse Query".to_string(),
                2 => "Status".to_string(),
                other => format!("Opcode {other}"),
            },
            (base + 2, base + 4),
        ),
        field("Questions", questions.to_string(), (base + 4, base + 6)),
    ];
    if is_response {
        fields.push(field(
            "Response Code",
            match rcode {
                0 => "No Error".to_string(),
                1 => "Format Error".to_string(),
                2 => "Server Failure".to_string(),
                3 => "Name Error (NXDOMAIN)".to_string(),
                4 => "Not Implemented".to_string(),
                5 => "Refused".to_string(),
                other => format!("RCODE {other}"),
            },
            (base + 2, base + 4),
        ));
    }
    for (label, count) in [
        ("Answers", answers),
        ("Authority", authority),
        ("Additional", additional),
    ] {
        if count > 0 {
            fields.push(field(label, count.to_string(), (base + 6, base + 8)));
        }
    }

    if questions > 0 {
        if let Some((name, after_name)) = read_dns_name(msg, 12) {
            let qtype = msg
                .get(after_name..after_name + 2)
                .map(|b| u16::from_be_bytes([b[0], b[1]]));
            let qclass = msg
                .get(after_name + 2..after_name + 4)
                .map(|b| u16::from_be_bytes([b[0], b[1]]));
            if let (Some(qtype), Some(qclass)) = (qtype, qclass) {
                fields.push(field(
                    "Question",
                    format!("{name} {} {}", dns_type_name(qtype), dns_class_name(qclass)),
                    (base + 12, base + after_name + 4),
                ));
            }
        }
    }

    layers.push(ProtocolLayer {
        name: "Domain Name System".to_string(),
        fields,
    });
}

/// Builds the application layer for a payload.
fn parse_application_layer(
    layers: &mut Vec<ProtocolLayer>,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
    is_tcp: bool,
    offset: usize,
) {
    if parse_http_layer(layers, payload, offset) {
        return;
    }

    // DNS (port 53)
    if src_port == 53 || dst_port == 53 {
        parse_dns_layer(layers, payload, is_tcp, offset);
        return;
    }

    // Generic application layer: nothing recognised, so only report the bytes.
    layers.push(ProtocolLayer {
        name: "Application Data".to_string(),
        fields: vec![field(
            "Payload Length",
            format!("{} bytes", payload.len()),
            (offset, offset + payload.len()),
        )],
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_summary_ipv4_tcp() {
        // Create a mock IPv4 TCP packet
        let mut data = Vec::new();

        // Ethernet header (14 bytes)
        data.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]); // dst mac
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]); // src mac
        data.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4

        // IPv4 header (20 bytes)
        data.extend_from_slice(&[0x45, 0x00, 0x00, 0x3C]); // version, ihl, tos, total len
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // id, flags, frag offset
        data.extend_from_slice(&[0x40, 0x06, 0x00, 0x00]); // ttl, protocol (TCP), checksum
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]); // src ip: 192.168.1.1
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]); // dst ip: 192.168.1.2

        // TCP header (20 bytes)
        data.extend_from_slice(&[0xD4, 0x31, 0x00, 0x50]); // src port 54321, dst port 80
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // seq number
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // ack number
        data.extend_from_slice(&[0x50, 0x02, 0x20, 0x00]); // data offset, flags, window
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum, urgent pointer

        let result = parse_summary(&data, 1, 1_000_000_000);
        assert!(result.is_some());

        let summary = result.unwrap();
        assert_eq!(summary.id, 1);
        assert_eq!(summary.timestamp, 1_000_000_000);
        assert_eq!(summary.source_addr, "192.168.1.1");
        assert_eq!(summary.dest_addr, "192.168.1.2");
        // The fixture is a bare SYN with no payload: port 80 alone must not be
        // enough to claim the segment is HTTP.
        assert_eq!(summary.protocol, "TCP");
        assert_eq!(summary.info, "192.168.1.1:54321 → 192.168.1.2:80");
        assert_eq!(summary.length, data.len() as u32);
        assert_eq!(summary.src_port, Some(54321));
        assert_eq!(summary.dst_port, Some(80));
    }

    #[test]
    fn parse_summary_labels_http_from_the_payload_not_the_port() {
        let data = tcp_frame_with_payload(b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n");
        let summary = parse_summary(&data, 1, 1_000_000_000).unwrap();
        assert_eq!(summary.protocol, "HTTP");

        // ... and an HTTP request on port 443 is still HTTP.
        let data = tcp_frame_with_payload_on(12345, 443, b"GET / HTTP/1.1\r\n\r\n");
        let summary = parse_summary(&data, 1, 1_000_000_000).unwrap();
        assert_eq!(summary.protocol, "HTTP");
    }

    #[test]
    fn parse_summary_labels_tls_regardless_of_port() {
        // A TLS handshake record on port 80.
        let data = tcp_frame_with_payload_on(
            12345,
            80,
            &[0x16, 0x03, 0x01, 0x00, 0x05, 0x01, 0x00, 0x00, 0x01],
        );
        let summary = parse_summary(&data, 1, 1_000_000_000).unwrap();
        assert_eq!(summary.protocol, "HTTPS");
    }

    #[test]
    fn parse_summary_reports_the_icmp_message_type() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00; 12]); // ethernet
        data.extend_from_slice(&[0x08, 0x00]); // ipv4
        data.extend_from_slice(&[0x45, 0x00, 0x00, 0x1c]); // ver/ihl, tos, total len 28
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x01, 0x00, 0x00]); // ttl, protocol (ICMP)
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]);
        data.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01]); // echo request

        let summary = parse_summary(&data, 1, 0).unwrap();
        assert_eq!(summary.protocol, "ICMP");
        assert_eq!(summary.info, "192.168.1.1 → 192.168.1.2 [Echo Request]");
    }

    #[test]
    fn test_parse_summary_ipv6() {
        let mut data = Vec::new();
        // Ethernet
        data.extend_from_slice(&[0x00; 12]);
        data.extend_from_slice(&[0x86, 0xDD]); // IPv6

        // IPv6 Header (40 bytes)
        data.extend_from_slice(&[0x60, 0x00, 0x00, 0x00]); // Version 6
        data.extend_from_slice(&[0x00, 0x00, 0x06, 0x40]); // Payload len 0, Next Header TCP, Hop limit 64
        data.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]); // Src: fe80::1
        data.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]); // Dst: fe80::2

        let result = parse_summary(&data, 1, 0);
        assert!(result.is_some());
        let summary = result.unwrap();
        assert_eq!(summary.protocol, "TCP");
        assert_eq!(summary.source_addr, "fe80::1");
        assert_eq!(summary.dest_addr, "fe80::2");
    }

    #[test]
    fn test_dissect_packet_with_layers() {
        // Create a mock IPv4 TCP packet
        let mut data = Vec::new();

        // Ethernet header
        data.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);
        data.extend_from_slice(&[0x08, 0x00]);

        // IPv4 header
        data.extend_from_slice(&[0x45, 0x00, 0x00, 0x3C]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x06, 0x00, 0x00]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]);

        // TCP header
        data.extend_from_slice(&[0xD4, 0x31, 0x00, 0x50]);
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        data.extend_from_slice(&[0x50, 0x02, 0x20, 0x00]);
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        let result = dissect_packet(&data, 1, 1_000_000_000);
        assert!(result.is_some());

        let detail = result.unwrap();
        assert!(detail.layers.len() >= 3); // Ethernet + IP + TCP
        assert_eq!(detail.layers[0].name, "Ethernet");
        assert_eq!(detail.layers[0].fields[0].name, "Destination");
        assert_eq!(detail.layers[0].fields[0].range, (0, 6));
    }

    #[test]
    fn extract_ports_is_none_for_non_transport_packets() {
        // Minimal Ethernet + ARP frame: no transport layer, so no ports.
        let mut data = Vec::new();
        data.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // dst mac
        data.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]); // src mac
        data.extend_from_slice(&[0x08, 0x06]); // ethertype ARP
        data.extend_from_slice(&[0u8; 28]); // ARP body

        assert!(extract_ports(&data).is_none());

        let summary = parse_summary(&data, 1, 0).expect("ARP summary should parse");
        assert_eq!(summary.protocol, "ARP");
        assert_eq!(summary.src_port, None);
        assert_eq!(summary.dst_port, None);
    }

    /// Ethernet + IPv4 + TCP frame with a zeroed IPv4 checksum field.
    fn ipv4_tcp_frame() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]); // dst mac
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]); // src mac
        data.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4
        data.extend_from_slice(&[0x45, 0x00, 0x00, 0x3C]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x06, 0x00, 0x00]); // ttl, protocol, checksum
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]);
        data.extend_from_slice(&[0xD4, 0x31, 0x00, 0x50]);
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        data.extend_from_slice(&[0x50, 0x02, 0x20, 0x00]);
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        data
    }

    fn ip_checksum_field(detail: &PacketDetail) -> (&str, (usize, usize)) {
        let field = detail
            .layers
            .iter()
            .find(|l| l.name == "Internet Protocol Version 4")
            .and_then(|l| l.fields.iter().find(|f| f.name == "Header Checksum"))
            .expect("IPv4 layer should expose Header Checksum");
        (field.value.as_str(), field.range)
    }

    #[test]
    fn ipv4_header_checksum_matches_the_worked_example() {
        // RFC 1071 worked example: the stored checksum is 0xb861.
        let header = [
            0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0xb8, 0x61, 0xc0, 0xa8,
            0x00, 0x01, 0xc0, 0xa8, 0x00, 0xc7,
        ];

        let (stored, computed) = verify_ipv4_checksum(&header).expect("20-byte header verifies");
        assert_eq!(stored, 0xb861);
        assert_eq!(computed, stored);
    }

    #[test]
    fn ipv4_header_checksum_detects_modification_and_truncation() {
        let mut header = [
            0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0xb8, 0x61, 0xc0, 0xa8,
            0x00, 0x01, 0xc0, 0xa8, 0x00, 0xc7,
        ];
        header[19] ^= 0x01; // alter the last byte of the destination address

        let (stored, computed) = verify_ipv4_checksum(&header).unwrap();
        assert_eq!(stored, 0xb861);
        assert_ne!(computed, stored);

        assert!(verify_ipv4_checksum(&header[..19]).is_none());
    }

    #[test]
    fn dissect_packet_reports_the_ipv4_header_checksum() {
        let mut data = ipv4_tcp_frame();
        // Fill in a valid checksum at bytes 10-11 of the IPv4 header (offset 14).
        let (_, computed) = verify_ipv4_checksum(&data[14..34]).unwrap();
        data[24] = (computed >> 8) as u8;
        data[25] = (computed & 0xff) as u8;

        let detail = dissect_packet(&data, 1, 1_000_000_000).unwrap();
        let (value, range) = ip_checksum_field(&detail);
        assert_eq!(range, (24, 26));
        assert!(value.contains("correct"), "got {value}");
        assert!(!detail
            .expert_summary
            .iter()
            .any(|e| e.contains("checksum invalid")));

        // Corrupt the destination address: the stored checksum no longer matches.
        data[33] ^= 0x01;
        let detail = dissect_packet(&data, 1, 1_000_000_000).unwrap();
        let (value, _) = ip_checksum_field(&detail);
        assert!(value.contains("incorrect"), "got {value}");
        assert!(detail
            .expert_summary
            .iter()
            .any(|e| e.contains("checksum invalid")));
    }

    /// Ethernet + IPv4 + TCP frame carrying `payload` at sequence number 0.
    fn tcp_frame_with_payload_on(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
        tcp_frame_with_seq_on(src_port, dst_port, 0, payload)
    }

    /// Ethernet + IPv4 + TCP frame carrying `payload`, claiming sequence `seq`.
    fn tcp_frame_with_seq_on(src_port: u16, dst_port: u16, seq: u32, payload: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]); // dst mac
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]); // src mac
        data.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4

        let total_len = 20 + 20 + payload.len();
        data.extend_from_slice(&[0x45, 0x00, (total_len >> 8) as u8, total_len as u8]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x06, 0x00, 0x00]); // ttl, protocol (TCP)
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]);

        data.extend_from_slice(&[
            (src_port >> 8) as u8,
            src_port as u8,
            (dst_port >> 8) as u8,
            dst_port as u8,
        ]);
        data.extend_from_slice(&seq.to_be_bytes()); // seq
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // ack
        data.extend_from_slice(&[0x50, 0x18, 0x20, 0x00]); // header len, PSH+ACK, window
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum, urgent pointer
        data.extend_from_slice(payload);

        // A valid IPv4 header checksum keeps the fixture free of findings the
        // tests are not about.
        let (_, checksum) = verify_ipv4_checksum(&data[14..34]).expect("full IPv4 header");
        data[24] = (checksum >> 8) as u8;
        data[25] = (checksum & 0xff) as u8;
        data
    }

    fn tcp_frame_with_payload(payload: &[u8]) -> Vec<u8> {
        tcp_frame_with_payload_on(54321, 80, payload)
    }

    /// Ethernet + IPv4 + ICMP echo request with a valid checksum.
    fn icmp_echo_frame() -> Vec<u8> {
        let mut icmp = vec![0x08, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01];
        let mut zeroed = icmp.clone();
        zeroed[2] = 0;
        zeroed[3] = 0;
        let checksum = internet_checksum(&zeroed);
        icmp[2] = (checksum >> 8) as u8;
        icmp[3] = (checksum & 0xff) as u8;

        let mut data = Vec::new();
        data.extend_from_slice(&[0x00; 12]);
        data.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4
        data.extend_from_slice(&[0x45, 0x00, 0x00, 28]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x01, 0x00, 0x00]); // ttl, protocol (ICMP)
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]);
        data.extend_from_slice(&icmp);
        data
    }

    fn icmp_layer(detail: &PacketDetail) -> &ProtocolLayer {
        detail
            .layers
            .iter()
            .find(|l| l.name == "Internet Control Message Protocol")
            .expect("ICMP layer should be dissected")
    }

    #[test]
    fn detect_app_protocol_checks_both_ports() {
        // 5353 (mDNS) is at or above 1024, so the old "destination if below
        // 1024, else source" rule consulted the client's ephemeral port and
        // reported plain UDP for the query while reporting mDNS for the reply.
        assert_eq!(
            detect_app_protocol(51234, 5353, b"query").unwrap().0,
            "mDNS"
        );
        assert_eq!(
            detect_app_protocol(5353, 51234, b"reply").unwrap().0,
            "mDNS"
        );

        // Same for SSDP discovery sent to a client's ephemeral port from 1900.
        assert_eq!(
            detect_app_protocol(1900, 40000, b"HTTP/1.1 200 OK\r\n")
                .unwrap()
                .0,
            "SSDP"
        );

        // Destination below 1024 but unknown must not hide a known source port.
        assert_eq!(detect_app_protocol(53, 1234, &[0u8; 12]).unwrap().0, "DNS");
    }

    #[test]
    fn classify_tcp_never_claims_control_segments_are_http() {
        assert!(classify_tcp(54321, 80, b"").is_none());
        assert!(classify_tcp(80, 54321, b"").is_none());
        assert!(classify_tcp(54321, 443, b"").is_none());
    }

    #[test]
    fn classify_tcp_prefers_payload_evidence() {
        assert_eq!(
            classify_tcp(54321, 443, b"GET / HTTP/1.1\r\n\r\n")
                .unwrap()
                .0,
            "HTTP"
        );
        assert_eq!(
            classify_tcp(54321, 80, &[0x16, 0x03, 0x03, 0x00, 0x04])
                .unwrap()
                .0,
            "HTTPS"
        );
        // Payload we cannot identify falls back to the well-known port.
        assert_eq!(
            classify_tcp(54321, 80, &[0x00, 0x01, 0x02, 0x03])
                .unwrap()
                .0,
            "HTTP"
        );
        assert!(classify_tcp(54321, 4433, &[0x00, 0x01, 0x02, 0x03]).is_none());
    }

    #[test]
    fn dissect_packet_builds_an_icmp_layer() {
        let mut data = icmp_echo_frame();

        let detail = dissect_packet(&data, 1, 0).unwrap();
        let layer = icmp_layer(&detail);
        let type_field = layer
            .fields
            .iter()
            .find(|f| f.name == "Type")
            .expect("Type field");
        assert!(
            type_field.value.contains("Echo Request"),
            "got {type_field:?}"
        );
        let checksum = layer
            .fields
            .iter()
            .find(|f| f.name == "Checksum")
            .expect("Checksum field");
        assert!(checksum.value.contains("correct"), "got {checksum:?}");
        assert!(layer.fields.iter().any(|f| f.name == "Identifier"));
        assert!(layer.fields.iter().any(|f| f.name == "Sequence Number"));
        assert!(!detail
            .expert_summary
            .iter()
            .any(|e| e.contains("ICMP checksum incorrect")));

        // Corrupt the identifier: the checksum no longer verifies.
        data[14 + 20 + 4] ^= 0x01;
        let detail = dissect_packet(&data, 1, 0).unwrap();
        let checksum = icmp_layer(&detail)
            .fields
            .iter()
            .find(|f| f.name == "Checksum")
            .expect("Checksum field");
        assert!(checksum.value.contains("incorrect"), "got {checksum:?}");
        assert!(detail
            .expert_summary
            .iter()
            .any(|e| e.contains("ICMP checksum incorrect")));
    }

    #[test]
    fn dissect_packet_builds_an_icmpv6_layer() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00; 12]);
        data.extend_from_slice(&[0x86, 0xDD]); // ethertype IPv6
        data.extend_from_slice(&[0x60, 0x00, 0x00, 0x00]);
        data.extend_from_slice(&[0x00, 0x08, 0x3a, 0x40]); // payload len 8, next header 58, hop 64
        data.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        data.extend_from_slice(&[0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        data.extend_from_slice(&[135, 0, 0, 0, 0, 0, 0, 0]); // neighbor solicitation

        let detail = dissect_packet(&data, 1, 0).unwrap();
        let layer = detail
            .layers
            .iter()
            .find(|l| l.name == "Internet Control Message Protocol v6")
            .expect("ICMPv6 layer should be dissected");
        let type_field = layer
            .fields
            .iter()
            .find(|f| f.name == "Type")
            .expect("Type field");
        assert!(
            type_field.value.contains("Neighbor Solicitation"),
            "got {type_field:?}"
        );
        // The ICMPv6 checksum covers the IPv6 pseudo-header, so it is reported
        // but not claimed to be verified.
        let checksum = layer
            .fields
            .iter()
            .find(|f| f.name == "Checksum")
            .expect("Checksum field");
        assert!(checksum.value.contains("not verified"), "got {checksum:?}");

        assert_eq!(detail.summary.protocol, "ICMPv6");
        assert!(detail.summary.info.contains("Neighbor Solicitation"));
    }

    /// Ethernet + IPv4 + UDP frame carrying `payload`.
    fn udp_frame_with_payload_on(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);
        data.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4

        let total_len = 20 + 8 + payload.len();
        data.extend_from_slice(&[0x45, 0x00, (total_len >> 8) as u8, total_len as u8]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x11, 0x00, 0x00]); // ttl, protocol (UDP)
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]);

        let udp_len = 8 + payload.len();
        data.extend_from_slice(&[
            (src_port >> 8) as u8,
            src_port as u8,
            (dst_port >> 8) as u8,
            dst_port as u8,
        ]);
        data.extend_from_slice(&[(udp_len >> 8) as u8, udp_len as u8]);
        data.extend_from_slice(&[0x00, 0x00]); // checksum
        data.extend_from_slice(payload);

        let (_, checksum) = verify_ipv4_checksum(&data[14..34]).expect("full IPv4 header");
        data[24] = (checksum >> 8) as u8;
        data[25] = (checksum & 0xff) as u8;
        data
    }

    /// One-question DNS message for `name` with the given header flags.
    fn dns_message(name: &str, flags: u16) -> Vec<u8> {
        let mut msg = vec![
            0x12,
            0x34, // transaction id
            (flags >> 8) as u8,
            flags as u8,
            0x00,
            0x01, // qdcount
            0x00,
            0x00, // ancount
            0x00,
            0x00, // nscount
            0x00,
            0x00, // arcount
        ];
        for label in name.split('.') {
            msg.push(label.len() as u8);
            msg.extend_from_slice(label.as_bytes());
        }
        msg.push(0);
        msg.extend_from_slice(&1u16.to_be_bytes()); // type A
        msg.extend_from_slice(&1u16.to_be_bytes()); // class IN
        msg
    }

    fn field_value<'a>(detail: &'a PacketDetail, layer: &str, name: &str) -> Option<&'a str> {
        detail
            .layers
            .iter()
            .find(|l| l.name == layer)?
            .fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.value.as_str())
    }

    #[test]
    fn dns_layer_shows_the_header_and_first_question() {
        let payload = dns_message("example.com", 0x0100); // standard query, RD
        let data = udp_frame_with_payload_on(51234, 53, &payload);
        let detail = dissect_packet(&data, 1, 0).unwrap();

        let layer = "Domain Name System";
        assert_eq!(
            field_value(&detail, layer, "Transaction ID"),
            Some("0x1234")
        );
        assert_eq!(field_value(&detail, layer, "Message Type"), Some("Query"));
        assert_eq!(field_value(&detail, layer, "Questions"), Some("1"));
        assert_eq!(
            field_value(&detail, layer, "Question"),
            Some("example.com A IN")
        );
        // Not a response, so no reply code is shown.
        assert_eq!(field_value(&detail, layer, "Response Code"), None);

        assert_eq!(detail.summary.protocol, "DNS");
        assert_eq!(detail.summary.info, "DNS Query");
    }

    #[test]
    fn dns_response_reports_the_rcode() {
        // 0x8183: response, recursion desired + available, rcode 3 (NXDOMAIN)
        let payload = dns_message("nope.example", 0x8183);
        let data = udp_frame_with_payload_on(53, 51234, &payload);
        let detail = dissect_packet(&data, 1, 0).unwrap();

        let layer = "Domain Name System";
        assert_eq!(
            field_value(&detail, layer, "Message Type"),
            Some("Response")
        );
        assert_eq!(
            field_value(&detail, layer, "Response Code"),
            Some("Name Error (NXDOMAIN)")
        );
        assert_eq!(detail.summary.info, "DNS Response");
    }

    #[test]
    fn dns_over_tcp_skips_the_length_prefix() {
        let message = dns_message("example.com", 0x0100);
        let mut payload = vec![(message.len() >> 8) as u8, message.len() as u8];
        payload.extend_from_slice(&message);

        let data = tcp_frame_with_payload_on(51234, 53, &payload);
        let detail = dissect_packet(&data, 1, 0).unwrap();

        let layer = "Domain Name System";
        assert_eq!(
            field_value(&detail, layer, "Transaction ID"),
            Some("0x1234")
        );
        assert_eq!(
            field_value(&detail, layer, "Question"),
            Some("example.com A IN")
        );
        assert_eq!(detail.summary.protocol, "DNS");
    }

    #[test]
    fn http_layer_is_built_from_the_payload_on_any_port() {
        let body = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\n\r\n";
        // 8080 is a proxy port the old port-80 check never looked at.
        let data = tcp_frame_with_payload_on(51234, 8080, body);
        let detail = dissect_packet(&data, 1, 0).unwrap();

        let layer = "Hypertext Transfer Protocol";
        assert_eq!(field_value(&detail, layer, "Method"), Some("GET"));
        assert_eq!(
            field_value(&detail, layer, "Request URI"),
            Some("/index.html")
        );
        assert_eq!(field_value(&detail, layer, "Version"), Some("HTTP/1.1"));
        assert_eq!(field_value(&detail, layer, "Host"), Some("example.com"));
    }

    #[test]
    fn http_response_shows_the_status_code() {
        let body = b"HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\n\r\n<html></html>";
        let data = tcp_frame_with_payload_on(80, 51234, body);
        let detail = dissect_packet(&data, 1, 0).unwrap();

        let layer = "Hypertext Transfer Protocol";
        assert_eq!(field_value(&detail, layer, "Status Code"), Some("404"));
        assert_eq!(
            field_value(&detail, layer, "Reason Phrase"),
            Some("Not Found")
        );
        assert_eq!(
            field_value(&detail, layer, "Content-Type"),
            Some("text/html")
        );
    }

    #[test]
    fn binary_payload_on_port_80_gets_no_http_layer() {
        let data = tcp_frame_with_payload_on(51234, 80, &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
        let detail = dissect_packet(&data, 1, 0).unwrap();

        assert!(!detail
            .layers
            .iter()
            .any(|l| l.name == "Hypertext Transfer Protocol"));
        assert!(detail.layers.iter().any(|l| l.name == "Application Data"));
    }

    #[test]
    fn artifacts_are_detected_in_the_payload_not_the_frame() {
        let body = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n";
        let data = tcp_frame_with_payload_on(12345, 12346, body);
        let detail = dissect_packet(&data, 1, 0).unwrap();

        assert_eq!(detail.artifacts.len(), 1);
        let artifact = &detail.artifacts[0];
        assert_eq!(artifact.name, "PDF document");
        assert_eq!(artifact.mime_type, "application/pdf");
        assert_eq!(artifact.size, body.len());
        assert_eq!(artifact.hash_sha256, compute_sha256(body));
    }

    #[test]
    fn a_mac_address_matching_a_signature_is_not_an_artifact() {
        // The destination MAC spells "%PDF-"; run against the raw frame the
        // old detector turned that into a PDF artifact.
        let mut data = Vec::new();
        data.extend_from_slice(b"%PDF-\n");
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);
        data.extend_from_slice(&[0x08, 0x00]);
        data.extend_from_slice(&[0x45, 0x00, 0x00, 0x1C]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]);
        data.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01]);

        let detail = dissect_packet(&data, 1, 0).unwrap();
        assert!(detail.artifacts.is_empty());
    }

    #[test]
    fn risk_score_is_evidence_based() {
        // A clean packet scores 0: nothing was found.
        let mut data = ipv4_tcp_frame();
        let (_, computed) = verify_ipv4_checksum(&data[14..34]).unwrap();
        data[24] = (computed >> 8) as u8;
        data[25] = (computed & 0xff) as u8;

        let detail = dissect_packet(&data, 1, 0).unwrap();
        assert!(
            detail.expert_summary.is_empty(),
            "{:?}",
            detail.expert_summary
        );
        assert_eq!(detail.intelligence.risk_score, 0);

        // One observed finding (low TTL) adds 20; the checksum is repaired so
        // it does not count as a second finding.
        data[22] = 5;
        let (_, computed) = verify_ipv4_checksum(&data[14..34]).unwrap();
        data[24] = (computed >> 8) as u8;
        data[25] = (computed & 0xff) as u8;

        let detail = dissect_packet(&data, 1, 0).unwrap();
        assert!(detail.expert_summary.iter().any(|e| e.contains("low TTL")));
        assert_eq!(detail.intelligence.risk_score, 20);
    }

    #[test]
    fn risk_score_reflects_payload_entropy_and_files() {
        // Every byte value equally often → entropy 8.0, above the 7.5 bar.
        let uniform: Vec<u8> = (0u16..=255).flat_map(|b| [b as u8; 4]).collect();
        let data = tcp_frame_with_payload_on(12345, 12346, &uniform);
        let detail = dissect_packet(&data, 1, 0).unwrap();
        assert!(
            detail.intelligence.entropy > 7.5,
            "{}",
            detail.intelligence.entropy
        );
        assert_eq!(detail.intelligence.risk_score, 30);

        // The same payload carrying a file signature adds 20.
        let mut payload = b"%PDF-".to_vec();
        payload.extend_from_slice(&uniform);
        let data = tcp_frame_with_payload_on(12345, 12346, &payload);
        let detail = dissect_packet(&data, 1, 0).unwrap();
        assert_eq!(detail.artifacts.len(), 1);
        assert_eq!(detail.intelligence.risk_score, 50);
    }

    #[test]
    fn every_field_highlights_at_least_one_byte() {
        // HexView highlights `[start, end)`, so a field with an empty range
        // (start == end) highlights nothing when the row is hovered.
        let frames = vec![
            ipv4_tcp_frame(),
            icmp_echo_frame(),
            udp_frame_with_payload_on(12345, 53, &dns_message("example.com", 0x0100)),
            tcp_frame_with_payload_on(12345, 80, b"GET / HTTP/1.1\r\nHost: a\r\n\r\n"),
            tcp_frame_with_payload_on(
                12345,
                443,
                &[0x16, 0x03, 0x01, 0x00, 0x05, 0x01, 0x00, 0x00, 0x01, 0x00],
            ),
        ];

        for frame in frames {
            let detail = dissect_packet(&frame, 1, 0).expect("frame should dissect");
            assert!(!detail.layers.is_empty());
            for layer in &detail.layers {
                for field in &layer.fields {
                    assert!(
                        field.range.1 > field.range.0,
                        "{}.{} has an empty byte range {:?}",
                        layer.name,
                        field.name,
                        field.range
                    );
                }
            }
        }
    }

    fn seg(seq: u32, timestamp_ns: i64, payload: &[u8]) -> TcpSegment {
        TcpSegment {
            seq,
            timestamp_ns,
            payload: payload.to_vec(),
        }
    }

    #[test]
    fn get_tcp_segment_reads_sequence_and_payload() {
        let frame = tcp_frame_with_seq_on(54321, 80, 1000, b"hello");
        let segment = get_tcp_segment(&frame, 42).expect("TCP segment");
        assert_eq!(segment.seq, 1000);
        assert_eq!(segment.timestamp_ns, 42);
        assert_eq!(segment.payload, b"hello");
    }

    #[test]
    fn segments_without_payload_contribute_nothing() {
        // SYN/FIN and pure ACKs consume sequence numbers but carry no bytes.
        let ack = tcp_frame_with_payload_on(54321, 80, b"");
        assert!(get_tcp_segment(&ack, 1).is_none());
    }

    #[test]
    fn reassembly_orders_by_sequence_not_by_arrival() {
        let blocks = reassemble_tcp(vec![
            seg(6, 2, b" world"), // arrives first, belongs last
            seg(0, 1, b"hello"),
            seg(5, 3, b"!"),
        ]);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].data, b"hello! world");
        assert_eq!(blocks[0].missing_before, 0);
        // The timestamp belongs to the packet that carried the *first byte*,
        // not to whichever segment happened to arrive first.
        assert_eq!(blocks[0].timestamp_ns, 1);
    }

    #[test]
    fn retransmissions_and_overlaps_contribute_no_byte_twice() {
        let blocks = reassemble_tcp(vec![
            seg(0, 1, b"hello"),
            seg(0, 2, b"hello"), // full retransmission
            seg(5, 3, b" world"),
            seg(3, 4, b"lo world"), // late segment overlapping bytes 3..11
        ]);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].data, b"hello world");
    }

    #[test]
    fn the_first_segment_to_claim_a_byte_keeps_it() {
        let blocks = reassemble_tcp(vec![seg(0, 1, b"AAA"), seg(0, 2, b"BBB")]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].data, b"AAA");
    }

    #[test]
    fn a_gap_splits_the_stream_and_is_reported() {
        let blocks = reassemble_tcp(vec![
            seg(0, 1, b"hello"),
            seg(100, 3, b"world"),
            seg(5, 2, b"..."), // arrives after the segment that follows the gap
        ]);

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].data, b"hello...");
        assert_eq!(blocks[0].missing_before, 0);
        assert_eq!(blocks[1].data, b"world");
        // Bytes 8..100 were never captured — reported, not invented.
        assert_eq!(blocks[1].missing_before, 92);
    }

    #[test]
    fn a_stream_we_started_reading_midway_claims_nothing_missing() {
        // There is no earlier segment to compare against, so the first block
        // never reports missing bytes — gaps are only claimed *between*
        // captured parts of the stream, where we can actually prove them.
        let blocks = reassemble_tcp(vec![seg(1000, 7, b"rest of it")]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].missing_before, 0);
        assert_eq!(blocks[0].data, b"rest of it");
    }

    #[test]
    fn sequence_numbers_that_wrap_are_still_ordered() {
        let blocks = reassemble_tcp(vec![
            seg(0x0000_0000, 2, b"def"), // after the wrap
            seg(0xFFFF_FFFD, 1, b"abc"), // before it
        ]);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].data, b"abcdef");
        assert_eq!(blocks[0].missing_before, 0);
        assert_eq!(blocks[0].timestamp_ns, 1);
    }

    #[test]
    fn nothing_in_produces_nothing_out() {
        assert!(reassemble_tcp(Vec::new()).is_empty());
    }
}
