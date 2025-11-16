#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::PacketSummary;

    fn create_test_packet(id: u64, protocol: &str, src: &str, dst: &str, info: &str) -> PacketSummary {
        PacketSummary {
            id,
            timestamp: 1_000_000_000,
            source_addr: src.to_string(),
            dest_addr: dst.to_string(),
            protocol: protocol.to_string(),
            length: 100,
            info: info.to_string(),
        }
    }

    #[test]
    fn test_matches_filter_protocol() {
        let packet = create_test_packet(1, "TCP", "192.168.1.1", "192.168.1.2", "test");

        assert!(matches_filter(&packet, "protocol:tcp"));
        assert!(matches_filter(&packet, "protocol:TCP"));
        assert!(!matches_filter(&packet, "protocol:udp"));
        assert!(!matches_filter(&packet, "protocol:")); // empty protocol filter
    }

    #[test]
    fn test_matches_filter_ip() {
        let packet = create_test_packet(1, "TCP", "192.168.1.1", "192.168.1.2", "test");

        assert!(matches_filter(&packet, "ip:192.168.1.1"));
        assert!(matches_filter(&packet, "ip:192.168.1.2"));
        assert!(!matches_filter(&packet, "ip:10.0.0.1"));
        assert!(!matches_filter(&packet, "ip:")); // empty ip filter
    }

    #[test]
    fn test_matches_filter_port() {
        let packet = create_test_packet(1, "TCP", "192.168.1.1", "192.168.1.2", "80");

        assert!(matches_filter(&packet, "port:80"));
        assert!(!matches_filter(&packet, "port:443"));
        assert!(!matches_filter(&packet, "port:")); // empty port filter
    }

    #[test]
    fn test_matches_filter_src_dst() {
        let packet = create_test_packet(1, "TCP", "192.168.1.1", "192.168.1.2", "test");

        assert!(matches_filter(&packet, "src:192.168.1.1"));
        assert!(!matches_filter(&packet, "src:10.0.0.1"));
        assert!(!matches_filter(&packet, "src:")); // empty src filter

        assert!(matches_filter(&packet, "dst:192.168.1.2"));
        assert!(!matches_filter(&packet, "dst:10.0.0.1"));
        assert!(!matches_filter(&packet, "dst:")); // empty dst filter
    }

    #[test]
    fn test_matches_filter_general_search() {
        let packet = create_test_packet(1, "TCP", "192.168.1.1", "192.168.1.2", "HTTP GET");

        assert!(matches_filter(&packet, "tcp"));
        assert!(matches_filter(&packet, "192.168"));
        assert!(matches_filter(&packet, "HTTP"));
        assert!(matches_filter(&packet, "GET"));
        assert!(!matches_filter(&packet, "UDP"));
        assert!(!matches_filter(&packet, "10.0.0.1"));
    }

    #[test]
    fn test_matches_filter_empty_or_whitespace() {
        let packet = create_test_packet(1, "TCP", "192.168.1.1", "192.168.1.2", "test");

        assert!(matches_filter(&packet, ""));
        assert!(matches_filter(&packet, "   "));
        assert!(matches_filter(&packet, "\t"));
    }

    #[test]
    fn test_get_filtered_packets() {
        let packets = vec![
            create_test_packet(1, "TCP", "192.168.1.1", "192.168.1.2", "80"),
            create_test_packet(2, "UDP", "192.168.1.1", "192.168.1.3", "53"),
            create_test_packet(3, "TCP", "192.168.1.2", "192.168.1.1", "443"),
        ];

        // No filter - should return all packets
        let filtered = get_filtered_packets(&packets, "");
        assert_eq!(filtered.len(), 3);

        // Protocol filter
        let filtered = get_filtered_packets(&packets, "protocol:tcp");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, 1);
        assert_eq!(filtered[1].id, 3);

        // IP filter
        let filtered = get_filtered_packets(&packets, "ip:192.168.1.1");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, 1);
        assert_eq!(filtered[1].id, 2);

        // Port filter
        let filtered = get_filtered_packets(&packets, "port:80");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 1);

        // General search
        let filtered = get_filtered_packets(&packets, "TCP");
        assert_eq!(filtered.len(), 2);
    }
}