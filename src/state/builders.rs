// ============================================================================
// State Builders - Build derived state from events
// ============================================================================

use std::collections::{BTreeMap, BTreeSet};

use super::types::*;
use crate::events::{EventCategory, EventKind};

/// Sort events by timestamp, view, node, and line number
pub fn stable_sort_events(events: &mut [Event]) {
    events.sort_by(|a, b| {
        let va = a.view.unwrap_or(u64::MAX);
        let vb = b.view.unwrap_or(u64::MAX);
        (
            a.raw.timestamp_ns,
            va,
            a.raw.node_id,
            a.raw.line_number,
        )
            .cmp(&(
                b.raw.timestamp_ns,
                vb,
                b.raw.node_id,
                b.raw.line_number,
            ))
    });
}

/// Build edges from send/receive event pairs
pub fn build_edges(events: &[Event]) -> Vec<Edge> {
    let mut edges = Vec::new();
    let mut sends: BTreeMap<String, &Event> = BTreeMap::new();
    
    for ev in events {
        if let Some(mid) = ev.message_id.as_ref() {
            if ev.kind.is_send() {
                sends.entry(mid.clone()).or_insert(ev);
            } else if ev.kind.is_recv() {
                if let Some(send_ev) = sends.get(mid) {
                    if send_ev.raw.node_id != ev.raw.node_id {
                        let latency_ns = ev.raw.timestamp_ns - send_ev.raw.timestamp_ns;
                        edges.push(Edge {
                            message_id: mid.clone(),
                            from_node: send_ev.raw.node_id,
                            to_node: ev.raw.node_id,
                            send_ts_ns: send_ev.raw.timestamp_ns,
                            recv_ts_ns: ev.raw.timestamp_ns,
                            latency_ms: latency_ns as f64 / 1_000_000.0,
                            kind: send_ev.kind,
                        });
                    }
                }
            }
        }
    }
    edges
}

fn precedence(status: &ViewStatus) -> u8 {
    match status {
        ViewStatus::Entered => 1,
        ViewStatus::ProposalReceived => 2,
        ViewStatus::Notarized => 3,
        ViewStatus::Nullified => 4,
        ViewStatus::Finalized => 5,
    }
}

/// Build view state records from events
pub fn build_view_states(events: &[Event]) -> BTreeMap<(u8, u64), ViewRecord> {
    let mut map: BTreeMap<(u8, u64), ViewRecord> = BTreeMap::new();
    
    for ev in events {
        if let Some(view) = ev.view {
            let (status, ts_field) = match ev.kind {
                EventKind::LeaderElected => (ViewStatus::Entered, "entered"),
                EventKind::ProposalReceived | EventKind::ProposalRequested => 
                    (ViewStatus::ProposalReceived, "proposal"),
                EventKind::NotarizeBroadcast | EventKind::NotarizedBuilt | EventKind::NotarizationReceived => 
                    (ViewStatus::Notarized, "notarized"),
                EventKind::NullifyBroadcast | EventKind::NullifiedBuilt | EventKind::NullificationReceived => 
                    (ViewStatus::Nullified, "nullified"),
                EventKind::FinalizeBroadcast | EventKind::FinalizedBuilt | EventKind::FinalizationReceived => 
                    (ViewStatus::Finalized, "finalized"),
                _ => continue,
            };

            let key = (ev.raw.node_id, view);
            let entry = map.entry(key).or_insert(ViewRecord {
                node_id: ev.raw.node_id,
                view,
                status: ViewStatus::Entered,
                leader: ev.leader,
                entered_ns: None,
                proposal_ns: None,
                notarized_ns: None,
                nullified_ns: None,
                finalized_ns: None,
                latency_to_notarize_ms: None,
                latency_to_finalize_ms: None,
            });

            if ev.leader.is_some() && entry.leader.is_none() {
                entry.leader = ev.leader;
            }

            if precedence(&status) > precedence(&entry.status) {
                entry.status = status;
            }

            match ts_field {
                "entered" if entry.entered_ns.is_none() => {
                    entry.entered_ns = Some(ev.raw.timestamp_ns);
                }
                "proposal" if entry.proposal_ns.is_none() => {
                    entry.proposal_ns = Some(ev.raw.timestamp_ns);
                    if entry.entered_ns.is_none() {
                        entry.entered_ns = Some(ev.raw.timestamp_ns);
                    }
                }
                "notarized" if entry.notarized_ns.is_none() => {
                    entry.notarized_ns = Some(ev.raw.timestamp_ns);
                }
                "nullified" if entry.nullified_ns.is_none() => {
                    entry.nullified_ns = Some(ev.raw.timestamp_ns);
                }
                "finalized" if entry.finalized_ns.is_none() => {
                    entry.finalized_ns = Some(ev.raw.timestamp_ns);
                }
                _ => {}
            }

            // Calculate latencies
            if let (Some(start), Some(end)) = (entry.entered_ns, entry.notarized_ns) {
                entry.latency_to_notarize_ms = Some((end - start) as f64 / 1_000_000.0);
            }
            if let (Some(start), Some(end)) = (entry.entered_ns, entry.finalized_ns) {
                entry.latency_to_finalize_ms = Some((end - start) as f64 / 1_000_000.0);
            }
        }
    }
    map
}

