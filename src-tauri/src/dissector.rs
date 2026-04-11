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

fn detect_artifacts(data: &[u8]) -> Vec<Artifact> {
    let mut artifacts = Vec::new();
    if data.len() < 8 {
        return artifacts;
    }

    let hash = compute_sha256(data);

    // PDF: %PDF-
    if data.starts_with(&[0x25, 0x50, 0x44, 0x46, 0x2D]) {
        artifacts.push(Artifact {
            name: "Document.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size: data.len(),
            hash_sha256: hash,
        });
    }
    // JPEG: FF D8 FF
    else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        artifacts.push(Artifact {
            name: "Image.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            size: data.len(),
            hash_sha256: hash,
        });
    }
    // PNG: 89 50 4E 47
    else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        artifacts.push(Artifact {
            name: "Image.png".to_string(),
            mime_type: "image/png".to_string(),
            size: data.len(),
            hash_sha256: hash,
        });
    }

    artifacts
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
fn detect_app_protocol(src_port: u16, dst_port: u16, payload: &[u8]) -> Option<(String, String)> {
    let port = if dst_port < 1024 { dst_port } else { src_port };

    match port {
        53 => Some(("DNS".to_string(), "DNS Query".to_string())),
        67 | 68 => Some((
            "DHCP".to_string(),
            if payload.get(0) == Some(&1) {
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
        _ => {
            if payload.len() >= 4 {
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
            } else {
                None
            }
        }
    }
}

// Lightweight parser for the packet list view
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
                        let (proto, info_str) = if dst_port == 80 || src_port == 80 {
                            (PROTO_HTTP.to_string(), "HTTP".to_string())
                        } else if dst_port == 443 || src_port == 443 {
                            (PROTO_HTTPS.to_string(), "TLS/SSL".to_string())
                        } else {
                            (
                                PROTO_TCP.to_string(),
                                format!("{}:{} → {}:{}", src, src_port, dst, dst_port),
                            )
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
                    format!("{} → {} [ICMP]", src, dst),
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
                        let (proto, info_str) = if dst_port == 80 || src_port == 80 {
                            (PROTO_HTTP.to_string(), "HTTP".to_string())
                        } else if dst_port == 443 || src_port == 443 {
                            (PROTO_HTTPS.to_string(), "TLS/SSL".to_string())
                        } else {
                            (
                                PROTO_TCP.to_string(),
                                format!("{}:{} → {}:{}", src, src_port, dst, dst_port),
                            )
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
                    format!("{} → {} [ICMPv6]", src, dst),
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

    Some(PacketSummary {
        id,
        timestamp: timestamp_ns,
        source_addr,
        dest_addr,
        protocol,
        length: raw_data.len() as u32,
        info,
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
                if ttl < 10 {
                    expert_summary.push("Suspiciously low TTL (Time To Live). Possible traceroute or network manipulation.".to_string());
                }

                let ip_fields = vec![
                    PacketField {
                        name: "Version".to_string(),
                        value: "4".to_string(),
                        range: (current_offset, current_offset),
                        expert: None,
                    },
                    PacketField {
                        name: "Header Length".to_string(),
                        value: format!("{} bytes", header_len),
                        range: (current_offset, current_offset),
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
                        range: (current_offset, current_offset),
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
                    _ => {}
                }
            }
        }
        _ => {}
    }

    // Get summary
    let summary = parse_summary(raw_data, id, timestamp_ns)?;

    let entropy = calculate_entropy(raw_data);
    let narrative = generate_narrative(&summary, &layers);
    let artifacts = detect_artifacts(raw_data);

    Some(PacketDetail {
        summary,
        layers,
        raw_bytes: raw_data.to_vec(),
        expert_summary,
        narrative,
        intelligence: ForensicIntelligence {
            entropy,
            ja3_hash: None,
            manufacturer,
            risk_score: if entropy > 7.5 { 70 } else { 10 },
        },
        artifacts,
    })
}

// Application layer parsing
fn parse_application_layer(
    layers: &mut Vec<ProtocolLayer>,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
    is_tcp: bool,
    offset: usize,
) {
    let payload_len = payload.len();
    let range = (offset, offset + payload_len);

    // DNS (port 53)
    if src_port == 53 || dst_port == 53 {
        layers.push(ProtocolLayer {
            name: "Domain Name System".to_string(),
            fields: vec![
                PacketField {
                    name: "Port".to_string(),
                    value: if src_port == 53 {
                        "53 (Response)".to_string()
                    } else {
                        "53 (Query)".to_string()
                    },
                    range: (offset, offset + 2),
                    expert: None,
                },
                PacketField {
                    name: "Payload Length".to_string(),
                    value: format!("{} bytes", payload_len),
                    range,
                    expert: None,
                },
            ],
        });
        return;
    }

    // HTTP (port 80)
    if is_tcp && (src_port == 80 || dst_port == 80) {
        if let Ok(payload_str) = std::str::from_utf8(&payload[..payload_len.min(100)]) {
            let mut http_fields = Vec::new();
            if payload_str.starts_with("GET") || payload_str.starts_with("POST") {
                http_fields.push(PacketField {
                    name: "Method".to_string(),
                    value: payload_str.split(' ').next().unwrap_or("").to_string(),
                    range: (offset, offset + 4),
                    expert: None,
                });
            }
            http_fields.push(PacketField {
                name: "Data".to_string(),
                value: format!("{} bytes", payload_len),
                range,
                expert: None,
            });
            layers.push(ProtocolLayer {
                name: "Hypertext Transfer Protocol".to_string(),
                fields: http_fields,
            });
            return;
        }
    }

    // Generic application layer
    layers.push(ProtocolLayer {
        name: "Application Data".to_string(),
        fields: vec![PacketField {
            name: "Payload Length".to_string(),
            value: format!("{} bytes", payload_len),
            range,
            expert: None,
        }],
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
        assert_eq!(summary.protocol, "TCP");
        assert_eq!(summary.length, data.len() as u32);
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
}
