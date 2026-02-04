// ============================================================================
// Simplex Consensus Trace Visualizer - Library
// ============================================================================
//
// A modular, extensible tool for visualizing Simplex consensus traces.
//
// ## Architecture
//
// - `events/`: Event classification and registry
// - `parser/`: Log parsing utilities
// - `state/`: Application state management
// - `api/`: HTTP API handlers and routing
//
// ## Adding Custom Events
//
// 1. Create a classifier function in `events/`:
//    ```rust
//    pub fn classify_my_event(msg: &str) -> Option<(EventKind, Option<String>)> {
//        if msg.contains("my event") {
//            Some((EventKind::MyEvent, None))
//        } else {
//            None
//        }
//    }
//    ```
//
// 2. Register in `EventRegistry::new()`:
//    ```rust
//    registry.register("my_events", classify_my_event);
//    ```

pub mod events;
pub mod parser;
pub mod state;
pub mod api;

pub use events::{EventKind, EventCategory, EventRegistry};
pub use state::{AppState, Event, Edge, ViewRecord};
