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
///
/// We intentionally accept a very relaxed shape because upstream logs can be
/// emitted with or without `ThreadId`, source file spans, or padding between
/// the timestamp and level. The only hard requirements are:
/// - RFC3339 timestamp
/// - UPPERCASE level word
/// - a single token for the target (may contain `:`) followed by the rest of
///   the message
///
/// Examples that must match:
/// 2026-02-04T12:33:07.043484Z DEBUG engine::tree: received new engine message
/// 2026-02-04T12:33:07.060579Z  INFO reth_node_events::node: Received block...
///
/// The target token is captured as everything up to the first whitespace after
/// the level, so it may include trailing colons (e.g. `engine::tree:`). We
/// strip the trailing colon before storing it to keep categorisation stable.
const LINE_RE: &str = r"^(?P<ts>[^ ]+)\s+(?P<level>[A-Z]+)\s+(?P<target>\S+)\s+(?P<msg>.*)$";

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
    // Prefer strict parsing first for determinism.
    if let Some(caps) = LOG_REGEX.captures(line) {
        let ts = caps.name("ts")?.as_str();
        let level = caps.name("level")?.as_str().to_string();
        let mut target = caps.name("target")?.as_str().to_string();
        // Normalise target: drop trailing ':' so category detection is consistent.
        if target.ends_with(':') {
            target.pop();
        }
        let file = String::from("unknown:0");
        let message = caps.name("msg")?.as_str().to_string();
        let timestamp_ns = parse_timestamp_ns(ts)?;

        return Some(RawLine {
            node_id,
            line_number,
            timestamp_ns,
            level,
            target,
            file,
            message,
        });
    }

    // Lenient fallback: accept any line whose first token parses as RFC3339.
    // This allows us to classify messages even when upstream logging format
    // changes (e.g., missing thread id, different file spans).
    let mut parts = line.splitn(3, ' ');
    let ts = parts.next()?;
    let level = parts.next().unwrap_or("UNKNOWN").to_string();
    let rest = parts.next().unwrap_or("");

    let timestamp_ns = parse_timestamp_ns(ts)?;
    // Heuristic: treat the first word in `rest` as target if it contains '::',
    // otherwise leave target unknown and keep the full remainder as message.
    let mut rest_parts = rest.splitn(2, ' ');
    let mut target = rest_parts.next().unwrap_or("").to_string();
    let message_tail = rest_parts.next().unwrap_or("");
    if !target.contains("::") {
        // Reattach to message if this was not a target-like token.
        target = "unknown".to_string();
    } else if target.ends_with(':') {
        target.pop();
    }

    let message = if target == "unknown" {
        rest.trim_start().to_string()
    } else {
        message_tail.trim_start().to_string()
    };

    Some(RawLine {
        node_id,
        line_number,
        timestamp_ns,
        level,
        target,
        file: "unknown:0".to_string(),
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

    fn sample_relaxed_line(msg: &str) -> String {
        // Matches the real logs we get from nodes (no ThreadId/file span, extra spaces before INFO)
        format!(
            "2026-02-04T12:33:07.043484Z DEBUG engine::tree: {}",
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
    fn test_parse_line_relaxed_format() {
        let raw = parse_line(0, 42, &sample_relaxed_line("received new engine message"))
            .expect("relaxed parse");
        assert_eq!(raw.level, "DEBUG");
        assert_eq!(raw.target, "engine::tree"); // trailing ':' stripped
        assert_eq!(raw.line_number, 42);
        assert_eq!(raw.message, "received new engine message");
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
