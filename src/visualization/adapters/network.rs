use super::util::{
    ext, live_source, read_source, selected_path, source_for_path, stable_id, workspace_files,
};
use crate::process_window::CommandWindowExt;
use crate::visualization::model::{
    VisualizationDiagnostic, VisualizationDocument, VisualizationEdge, VisualizationEvent,
    VisualizationFrame, VisualizationNode, VisualizationSource, VisualizationTypeDescriptor,
};
use crate::visualization::{type_descriptor, VisualizationAdapter, VisualizationContext};
use anyhow::Result;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::process::Command;

pub struct NetworkAdapter;

impl VisualizationAdapter for NetworkAdapter {
    fn descriptor(&self) -> VisualizationTypeDescriptor {
        type_descriptor(
            "network",
            "Network",
            "Packet captures, request traces, protocol logs, and distributed communication.",
            "atlas.network.artifacts",
        )
    }

    fn discover(&self, context: &VisualizationContext<'_>) -> Result<Vec<VisualizationSource>> {
        let mut sources = vec![live_source(
            "runtime:network",
            "network",
            "Active network connections",
            "runtime-connections",
        )];
        sources.extend(
            workspace_files(
                context.workspace_root,
                |path| {
                    let name = path
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or_default()
                        .to_ascii_lowercase();
                    matches!(ext(path).as_str(), "pcap" | "pcapng" | "har")
                        || (matches!(ext(path).as_str(), "log" | "json" | "jsonl" | "csv" | "txt")
                            && [
                                "network", "request", "trace", "access", "packet", "span", "http",
                                "rpc",
                            ]
                            .iter()
                            .any(|needle| name.contains(needle)))
                },
                100,
            )
            .iter()
            .map(|path| {
                source_for_path(
                    context.workspace_root,
                    path,
                    "network",
                    network_source_type(path),
                )
            })
            .collect::<Vec<_>>(),
        );
        Ok(sources)
    }

