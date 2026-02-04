// ============================================================================
// Data Types - Core data structures
// ============================================================================

use serde::Serialize;
use crate::events::{EventCategory, EventKind};

/// Raw log line data
#[derive(Clone, Debug, Serialize)]
pub struct RawLine {
    pub node_id: u8,
    pub line_number: u64,
    pub timestamp_ns: i128,
    pub level: String,
    pub target: String,
    pub file: String,
    pub message: String,
}

/// Parsed and classified event
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub id: u64,
    pub raw: RawLine,
    pub view: Option<u64>,
    pub payload: Option<String>,
    pub kind: EventKind,
    pub kind_tag: u8,
    pub category: EventCategory,
    pub leader: Option<u8>,
    pub message_id: Option<String>,
    pub peer: Option<String>,
    pub actor: Option<String>,
}

/// Message edge between nodes
#[derive(Clone, Debug, Serialize)]
pub struct Edge {
    pub message_id: String,
    pub from_node: u8,
    pub to_node: u8,
    pub send_ts_ns: i128,
    pub recv_ts_ns: i128,
    pub latency_ms: f64,
    pub kind: EventKind,
}

/// View status for a node
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViewStatus {
    Entered,
    ProposalReceived,
    Notarized,
    Nullified,
    Finalized,
}

/// View state record
#[derive(Clone, Debug, Serialize)]
pub struct ViewRecord {
    pub node_id: u8,
    pub view: u64,
    pub status: ViewStatus,
    pub leader: Option<u8>,
    pub entered_ns: Option<i128>,
    pub proposal_ns: Option<i128>,
    pub notarized_ns: Option<i128>,
    pub nullified_ns: Option<i128>,
    pub finalized_ns: Option<i128>,
    pub latency_to_notarize_ms: Option<f64>,
    pub latency_to_finalize_ms: Option<f64>,
}

/// Time segment for navigation
#[derive(Clone, Debug, Serialize)]
pub struct TimeSegment {
    pub index: usize,
    pub start_ns: i128,
    pub end_ns: i128,
    pub event_count: usize,
    pub views: Vec<u64>,
}

/// Event metadata for UI
#[derive(Clone, Debug, Serialize)]
pub struct EventMeta {
    pub kind: EventKind,
    pub tag: u8,
    pub label: String,
    pub category: EventCategory,
    pub color: String,
}

/// Overall metadata
#[derive(Clone, Debug, Serialize, Default)]
pub struct Metadata {
    pub nodes: Vec<u8>,
    pub min_ts_ns: i128,
    pub max_ts_ns: i128,
    pub min_view: u64,
    pub max_view: u64,
    pub total_events: usize,
    pub event_kinds: Vec<EventMeta>,
}

/// View mode for the UI
#[derive(Clone, Debug, Serialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    Consensus,
    Actors,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Consensus
    }
}