/// Build time segments for navigation
pub fn build_segments(events: &[Event], num_segments: usize) -> Vec<TimeSegment> {
    if events.is_empty() {
        return Vec::new();
    }

    let min_ts = events.first().map(|e| e.raw.timestamp_ns).unwrap_or(0);
    let max_ts = events.last().map(|e| e.raw.timestamp_ns).unwrap_or(0);
    let span = max_ts - min_ts;
    
    if span == 0 {
        return vec![TimeSegment {
            index: 0,
            start_ns: min_ts,
            end_ns: max_ts,
            event_count: events.len(),
            views: events.iter().filter_map(|e| e.view).collect(),
        }];
    }

    let mut segments = Vec::with_capacity(num_segments);
    
    for i in 0..num_segments {
        let start_ns = min_ts + (span * i as i128) / num_segments as i128;
        let end_ns = min_ts + (span * (i + 1) as i128) / num_segments as i128;
        
        let mut event_count = 0;
        let mut views = BTreeSet::new();
        
        for ev in events.iter() {
            if ev.raw.timestamp_ns >= start_ns && ev.raw.timestamp_ns < end_ns {
                event_count += 1;
                if let Some(v) = ev.view {
                    views.insert(v);
                }
            }
        }
        
        segments.push(TimeSegment {
            index: i,
            start_ns,
            end_ns,
            event_count,
            views: views.into_iter().collect(),
        });
    }
    
    segments
}

/// Build metadata from events
pub fn build_metadata(events: &[Event]) -> Metadata {
    if events.is_empty() {
        return Metadata::default();
    }

    let mut nodes = BTreeSet::new();
    let mut min_view = u64::MAX;
    let mut max_view = 0u64;
    let mut kind_set = BTreeSet::new();

    for ev in events {
        nodes.insert(ev.raw.node_id);
        if let Some(v) = ev.view {
            min_view = min_view.min(v);
            max_view = max_view.max(v);
        }
        kind_set.insert(ev.kind);
    }

    let event_kinds: Vec<EventMeta> = kind_set
        .into_iter()
        .map(|kind| EventMeta {
            kind,
            tag: kind.as_tag(),
            label: kind.label().to_string(),
            category: EventCategory::from_target(&format!("{:?}", kind)),
            color: kind.color().to_string(),
        })
        .collect();

    Metadata {
        nodes: nodes.into_iter().collect(),
        min_ts_ns: events.first().map(|e| e.raw.timestamp_ns).unwrap_or(0),
        max_ts_ns: events.last().map(|e| e.raw.timestamp_ns).unwrap_or(0),
        min_view: if min_view == u64::MAX { 0 } else { min_view },
        max_view,
        total_events: events.len(),
        event_kinds,
    }
}
