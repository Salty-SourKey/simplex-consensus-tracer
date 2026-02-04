// ============================================================================
// Application State - Thread-safe state management
// ============================================================================

use std::collections::BTreeMap;
use std::sync::Arc;
use parking_lot::RwLock;

use super::types::*;

/// Thread-safe application state
#[derive(Clone)]
pub struct AppState {
    pub events: Arc<RwLock<Vec<Event>>>,
    pub edges: Arc<RwLock<Vec<Edge>>>,
    pub views: Arc<RwLock<BTreeMap<(u8, u64), ViewRecord>>>,
    pub segments: Arc<RwLock<Vec<TimeSegment>>>,
    pub meta: Arc<RwLock<Metadata>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            edges: Arc::new(RwLock::new(Vec::new())),
            views: Arc::new(RwLock::new(BTreeMap::new())),
            segments: Arc::new(RwLock::new(Vec::new())),
            meta: Arc::new(RwLock::new(Metadata::default())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
