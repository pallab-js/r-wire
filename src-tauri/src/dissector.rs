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

                        // Application layer parsing
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

                        // Application layer parsing
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

    // Get summary
    let summary = parse_summary(raw_data, id, 0)?;

    Some(PacketDetail {
        summary,
        layers,
        raw_bytes: raw_data.to_vec(),
    })
}

// Application layer parsing
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
        assert!(summary.info.contains("192.168.1.1"));
        assert!(summary.info.contains("192.168.1.2"));
    }

    #[test]
    fn test_parse_summary_ipv4_udp() {
        // Create a mock IPv4 UDP packet
        let mut data = Vec::new();

        // Ethernet header
        data.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);
        data.extend_from_slice(&[0x08, 0x00]);

        // IPv4 header
        data.extend_from_slice(&[0x45, 0x00, 0x00, 0x20]); // total len = 32
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x40, 0x11, 0x00, 0x00]); // protocol UDP (17)
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]);
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]);

        // UDP header (8 bytes)
        data.extend_from_slice(&[0xC3, 0x50, 0x00, 0x35]); // src port 50000, dst port 53 (DNS)
        data.extend_from_slice(&[0x00, 0x0C, 0x00, 0x00]); // length 12, checksum 0

        let result = parse_summary(&data, 2, 2_000_000_000);
        assert!(result.is_some());

        let summary = result.unwrap();
        assert_eq!(summary.protocol, "UDP");
        assert!(summary.info.contains("192.168.1.1"));
        assert!(summary.info.contains("192.168.1.2"));
    }

    #[test]
    fn test_parse_summary_arp() {
        // Create a mock ARP packet
        let mut data = Vec::new();

        // Ethernet header
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]); // broadcast dst
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]); // src mac
        data.extend_from_slice(&[0x08, 0x06]); // ethertype ARP

        // ARP header (28 bytes minimum)
        data.extend_from_slice(&[0x00, 0x01, 0x08, 0x00]); // htype, ptype
        data.extend_from_slice(&[0x06, 0x04, 0x00, 0x01]); // hlen, plen, oper (request)
        data.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]); // sender mac
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x01]); // sender ip
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // target mac
        data.extend_from_slice(&[0xC0, 0xA8, 0x01, 0x02]); // target ip

        let result = parse_summary(&data, 3, 3_000_000_000);
        assert!(result.is_some());

        let summary = result.unwrap();
        assert_eq!(summary.protocol, "ARP");
        assert_eq!(summary.source_addr, "66:77:88:99:aa:bb");
        assert_eq!(summary.dest_addr, "ff:ff:ff:ff:ff:ff");
        assert_eq!(summary.info, "ARP");
    }

    #[test]
    fn test_parse_summary_invalid_packet() {
        // Test with insufficient data
        let data = vec![0x00, 0x01, 0x02];
        let result = parse_summary(&data, 1, 1_000_000_000);
        assert!(result.is_none());
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

        let result = dissect_packet(&data, 1);
        assert!(result.is_some());

        let detail = result.unwrap();
        assert!(detail.layers.len() >= 2); // At least Ethernet + IP layers
        assert_eq!(detail.layers[0].name, "Ethernet");
        assert!(detail.layers[1].name.contains("Internet Protocol"));
        assert!(detail.raw_bytes.len() > 0);
    }

    #[test]
    fn test_application_layer_parsing_dns() {
        let mut layers = Vec::new();
        let payload = b"\x00\x01\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00\x07example\x03com\x00\x00\x01\x00\x01";

        parse_application_layer(&mut layers, 53, 53, payload, false);

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].name, "Domain Name System");
        assert!(layers[0].fields.iter().any(|(k, v)| k == "Port" && v.contains("Response")));
    }

    #[test]
    fn test_application_layer_parsing_http() {
        let mut layers = Vec::new();
        let payload = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";

        parse_application_layer(&mut layers, 80, 80, payload, true);

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].name, "Hypertext Transfer Protocol");
        assert!(layers[0].fields.iter().any(|(k, v)| k == "Method" && v == "GET"));
    }
}