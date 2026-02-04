// ============================================================================
// Log Parser - Parse log files and extract structured events
// ============================================================================

use chrono::DateTime;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::LazyLock;
use tracing::error;

use crate::events::{EventCategory, EventRegistry, derive_message_id};
use crate::state::types::{Event, RawLine};
use super::extractors;

/// Log line regex pattern
const LINE_RE: &str = r"^(?P<ts>[^ ]+)\s+(?P<level>[A-Z]+)\s+ThreadId\([^)]+\)\s+(?P<target>.+?):\s+(?P<file>.+?:\d+):\s+(?P<msg>.*)$";

static LOG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(LINE_RE).unwrap()
});

/// Parse a timestamp string to nanoseconds
pub fn parse_timestamp_ns(ts: &str) -> Option<i128> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .and_then(|dt| dt.timestamp_nanos_opt())
        .map(|n| n as i128)
}

/// Parse a single log line into a RawLine struct
pub fn parse_line(node_id: u8, line_number: u64, line: &str) -> Option<RawLine> {
    let caps = LOG_REGEX.captures(line)?;
    let ts = caps.name("ts")?.as_str();
    let level = caps.name("level")?.as_str().to_string();
    let target = caps.name("target")?.as_str().to_string();
    let file = caps.name("file")?.as_str().to_string();
    let message = caps.name("msg")?.as_str().to_string();
    let timestamp_ns = parse_timestamp_ns(ts)?;

    Some(RawLine {
        node_id,
        line_number,
        timestamp_ns,
        level,
        target,
        file,
        message,
    })
}

/// Classify a raw log line into a structured event
pub fn classify_event(raw: RawLine, registry: &EventRegistry, view_hint: Option<u64>) -> Event {
    let msg = raw.message.as_str();
    let target = raw.target.as_str();
    let extracted_view = extractors::extract_view(msg);
    let view = extracted_view.or(view_hint);
    let category = EventCategory::from_target(target);
    
    // Use the registry to classify the event
    let (kind, payload) = registry.classify(msg);
    
    let message_id = derive_message_id(kind, view, payload.as_deref());
    let leader = extractors::extract_leader(msg);
    let peer = extractors::extract_peer(msg);
    let actor = detect_actor_from_target(target);

    let kind_tag = kind.as_tag();
    Event {
        id: 0,
        raw,
        view,
        payload,
        kind,
        kind_tag,
        category,
        leader,
        message_id,
        peer,
        actor,
    }
}

/// Detect actor from target path
fn detect_actor_from_target(target: &str) -> Option<String> {
    if target.contains("voter") {
        Some("voter".to_string())
    } else if target.contains("marshal") {
        Some("marshal".to_string())
    } else if target.contains("batcher") {
        Some("batcher".to_string())
    } else if target.contains("resolver") {
        Some("resolver".to_string())
    } else if target.contains("buffered") || target.contains("broadcast") {
        Some("broadcast".to_string())
    } else if target.contains("p2p") || target.contains("discovery") {
        Some("network".to_string())
    } else if target.contains("storage") || target.contains("journal") || target.contains("archive") {
        Some("storage".to_string())
    } else {
        None
    }
}

/// Read and parse a log file
pub fn read_log_file(path: &Path, node_id: u8, registry: &EventRegistry) -> Vec<Event> {
    let Ok(file) = File::open(path) else {
        error!("failed to open {:?}", path);
        return Vec::new();
    };
    
    let mut events = Vec::new();
    let reader = BufReader::new(file);
    let mut last_view: Option<u64> = None;
    
    for (idx, line) in reader.lines().enumerate() {
        if let Ok(l) = line {
            if let Some(raw) = parse_line(node_id, idx as u64, &l) {
                let ev = classify_event(raw, registry, last_view);
                if let Some(v) = ev.view {
                    last_view = Some(v);
                }
                events.push(ev);
            }
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventKind;
    
    fn sample_line(msg: &str) -> String {
        format!(
            "2024-01-15T10:00:00.123456789Z DEBUG ThreadId(01) commonware_consensus::simplex::actors::voter::actor: voter/actor.rs:100: {}",
            msg
        )
    }
    
    #[test]
    fn test_parse_line() {
        let raw = parse_line(0, 1, &sample_line("test message")).expect("parse");
        assert_eq!(raw.node_id, 0);
        assert_eq!(raw.line_number, 1);
        assert_eq!(raw.level, "DEBUG");
        assert!(raw.target.contains("voter"));
        assert_eq!(raw.message, "test message");
    }
    
    #[test]
    fn test_classify_event() {
        let registry = EventRegistry::new();
        let raw = parse_line(0, 1, &sample_line("leader elected round=Round { view: View(5) }")).expect("parse");
        let ev = classify_event(raw, &registry, None);
        assert_eq!(ev.kind, EventKind::LeaderElected);
        assert_eq!(ev.view, Some(5));
    }
}
