// ============================================================================
// Simplex Consensus Trace Visualizer - Main Entry Point
// ============================================================================

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Parser;
use tokio::signal;
use tracing::info;

use simplex_trace_tool::{
    events::EventRegistry,
    parser::log_parser,
    state::{AppState, builders},
    api,
};

/// CLI arguments.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Commonware Simplex Consensus Trace Visualizer"
)]
struct Args {
    /// Directory containing validator-*.log
    #[arg(long)]
    logs: Option<PathBuf>,
    /// Host to bind
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    /// Port to bind
    #[arg(long, default_value_t = 3011)]
    port: u16,
}

/// List validator log files in directory
fn list_validator_logs(dir: &std::path::Path) -> Vec<(u8, PathBuf)> {
    let mut files = BTreeMap::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("validator-") && name.ends_with(".log") {
                    if let Some(id_str) = name.strip_prefix("validator-").and_then(|s| s.strip_suffix(".log")) {
                        if let Ok(id) = id_str.parse::<u8>() {
                            files.insert(id, entry.path());
                        }
                    }
                }
            }
        }
    }
    files.into_iter().collect()
}

/// Ingest all log files and populate state
async fn ingest_all(state: &AppState, dir: &std::path::Path) {
    let registry = EventRegistry::new();
    let mut events = Vec::new();
    
    for (id, path) in list_validator_logs(dir) {
        info!("reading log file for validator {}", id);
        let mut evs = log_parser::read_log_file(&path, id, &registry);
        events.append(&mut evs);
    }
    
    // Sort and assign IDs
    builders::stable_sort_events(&mut events);
    for (idx, ev) in events.iter_mut().enumerate() {
        ev.id = idx as u64;
    }
    
    // Build derived state
    let edges = builders::build_edges(&events);
    let views = builders::build_view_states(&events);
    let segments = builders::build_segments(&events, 7);
    let meta = builders::build_metadata(&events);

    info!(
        "ingested {} events, {} edges, {} view records, {} segments",
        events.len(),
        edges.len(),
        views.len(),
        segments.len()
    );

    // Update state
    {
        let mut guard = state.events.write();
        *guard = events;
    }
    {
        let mut guard = state.edges.write();
        *guard = edges;
    }
    {
        let mut guard = state.views.write();
        *guard = views;
    }
    {
        let mut guard = state.segments.write();
        *guard = segments;
    }
    {
        let mut guard = state.meta.write();
        *guard = meta;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // Determine logs directory
    let default_logs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../logs");
    let candidate = args.logs.clone().unwrap_or_else(|| PathBuf::from("../logs"));
    let logs_dir = if candidate.exists() {
        candidate
    } else if default_logs.exists() {
        default_logs
    } else {
        PathBuf::from("./logs")
    };

    // Initialize state
    let state = AppState::new();
    
    // Ingest log files
    ingest_all(&state, &logs_dir).await;

    // Build router
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
    let app = api::build_router(state, static_dir);

    // Start server
    let addr = format!("{}:{}", args.host, args.port);
    info!("Simplex Trace Visualizer running on http://{}", addr);
    info!("logs directory: {:?}", logs_dir);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind failed");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = signal::ctrl_c().await;
        })
        .await
        .expect("server error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use simplex_trace_tool::events::EventKind;
    use simplex_trace_tool::parser::log_parser;
    use regex::Regex;
    
    fn sample_line(msg: &str) -> String {
        format!(
            "2024-01-15T10:00:00.123456789Z DEBUG ThreadId(01) commonware_consensus::simplex::actors::voter::actor: voter/actor.rs:100: {}",
            msg
        )
    }

    #[test]
    fn parse_and_classify_notarize() {
        let registry = simplex_trace_tool::events::EventRegistry::new();
        let raw = log_parser::parse_line(0, 1, &sample_line("broadcasting notarize proposal=Proposal { round: Round { epoch: Epoch(0), view: View(409) }, parent: View(408), payload: f0ccfc5b95becacd0a80eb5a2455abb6c66cfba28d47b99c7c2819fdfe0dc285 }")).expect("parse");
        let ev = log_parser::classify_event(raw, &registry, None);
        assert_eq!(ev.kind, EventKind::NotarizeBroadcast);
        assert_eq!(ev.view, Some(409));
        assert!(ev.payload.is_some());
        assert!(ev.message_id.is_some());
    }

    #[test]
    fn parse_leader_elected() {
        let registry = simplex_trace_tool::events::EventRegistry::new();
        let raw = log_parser::parse_line(0, 1, &sample_line("leader elected round=Round { epoch: Epoch(0), view: View(5) } leader=2 key=abc123")).expect("parse");
        let ev = log_parser::classify_event(raw, &registry, None);
        assert_eq!(ev.kind, EventKind::LeaderElected);
        assert_eq!(ev.view, Some(5));
        assert_eq!(ev.leader, Some(2));
    }

    #[test]
    fn event_categories() {
        use simplex_trace_tool::events::EventCategory;
        assert_eq!(EventCategory::from_target("commonware_consensus::simplex::actors::voter"), EventCategory::Consensus);
        assert_eq!(EventCategory::from_target("commonware_p2p::authenticated"), EventCategory::Network);
        assert_eq!(EventCategory::from_target("commonware_broadcast::buffered"), EventCategory::Broadcast);
    }
}
