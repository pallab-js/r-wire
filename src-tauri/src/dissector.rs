use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::Packet;
use crate::model::{PacketSummary, PacketDetail, ProtocolLayer};

// Protocol name constants to avoid repeated string allocations
const PROTO_TCP: &str = "TCP";
const PROTO_UDP: &str = "UDP";
const PROTO_ICMP: &str = "ICMP";
const PROTO_ICMPV6: &str = "ICMPv6";
const PROTO_IPV4: &str = "IPv4";
const PROTO_IPV6: &str = "IPv6";
const PROTO_ARP: &str = "ARP";
const PROTO_UNKNOWN: &str = "Unknown";

// Lightweight parser for the packet list view
pub fn parse_summary(raw_data: &[u8], id: u64, timestamp_ns: i64) -> Option<PacketSummary> {
    let ethernet = EthernetPacket::new(raw_data)?;
    
    let (source_addr, dest_addr, protocol, info) = match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ipv4 = Ipv4Packet::new(ethernet.payload())?;
            let src = ipv4.get_source().to_string();
            let dst = ipv4.get_destination().to_string();
            let proto = match ipv4.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => PROTO_TCP,
                IpNextHeaderProtocols::Udp => PROTO_UDP,
                IpNextHeaderProtocols::Icmp => PROTO_ICMP,
                _ => PROTO_IPV4,
            };
            let info_str = match ipv4.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => {
                    if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                        format!("{} → {} [{}]", src, dst, tcp.get_destination())
                    } else {
                        format!("{} → {}", src, dst)
                    }
                }
                IpNextHeaderProtocols::Udp => {
                    if let Some(udp) = UdpPacket::new(ipv4.payload()) {
                        format!("{} → {} [{}]", src, dst, udp.get_destination())
                    } else {
                        format!("{} → {}", src, dst)
                    }
                }
                _ => format!("{} → {}", src, dst),
            };
            (src, dst, proto.to_string(), info_str)
        }
        EtherTypes::Ipv6 => {
            let ipv6 = Ipv6Packet::new(ethernet.payload())?;
            let src = ipv6.get_source().to_string();
            let dst = ipv6.get_destination().to_string();
            let proto = match ipv6.get_next_header() {
                IpNextHeaderProtocols::Tcp => PROTO_TCP,
                IpNextHeaderProtocols::Udp => PROTO_UDP,
                IpNextHeaderProtocols::Icmpv6 => PROTO_ICMPV6,
                _ => PROTO_IPV6,
            };
            let info_str = format!("{} → {}", src, dst);
            (src, dst, proto.to_string(), info_str)
        }
        EtherTypes::Arp => {
            let src = ethernet.get_source().to_string();
            let dst = ethernet.get_destination().to_string();
            (src, dst, PROTO_ARP.to_string(), PROTO_ARP.to_string())
        }
        _ => {
            let src = ethernet.get_source().to_string();
            let dst = ethernet.get_destination().to_string();
            (src, dst, PROTO_UNKNOWN.to_string(), PROTO_UNKNOWN.to_string())
        }
    };

    // Use the actual packet timestamp from pcap
    let timestamp = timestamp_ns;

    Some(PacketSummary {
        id,
        timestamp,
        source_addr,
        dest_addr,
        protocol,
        length: raw_data.len() as u32,
        info,
    })
}