    fn parse(&self, context: &VisualizationContext<'_>) -> Result<VisualizationDocument> {
        if context.source_id == Some("runtime:network") {
            let source = live_source(
                "runtime:network",
                "network",
                "Active network connections",
                "runtime-connections",
            );
            let mut document =
                VisualizationDocument::empty("network", "Active Network Connections", source);
            parse_runtime_connections(&mut document)?;
            finalize_frames(&mut document);
            return Ok(document);
        }
        let path = selected_path(context.workspace_root, context.source_id)?;
        let source = source_for_path(
            context.workspace_root,
            &path,
            "network",
            network_source_type(&path),
        );
        let mut document = VisualizationDocument::empty(
            "network",
            path.file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("Network"),
            source,
        );
        match ext(&path).as_str() {
            "pcap" => parse_pcap(&path, &mut document)?,
            "pcapng" => parse_pcapng(&path, &mut document)?,
            "har" => parse_har(&read_source(&path)?, &mut document)?,
            _ => parse_trace_log(&read_source(&path)?, &mut document),
        }
        finalize_frames(&mut document);
        if document.nodes.is_empty() {
            document.diagnostics.push(VisualizationDiagnostic {
                level: "info".to_string(),
                message: "No endpoints or request relationships were found in this source."
                    .to_string(),
                metadata: BTreeMap::new(),
            });
        }
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_u16(buffer: &mut Vec<u8>, value: u16) {
        buffer.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(buffer: &mut Vec<u8>, value: u32) {
        buffer.extend_from_slice(&value.to_le_bytes());
    }

    fn push_block(buffer: &mut Vec<u8>, block_type: u32, mut body: Vec<u8>) {
        while body.len() % 4 != 0 {
            body.push(0);
        }
        let length = (body.len() + 12) as u32;
        push_u32(buffer, block_type);
        push_u32(buffer, length);
        buffer.extend_from_slice(&body);
        push_u32(buffer, length);
    }

    fn ethernet_ipv4_tcp_packet() -> Vec<u8> {
        let mut packet = vec![0u8; 14 + 20 + 20];
        packet[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        let ip = &mut packet[14..34];
        ip[0] = 0x45;
        ip[2..4].copy_from_slice(&40u16.to_be_bytes());
        ip[8] = 64;
        ip[9] = 6;
        ip[12..16].copy_from_slice(&[10, 0, 0, 1]);
        ip[16..20].copy_from_slice(&[10, 0, 0, 2]);
        let tcp = &mut packet[34..];
        tcp[0..2].copy_from_slice(&12345u16.to_be_bytes());
        tcp[2..4].copy_from_slice(&443u16.to_be_bytes());
        packet
    }

    #[test]
    fn pcapng_parser_decodes_real_packet_blocks() {
        let mut bytes = Vec::new();
        let mut section = Vec::new();
        section.extend_from_slice(&[0x4d, 0x3c, 0x2b, 0x1a]);
        push_u16(&mut section, 1);
        push_u16(&mut section, 0);
        section.extend_from_slice(&u64::MAX.to_le_bytes());
        push_block(&mut bytes, 0x0a0d0d0a, section);

        let mut interface = Vec::new();
        push_u16(&mut interface, 1);
        push_u16(&mut interface, 0);
        push_u32(&mut interface, 65_535);
        push_block(&mut bytes, 1, interface);

        let packet = ethernet_ipv4_tcp_packet();
        let mut enhanced = Vec::new();
        push_u32(&mut enhanced, 0);
        push_u32(&mut enhanced, 0);
        push_u32(&mut enhanced, 1_000_000);
        push_u32(&mut enhanced, packet.len() as u32);
        push_u32(&mut enhanced, packet.len() as u32);
        enhanced.extend_from_slice(&packet);
        push_block(&mut bytes, 6, enhanced);

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("capture.pcapng");
        std::fs::write(&path, bytes).unwrap();
        let source = source_for_path(temp.path(), &path, "network", "packet-capture");
        let mut document = VisualizationDocument::empty("network", "capture", source);
        parse_pcapng(&path, &mut document).unwrap();
        finalize_frames(&mut document);

        assert_eq!(document.events.len(), 1);
        assert_eq!(document.frames.len(), 1);
        assert!(document
            .nodes
            .iter()
            .any(|node| node.label == "10.0.0.1:12345"));
        assert!(document
            .nodes
            .iter()
            .any(|node| node.label == "10.0.0.2:443"));
        assert_eq!(document.edges[0].label, "TCP");
        assert_eq!(document.metadata["decoded_packets"], 1);
    }
}

fn parse_runtime_connections(document: &mut VisualizationDocument) -> Result<()> {
    #[cfg(windows)]
    let output = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .hide_window()
        .output()?;
    #[cfg(not(windows))]
    let output = Command::new("sh")
        .args(["-c", "ss -tan 2>/dev/null || netstat -tan 2>/dev/null"])
        .hide_window()
        .output()?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut sequence = 0usize;
    for line in raw.lines() {
        let columns = line.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 4 || !columns[0].eq_ignore_ascii_case("tcp") {
            continue;
        }
        let source = columns[1];
        let target = columns[2];
        if target.ends_with(":0") || target == "0.0.0.0:*" || target == ":::*" {
            continue;
        }
        let status = columns[3];
        let mut metadata = BTreeMap::new();
        metadata.insert("state".to_string(), Value::String(status.to_string()));
        if let Some(pid) = columns.get(4).and_then(|value| value.parse::<u64>().ok()) {
            metadata.insert("pid".to_string(), Value::from(pid));
        }
        add_exchange(
            document,
            sequence,
            source,
            target,
            "TCP",
            status,
            Some(chrono::Utc::now().to_rfc3339()),
            metadata,
        );
        sequence += 1;
        if sequence >= 2_000 {
            break;
        }
    }
    document
        .metadata
        .insert("connection_count".to_string(), Value::from(sequence));
    Ok(())
}

fn network_source_type(path: &Path) -> &'static str {
    match ext(path).as_str() {
        "pcap" | "pcapng" => "packet-capture",
        "har" => "http-archive",
        _ => "network-log",
    }
}

fn ensure_node(document: &mut VisualizationDocument, id: &str, label: &str, category: &str) {
    if document.nodes.iter().any(|node| node.id == id) {
        return;
    }
    document
        .nodes
        .push(VisualizationNode::new(id, label, category));
}

fn add_exchange(
    document: &mut VisualizationDocument,
    sequence: usize,
    source: &str,
    target: &str,
    protocol: &str,
    status: &str,
    timestamp: Option<String>,
    metadata: BTreeMap<String, Value>,
) {
    let source_id = stable_id("endpoint", source);
    let target_id = stable_id("endpoint", target);
    ensure_node(document, &source_id, source, "endpoint");
    ensure_node(document, &target_id, target, "endpoint");
    let edge_id = format!("exchange:{sequence}");
    let mut edge = VisualizationEdge::new(&edge_id, &source_id, &target_id, protocol, "request");
    edge.status = status.to_string();
    edge.metadata = metadata.clone();
    document.edges.push(edge);
    document.events.push(VisualizationEvent {
        id: format!("network-event:{sequence}"),
        sequence,
        label: protocol.to_string(),
        category: "request".to_string(),
        status: status.to_string(),
        timestamp,
        source_id: Some(source_id),
        target_id: Some(target_id),
        metadata,
    });
}

fn parse_har(raw: &str, document: &mut VisualizationDocument) -> Result<()> {
    let value: Value = serde_json::from_str(raw)?;
    let entries = value
        .pointer("/log/entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (index, entry) in entries.iter().take(5000).enumerate() {
        let url = entry
            .pointer("/request/url")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let method = entry
            .pointer("/request/method")
            .and_then(Value::as_str)
            .unwrap_or("HTTP");
        let target = url
            .split("//")
            .nth(1)
            .unwrap_or(url)
            .split('/')
            .next()
            .unwrap_or(url);
        let source = entry
            .get("serverIPAddress")
            .and_then(Value::as_str)
            .unwrap_or("client");
        let status_code = entry
            .pointer("/response/status")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let mut metadata = BTreeMap::new();
        metadata.insert("url".to_string(), Value::String(url.to_string()));
        metadata.insert(
            "duration_ms".to_string(),
            entry.get("time").cloned().unwrap_or(Value::Null),
        );
        metadata.insert("status_code".to_string(), Value::from(status_code));
        add_exchange(
            document,
            index,
            source,
            target,
            method,
            &status_code.to_string(),
            entry
                .get("startedDateTime")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            metadata,
        );
    }
    Ok(())
}

fn parse_trace_log(raw: &str, document: &mut VisualizationDocument) {
    let endpoint_regex = Regex::new(r"(?i)\b(?:https?://)?(?:[a-z0-9-]+\.)+[a-z]{2,}(?::\d+)?\b|\b(?:\d{1,3}\.){3}\d{1,3}(?::\d+)?\b").unwrap();
    let arrow_regex = Regex::new(r"([^\s]+)\s*(?:->|=>|鈫?\s*([^\s]+)").unwrap();
    for (index, line) in raw.lines().take(10_000).enumerate() {
        let value = serde_json::from_str::<Value>(line).ok();
        let object = value.as_ref().and_then(Value::as_object);
        let source = object
            .and_then(|value| first_string(value, &["source", "src", "client", "from", "peer"]));
        let target = object.and_then(|value| {
            first_string(value, &["target", "dst", "server", "to", "host", "service"])
        });
        let (source, target) = match (source, target) {
            (Some(source), Some(target)) => (source, target),
            _ => {
                if let Some(capture) = arrow_regex.captures(line) {
                    (
                        capture.get(1).unwrap().as_str().to_string(),
                        capture.get(2).unwrap().as_str().to_string(),
                    )
                } else {
                    let endpoints = endpoint_regex
                        .find_iter(line)
                        .map(|value| value.as_str().to_string())
                        .collect::<Vec<_>>();
                    if endpoints.len() < 2 {
                        continue;
                    }
                    (endpoints[0].clone(), endpoints[1].clone())
                }
            }
        };
        let protocol = object
            .and_then(|value| {
                first_string(value, &["protocol", "method", "rpc", "operation", "name"])
            })
            .unwrap_or_else(|| infer_protocol(line).to_string());
        let status = object
            .and_then(|value| first_string(value, &["status", "status_code", "code"]))
            .unwrap_or_default();
        let timestamp = object
            .and_then(|value| first_string(value, &["timestamp", "time", "ts", "start_time"]));
        let metadata = object
            .map(|value| {
                value
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_else(|| {
                BTreeMap::from([(
                    "raw".to_string(),
                    Value::String(line.chars().take(600).collect()),
                )])
            });
        add_exchange(
            document, index, &source, &target, &protocol, &status, timestamp, metadata,
        );
    }
}

fn first_string(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        object.get(*key).and_then(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .or_else(|| value.as_i64().map(|value| value.to_string()))
        })
    })
}

fn infer_protocol(line: &str) -> &'static str {
    let line = line.to_ascii_lowercase();
    if line.contains("grpc") {
        "gRPC"
    } else if line.contains("http") {
        "HTTP"
    } else if line.contains("tcp") {
        "TCP"
    } else if line.contains("udp") {
        "UDP"
    } else if line.contains("dns") {
        "DNS"
    } else {
        "request"
    }
}

