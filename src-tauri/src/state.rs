use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlowKey {
    pub src_ip: IpAddr,
    pub dst_ip: IpAddr,
    pub protocol: u8,
    pub src_port: u16,
    pub dst_port: u16,
}

impl FlowKey {
    pub fn new(src_ip: IpAddr, dst_ip: IpAddr, protocol: u8, src_port: u16, dst_port: u16) -> Self {
        // Canonicalize the flow key so (A, B) is same as (B, A) for bidirectional streams
        if (src_ip, src_port) < (dst_ip, dst_port) {
            FlowKey { src_ip, dst_ip, protocol, src_port, dst_port }
        } else {
            FlowKey { src_ip: dst_ip, dst_ip: src_ip, protocol, src_port: dst_port, dst_port: src_port }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub key: FlowKey,
    pub packet_ids: Vec<u64>,
    pub start_time_ns: i64,
    pub end_time_ns: i64,
    pub total_bytes: u64,
    pub packet_count: u64,
}

pub struct FlowTable {
    pub flows: HashMap<FlowKey, Flow>,
}

impl FlowTable {
    pub fn new() -> Self {
        FlowTable {
            flows: HashMap::new(),
        }
    }

    pub fn update(&mut self, packet_id: u64, timestamp_ns: i64, length: u32, key: FlowKey) {
        let flow = self.flows.entry(key.clone()).or_insert_with(|| Flow {
            key,
            packet_ids: Vec::new(),
            start_time_ns: timestamp_ns,
            end_time_ns: timestamp_ns,
            total_bytes: 0,
            packet_count: 0,
        });

        flow.packet_ids.push(packet_id);
        flow.end_time_ns = timestamp_ns;
        flow.total_bytes += length as u64;
        flow.packet_count += 1;
    }

    pub fn clear(&mut self) {
        self.flows.clear();
    }
}