// Full packet dissection for detail view
pub fn dissect_packet(raw_data: &[u8], id: u64) -> Option<PacketDetail> {
    let mut layers = Vec::new();
    
    // Parse Ethernet layer (L2)
    let ethernet = EthernetPacket::new(raw_data)?;
    let ethernet_layer = ProtocolLayer {
        name: "Ethernet".to_string(),
        fields: vec![
            ("Destination".to_string(), ethernet.get_destination().to_string()),
            ("Source".to_string(), ethernet.get_source().to_string()),
            ("Type".to_string(), format!("0x{:04x}", ethernet.get_ethertype().0)),
        ],
    };
    layers.push(ethernet_layer);

    // Parse IP layer (L3)
    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ipv4 = Ipv4Packet::new(ethernet.payload())?;
            let ip_layer = ProtocolLayer {
                name: "Internet Protocol Version 4".to_string(),
                fields: vec![
                    ("Version".to_string(), "4".to_string()),
                    ("Header Length".to_string(), format!("{} bytes", ipv4.get_header_length() * 4)),
                    ("Total Length".to_string(), format!("{} bytes", ipv4.get_total_length())),
                    ("Identification".to_string(), format!("0x{:04x}", ipv4.get_identification())),
                    ("Flags".to_string(), format!("0x{:02x}", ipv4.get_flags())),
                    ("TTL".to_string(), ipv4.get_ttl().to_string()),
                    ("Protocol".to_string(), format!("{} ({})", ipv4.get_next_level_protocol().0, ipv4.get_next_level_protocol())),
                    ("Source".to_string(), ipv4.get_source().to_string()),
                    ("Destination".to_string(), ipv4.get_destination().to_string()),
                ],
            };
            layers.push(ip_layer);

            // Parse Transport layer (L4)
            match ipv4.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => {
                    if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                        let tcp_layer = ProtocolLayer {
                            name: "Transmission Control Protocol".to_string(),
                            fields: vec![
                                ("Source Port".to_string(), tcp.get_source().to_string()),
                                ("Destination Port".to_string(), tcp.get_destination().to_string()),
                                ("Sequence Number".to_string(), tcp.get_sequence().to_string()),
                                ("Acknowledgment Number".to_string(), tcp.get_acknowledgement().to_string()),
                                ("Data Offset".to_string(), format!("{} bytes", (tcp.get_data_offset() as u32) * 4)),
                                ("Flags".to_string(), format!("0x{:02x}", tcp.get_flags())),
                                ("Window Size".to_string(), tcp.get_window().to_string()),
                            ],
                        };
                        layers.push(tcp_layer);

                        // Application layer parsing (F6)
                        let payload = tcp.payload();
                        if !payload.is_empty() {
                            parse_application_layer(&mut layers, tcp.get_source(), tcp.get_destination(), payload, true);
                        }
                    }
                }
                IpNextHeaderProtocols::Udp => {
                    if let Some(udp) = UdpPacket::new(ipv4.payload()) {
                        let udp_layer = ProtocolLayer {
                            name: "User Datagram Protocol".to_string(),
                            fields: vec![
                                ("Source Port".to_string(), udp.get_source().to_string()),
                                ("Destination Port".to_string(), udp.get_destination().to_string()),
                                ("Length".to_string(), format!("{} bytes", udp.get_length())),
                                ("Checksum".to_string(), format!("0x{:04x}", udp.get_checksum())),
                            ],
                        };
                        layers.push(udp_layer);

                        // Application layer parsing (F6)
                        let payload = udp.payload();
                        if !payload.is_empty() {
                            parse_application_layer(&mut layers, udp.get_source(), udp.get_destination(), payload, false);
                        }
                    }
                }
                IpNextHeaderProtocols::Icmp => {
                    let icmp_layer = ProtocolLayer {
                        name: "Internet Control Message Protocol".to_string(),
                        fields: vec![
                            ("Payload Length".to_string(), format!("{} bytes", ipv4.payload().len())),
                        ],
                    };
                    layers.push(icmp_layer);
                }
                _ => {}
            }
        }
        EtherTypes::Ipv6 => {
            let ipv6 = Ipv6Packet::new(ethernet.payload())?;
            let ip_layer = ProtocolLayer {
                name: "Internet Protocol Version 6".to_string(),
                fields: vec![
                    ("Version".to_string(), "6".to_string()),
                    ("Traffic Class".to_string(), format!("0x{:02x}", ipv6.get_traffic_class())),
                    ("Flow Label".to_string(), format!("0x{:05x}", ipv6.get_flow_label())),
                    ("Payload Length".to_string(), format!("{} bytes", ipv6.get_payload_length())),
                    ("Next Header".to_string(), format!("{} ({})", ipv6.get_next_header().0, ipv6.get_next_header())),
                    ("Hop Limit".to_string(), ipv6.get_hop_limit().to_string()),
                    ("Source".to_string(), ipv6.get_source().to_string()),
                    ("Destination".to_string(), ipv6.get_destination().to_string()),
                ],
            };
            layers.push(ip_layer);

            // Parse Transport layer (L4) for IPv6
            match ipv6.get_next_header() {
                IpNextHeaderProtocols::Tcp => {
                    if let Some(tcp) = TcpPacket::new(ipv6.payload()) {
                        let tcp_layer = ProtocolLayer {
                            name: "Transmission Control Protocol".to_string(),
                            fields: vec![
                                ("Source Port".to_string(), tcp.get_source().to_string()),
                                ("Destination Port".to_string(), tcp.get_destination().to_string()),
                                ("Sequence Number".to_string(), tcp.get_sequence().to_string()),
                                ("Acknowledgment Number".to_string(), tcp.get_acknowledgement().to_string()),
                                ("Data Offset".to_string(), format!("{} bytes", (tcp.get_data_offset() as u32) * 4)),
                                ("Flags".to_string(), format!("0x{:02x}", tcp.get_flags())),
                                ("Window Size".to_string(), tcp.get_window().to_string()),
                            ],
                        };
                        layers.push(tcp_layer);

                        let payload = tcp.payload();
                        if !payload.is_empty() {
                            parse_application_layer(&mut layers, tcp.get_source(), tcp.get_destination(), payload, true);
                        }
                    }
                }
                IpNextHeaderProtocols::Udp => {
                    if let Some(udp) = UdpPacket::new(ipv6.payload()) {
                        let udp_layer = ProtocolLayer {
                            name: "User Datagram Protocol".to_string(),
                            fields: vec![
                                ("Source Port".to_string(), udp.get_source().to_string()),
                                ("Destination Port".to_string(), udp.get_destination().to_string()),
                                ("Length".to_string(), format!("{} bytes", udp.get_length())),
                                ("Checksum".to_string(), format!("0x{:04x}", udp.get_checksum())),
                            ],
                        };
                        layers.push(udp_layer);

                        let payload = udp.payload();
                        if !payload.is_empty() {
                            parse_application_layer(&mut layers, udp.get_source(), udp.get_destination(), payload, false);
                        }
                    }
                }
                _ => {}
            }
        }
        EtherTypes::Arp => {
            let arp_layer = ProtocolLayer {
                name: "Address Resolution Protocol".to_string(),
                fields: vec![
                    ("Payload Length".to_string(), format!("{} bytes", ethernet.payload().len())),
                ],
            };
            layers.push(arp_layer);
        }
        _ => {}
    }

    // Get summary - use current time as fallback since we don't have timestamp here
    // This is only used for detail view, so approximate timestamp is acceptable
    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64;
    let summary = parse_summary(raw_data, id, timestamp_ns)?;

    Some(PacketDetail {
        summary,
        layers,
        raw_bytes: raw_data.to_vec(),
    })
}