fn parse_pcap(path: &Path, document: &mut VisualizationDocument) -> Result<()> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 24 {
        anyhow::bail!("PCAP header is truncated");
    }
    let magic = &bytes[..4];
    let little = matches!(magic, [0xd4, 0xc3, 0xb2, 0xa1] | [0x4d, 0x3c, 0xb2, 0xa1]);
    if !little && !matches!(magic, [0xa1, 0xb2, 0xc3, 0xd4] | [0xa1, 0xb2, 0x3c, 0x4d]) {
        anyhow::bail!("unsupported PCAP magic");
    }
    let read_u32 = |slice: &[u8]| {
        if little {
            u32::from_le_bytes(slice.try_into().unwrap())
        } else {
            u32::from_be_bytes(slice.try_into().unwrap())
        }
    };
    let link_type = if little {
        u32::from_le_bytes(bytes[20..24].try_into().unwrap())
    } else {
        u32::from_be_bytes(bytes[20..24].try_into().unwrap())
    };
    let mut offset = 24usize;
    let mut sequence = 0usize;
    while offset + 16 <= bytes.len() && sequence < 5000 {
        let ts_sec = read_u32(&bytes[offset..offset + 4]);
        let ts_sub = read_u32(&bytes[offset + 4..offset + 8]);
        let included = read_u32(&bytes[offset + 8..offset + 12]) as usize;
        offset += 16;
        if included == 0 || offset + included > bytes.len() {
            break;
        }
        let packet = &bytes[offset..offset + included];
        offset += included;
        let Some(decoded) = decode_packet_endpoints(packet, link_type) else {
            continue;
        };
        let metadata = BTreeMap::from([
            ("captured_bytes".to_string(), Value::from(included)),
            ("link_type".to_string(), Value::from(link_type)),
            ("timestamp_subsecond".to_string(), Value::from(ts_sub)),
        ]);
        add_exchange(
            document,
            sequence,
            &decoded.source,
            &decoded.target,
            &decoded.protocol,
            "",
            Some(
                chrono::DateTime::from_timestamp(ts_sec as i64, 0)
                    .map(|value| value.to_rfc3339())
                    .unwrap_or_default(),
            ),
            metadata,
        );
        sequence += 1;
    }
    document
        .metadata
        .insert("decoded_packets".to_string(), json!(sequence));
    Ok(())
}

