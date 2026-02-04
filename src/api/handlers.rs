// ============================================================================
// API Handlers - Request handlers for all endpoints
// ============================================================================

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
    Json,
};
use serde::Deserialize;
use std::collections::HashSet;

use crate::state::{AppState, Event, ViewRecord};

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Deserialize)]
pub struct EventQuery {
    pub from_ts: Option<i128>,
    pub to_ts: Option<i128>,
    pub nodes: Option<String>,
    pub kinds: Option<String>,
    pub categories: Option<String>,
    pub view_from: Option<u64>,
    pub view_to: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub segment: Option<usize>,
}

// ============================================================================
// Consensus Mode Handlers
// ============================================================================

/// Get events with filtering
pub async fn get_events(
    State(state): State<AppState>,
    Query(q): Query<EventQuery>,
) -> impl IntoResponse {
    let events = state.events.read();
    let segments = state.segments.read();
    
    // Determine time range from segment if specified
    let (seg_start, seg_end) = if let Some(seg_idx) = q.segment {
        if let Some(seg) = segments.get(seg_idx) {
            (Some(seg.start_ns), Some(seg.end_ns))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let filtered = events.iter().filter(|ev| {
        // Time range filter
        let from_ts = q.from_ts.or(seg_start);
        let to_ts = q.to_ts.or(seg_end);
        
        if let Some(from) = from_ts {
            if ev.raw.timestamp_ns < from {
                return false;
            }
        }
        if let Some(to) = to_ts {
            if ev.raw.timestamp_ns > to {
                return false;
            }
        }
        
        // View range filter
        if let Some(vf) = q.view_from {
            if ev.view.map_or(true, |v| v < vf) {
                return false;
            }
        }
        if let Some(vt) = q.view_to {
            if ev.view.map_or(true, |v| v > vt) {
                return false;
            }
        }
        
        // Node filter
        if let Some(ref nodes) = q.nodes {
            let node_set: HashSet<u8> = nodes
                .split(',')
                .filter_map(|s| s.trim().parse::<u8>().ok())
                .collect();
            if !node_set.is_empty() && !node_set.contains(&ev.raw.node_id) {
                return false;
            }
        }
        
        // Kind filter
        if let Some(ref kinds) = q.kinds {
            let kind_set: HashSet<u8> = kinds
                .split(',')
                .filter_map(|s| s.trim().parse::<u8>().ok())
                .collect();
            if !kind_set.is_empty() && !kind_set.contains(&ev.kind_tag) {
                return false;
            }
        }
        
        // Category filter
        if let Some(ref categories) = q.categories {
            let cat_names: Vec<&str> = categories.split(',').map(|s| s.trim()).collect();
            if !cat_names.is_empty() {
                let ev_cat = format!("{:?}", ev.category).to_lowercase();
                if !cat_names.iter().any(|c| c.to_lowercase() == ev_cat) {
                    return false;
                }
            }
        }
        
        true
    });

    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(10000);
    let slice: Vec<Event> = filtered.skip(offset).take(limit).cloned().collect();

    Json(slice)
}

/// Get message edges
pub async fn get_edges(State(state): State<AppState>) -> impl IntoResponse {
    let edges = state.edges.read();
    Json(edges.clone())
}

/// Get view records
pub async fn get_views(State(state): State<AppState>) -> impl IntoResponse {
    let views = state.views.read();
    let values: Vec<ViewRecord> = views.values().cloned().collect();
    Json(values)
}

/// Get time segments
pub async fn get_segments(State(state): State<AppState>) -> impl IntoResponse {
    let segments = state.segments.read();
    Json(segments.clone())
}

/// Get metadata
pub async fn get_metadata(State(state): State<AppState>) -> impl IntoResponse {
    let meta = state.meta.read();
    Json(meta.clone())
}

// ============================================================================
// Health and Index
// ============================================================================

/// Health check endpoint
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let ev_count = state.events.read().len();
    let edge_count = state.edges.read().len();
    let view_count = state.views.read().len();
    
    Json(serde_json::json!({
        "status": "ok",
        "events": ev_count,
        "edges": edge_count,
        "views": view_count
    }))
}

/// Serve index HTML
pub async fn serve_index() -> impl IntoResponse {
    Html(INDEX_HTML)
}

// ============================================================================
// Embedded HTML
// ============================================================================

pub const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Simplex Consensus Trace Visualizer</title>
    <link rel="stylesheet" href="/static/styles.css">
</head>
<body>
    <div id="app">
        <header id="header">
            <div class="header-left">
                <h1>Simplex Consensus Tracer</h1>
                <span class="subtitle" id="status">Loading...</span>
            </div>
            <div class="header-center">
                <div class="segment-nav" id="segment-nav"></div>
            </div>
            <div class="header-right">
                <button id="view-mode-toggle" class="view-toggle-btn attention" title="Toggle Pipelining View" aria-pressed="false">
                    <span class="icon" aria-hidden="true">⟂</span>
                    <span class="eyebrow">Visualization</span>
                    <span class="label">Standard View</span>
                </button>
                <div class="keyboard-hints">
                    <kbd>1</kbd>-<kbd>7</kbd> segments
                    <kbd>←</kbd><kbd>→</kbd> pan
                    <kbd>[</kbd><kbd>]</kbd> zoom
                </div>
            </div>
        </header>
        
        <div id="minimap-container">
            <canvas id="minimap"></canvas>
            <div id="minimap-viewport"></div>
        </div>
        
        <div id="pipeline-container">
            <div class="pipeline-header">
                <span class="pipeline-title">Pipeline View</span>
                <div class="pipeline-legend">
                    <span class="legend-item"><span class="legend-dot proposal"></span>Proposal</span>
                    <span class="legend-item"><span class="legend-dot notarization"></span>Notarization</span>
                    <span class="legend-item"><span class="legend-dot finalization"></span>Finalization</span>
                </div>
            </div>
            <canvas id="pipeline"></canvas>
        </div>
        
        <div id="main-container">
            <aside id="sidebar">
                <div class="filter-section">
                    <h3>Categories</h3>
                    <div id="category-filters"></div>
                </div>
                <div class="filter-section">
                    <h3>Event Types</h3>
                    <div id="kind-filters"></div>
                </div>
                <div class="filter-section">
                    <h3>Nodes</h3>
                    <div id="node-filters"></div>
                </div>
            </aside>
            
            <main id="timeline-container">
                <canvas id="timeline"></canvas>
                <div id="tooltip" class="tooltip"></div>
            </main>
        </div>
        
        <footer id="footer">
            <div id="view-info"></div>
            <div id="time-info"></div>
        </footer>
    </div>
    <script src="/static/app.js"></script>
</body>
</html>
"##;