// Application layer parsing (F6)
fn parse_application_layer(
    layers: &mut Vec<ProtocolLayer>,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
    is_tcp: bool,
) {
    // Skip if payload is empty
    if payload.is_empty() {
        return;
    }
    
    // DNS (port 53)
    if src_port == 53 || dst_port == 53 {
        let dns_layer = ProtocolLayer {
            name: "Domain Name System".to_string(),
            fields: vec![
                ("Port".to_string(), if src_port == 53 { "53 (Response)".to_string() } else { "53 (Query)".to_string() }),
                ("Payload Length".to_string(), format!("{} bytes", payload.len())),
            ],
        };
        layers.push(dns_layer);
        return;
    }

    // HTTP (port 80) or HTTPS (port 443)
    if is_tcp && (src_port == 80 || dst_port == 80 || src_port == 443 || dst_port == 443) {
        // Try to parse HTTP methods from payload
        if let Ok(payload_str) = std::str::from_utf8(&payload[..payload.len().min(100)]) {
            let mut http_layer = ProtocolLayer {
                name: if src_port == 443 || dst_port == 443 {
                    "Hypertext Transfer Protocol Secure".to_string()
                } else {
                    "Hypertext Transfer Protocol".to_string()
                },
                fields: vec![],
            };

            if payload_str.starts_with("GET") {
                http_layer.fields.push(("Method".to_string(), "GET".to_string()));
            } else if payload_str.starts_with("POST") {
                http_layer.fields.push(("Method".to_string(), "POST".to_string()));
            } else if payload_str.starts_with("PUT") {
                http_layer.fields.push(("Method".to_string(), "PUT".to_string()));
            } else if payload_str.starts_with("DELETE") {
                http_layer.fields.push(("Method".to_string(), "DELETE".to_string()));
            } else if payload_str.starts_with("HTTP/") {
                http_layer.fields.push(("Type".to_string(), "Response".to_string()));
            }

            if !http_layer.fields.is_empty() {
                http_layer.fields.push(("Payload Length".to_string(), format!("{} bytes", payload.len())));
                layers.push(http_layer);
                return;
            }
        }
    }

    // Generic application layer
    let app_layer = ProtocolLayer {
        name: "Application Data".to_string(),
        fields: vec![
            ("Payload Length".to_string(), format!("{} bytes", payload.len())),
        ],
    };
    layers.push(app_layer);
}