#[derive(Debug, Clone)]
struct DecodedPacket {
    source: String,
    target: String,
    protocol: String,
}

#[derive(Debug, Clone, Copy)]
struct PcapngInterface {
    link_type: u32,
    timestamp_resolution: f64,
}

fn parse_pcapng(path: &Path, document: &mut VisualizationDocument) -> Result<()> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 12 || bytes[..4] != [0x0a, 0x0d, 0x0d, 0x0a] {
        anyhow::bail!("PCAPNG section header is missing");
    }
    let mut offset = 0usize;
    let mut little = true;
    let mut interfaces = Vec::<PcapngInterface>::new();
    let mut sequence = 0usize;
    while offset + 12 <= bytes.len() && sequence < 5_000 {
        let block_type_bytes = &bytes[offset..offset + 4];
        let is_section = block_type_bytes == [0x0a, 0x0d, 0x0d, 0x0a];
        if is_section {
            if offset + 12 > bytes.len() {
                break;
            }
            little = match &bytes[offset + 8..offset + 12] {
                [0x4d, 0x3c, 0x2b, 0x1a] => true,
                [0x1a, 0x2b, 0x3c, 0x4d] => false,
                _ => anyhow::bail!("PCAPNG byte-order magic is invalid"),
            };
            interfaces.clear();
        }
        let read_u16 = |slice: &[u8]| {
            if little {
                u16::from_le_bytes(slice.try_into().unwrap())
            } else {
                u16::from_be_bytes(slice.try_into().unwrap())
            }
        };
        let read_u32 = |slice: &[u8]| {
            if little {
                u32::from_le_bytes(slice.try_into().unwrap())
            } else {
                u32::from_be_bytes(slice.try_into().unwrap())
            }
        };
        let block_type = read_u32(block_type_bytes);
        let total_length = read_u32(&bytes[offset + 4..offset + 8]) as usize;
        if total_length < 12 || total_length % 4 != 0 || offset + total_length > bytes.len() {
            anyhow::bail!("PCAPNG block length is invalid at byte {offset}");
        }
        let trailing_length = read_u32(&bytes[offset + total_length - 4..offset + total_length]);
        if trailing_length as usize != total_length {
            anyhow::bail!("PCAPNG block length footer does not match at byte {offset}");
        }
        let body = &bytes[offset + 8..offset + total_length - 4];
        match block_type {
            0x0a0d0d0a => {}
            1 if body.len() >= 8 => {
                let link_type = read_u16(&body[..2]) as u32;
                let timestamp_resolution = pcapng_timestamp_resolution(&body[8..], little);
                interfaces.push(PcapngInterface {
                    link_type,
                    timestamp_resolution,
                });
            }
            6 if body.len() >= 20 => {
                let interface_id = read_u32(&body[..4]) as usize;
                let timestamp_high = read_u32(&body[4..8]) as u64;
                let timestamp_low = read_u32(&body[8..12]) as u64;
                let captured_len = read_u32(&body[12..16]) as usize;
                if 20 + captured_len > body.len() {
                    anyhow::bail!("PCAPNG enhanced packet is truncated");
                }
                let Some(interface) = interfaces.get(interface_id).copied() else {
                    offset += total_length;
                    continue;
                };
                let packet = &body[20..20 + captured_len];
                let timestamp = (timestamp_high << 32) | timestamp_low;
                if let Some(decoded) = decode_packet_endpoints(packet, interface.link_type) {
                    let timestamp_seconds = timestamp as f64 * interface.timestamp_resolution;
                    add_captured_packet(
                        document,
                        sequence,
                        &decoded,
                        captured_len,
                        interface.link_type,
                        timestamp_seconds,
                    );
                    sequence += 1;
                }
            }
            3 if body.len() >= 4 => {
                let captured_len = body.len().saturating_sub(4);
                let link_type = interfaces.first().map(|value| value.link_type).unwrap_or(1);
                if let Some(decoded) = decode_packet_endpoints(&body[4..], link_type) {
                    add_captured_packet(document, sequence, &decoded, captured_len, link_type, 0.0);
                    sequence += 1;
                }
            }
            _ => {}
        }
        offset += total_length;
    }
    document
        .metadata
        .insert("decoded_packets".to_string(), Value::from(sequence));
    document
        .metadata
        .insert("interfaces".to_string(), Value::from(interfaces.len()));
    Ok(())
}

fn pcapng_timestamp_resolution(options: &[u8], little: bool) -> f64 {
    let read_u16 = |slice: &[u8]| {
        if little {
            u16::from_le_bytes(slice.try_into().unwrap())
        } else {
            u16::from_be_bytes(slice.try_into().unwrap())
        }
    };
    let mut offset = 0usize;
    while offset + 4 <= options.len() {
        let code = read_u16(&options[offset..offset + 2]);
        let length = read_u16(&options[offset + 2..offset + 4]) as usize;
        offset += 4;
        if code == 0 || offset + length > options.len() {
            break;
        }
        if code == 9 && length >= 1 {
            let exponent = options[offset];
            return if exponent & 0x80 == 0 {
                10f64.powi(-(exponent as i32))
            } else {
                2f64.powi(-((exponent & 0x7f) as i32))
            };
        }
        offset += (length + 3) & !3;
    }
    1e-6
}

fn add_captured_packet(
    document: &mut VisualizationDocument,
    sequence: usize,
    decoded: &DecodedPacket,
    captured_len: usize,
    link_type: u32,
    timestamp_seconds: f64,
) {
    let timestamp = if timestamp_seconds > 0.0 {
        chrono::DateTime::from_timestamp_micros((timestamp_seconds * 1_000_000.0) as i64)
            .map(|value| value.to_rfc3339())
    } else {
        None
    };
    add_exchange(
        document,
        sequence,
        &decoded.source,
        &decoded.target,
        &decoded.protocol,
        "",
        timestamp,
        BTreeMap::from([
            ("captured_bytes".to_string(), Value::from(captured_len)),
            ("link_type".to_string(), Value::from(link_type)),
        ]),
    );
}

fn decode_packet_endpoints(packet: &[u8], link_type: u32) -> Option<DecodedPacket> {
    let network = match link_type {
        1 if packet.len() >= 14 => {
            let ether_type = u16::from_be_bytes([packet[12], packet[13]]);
            let mut offset = 14usize;
            let ether_type = if matches!(ether_type, 0x8100 | 0x88a8) && packet.len() >= 18 {
                offset = 18;
                u16::from_be_bytes([packet[16], packet[17]])
            } else {
                ether_type
            };
            match ether_type {
                0x0800 | 0x86dd => &packet[offset..],
                _ => return None,
            }
        }
        101 | 228 | 229 => packet,
        113 if packet.len() >= 16 => &packet[16..],
        276 if packet.len() >= 20 => &packet[20..],
        _ => return None,
    };
    match network.first()? >> 4 {
        4 => decode_ipv4(network),
        6 => decode_ipv6(network),
        _ => None,
    }
}

fn decode_ipv4(ip: &[u8]) -> Option<DecodedPacket> {
    if ip.len() < 20 {
        return None;
    }
    let ihl = ((ip[0] & 0x0f) as usize) * 4;
    if ihl < 20 || ip.len() < ihl {
        return None;
    }
    let source = Ipv4Addr::new(ip[12], ip[13], ip[14], ip[15]);
    let target = Ipv4Addr::new(ip[16], ip[17], ip[18], ip[19]);
    decoded_transport(
        source.to_string(),
        target.to_string(),
        ip[9],
        &ip[ihl..],
        "IPv4",
    )
}

fn decode_ipv6(ip: &[u8]) -> Option<DecodedPacket> {
    if ip.len() < 40 {
        return None;
    }
    let source = Ipv6Addr::from(<[u8; 16]>::try_from(&ip[8..24]).ok()?);
    let target = Ipv6Addr::from(<[u8; 16]>::try_from(&ip[24..40]).ok()?);
    decoded_transport(
        source.to_string(),
        target.to_string(),
        ip[6],
        &ip[40..],
        "IPv6",
    )
}

fn decoded_transport(
    source: String,
    target: String,
    protocol_number: u8,
    transport: &[u8],
    fallback: &str,
) -> Option<DecodedPacket> {
    let (protocol, source, target) = if matches!(protocol_number, 6 | 17) && transport.len() >= 4 {
        let source_port = u16::from_be_bytes([transport[0], transport[1]]);
        let target_port = u16::from_be_bytes([transport[2], transport[3]]);
        (
            if protocol_number == 6 { "TCP" } else { "UDP" },
            format!("{source}:{source_port}"),
            format!("{target}:{target_port}"),
        )
    } else {
        (fallback, source, target)
    };
    Some(DecodedPacket {
        source,
        target,
        protocol: protocol.to_string(),
    })
}

fn finalize_frames(document: &mut VisualizationDocument) {
    document.frames = document
        .events
        .iter()
        .map(|event| VisualizationFrame {
            id: format!("network-frame:{}", event.sequence),
            sequence: event.sequence,
            label: event.label.clone(),
            active_nodes: [event.source_id.clone(), event.target_id.clone()]
                .into_iter()
                .flatten()
                .collect(),
            active_edges: vec![format!("exchange:{}", event.sequence)],
            metrics: BTreeMap::new(),
        })
        .collect();
    let mut counts = HashMap::<(String, String, String), usize>::new();
    for edge in &document.edges {
        *counts
            .entry((edge.source.clone(), edge.target.clone(), edge.label.clone()))
            .or_default() += 1;
    }
    for edge in &mut document.edges {
        edge.weight = *counts
            .get(&(edge.source.clone(), edge.target.clone(), edge.label.clone()))
            .unwrap_or(&1) as f64;
    }
}
