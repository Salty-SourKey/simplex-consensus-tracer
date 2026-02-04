// ============================================================================
// API Routes - Router configuration
// ============================================================================

use axum::{
    routing::get,
    Router,
};
use tower_http::services::ServeDir;
use std::path::PathBuf;

use crate::state::AppState;
use super::handlers;

/// Build the application router
pub fn build_router(state: AppState, static_dir: PathBuf) -> Router {
    Router::new()
        // Index page
        .route("/", get(handlers::serve_index))
        
        // Consensus mode endpoints
        .route("/api/events", get(handlers::get_events))
        .route("/api/edges", get(handlers::get_edges))
        .route("/api/views", get(handlers::get_views))
        .route("/api/segments", get(handlers::get_segments))
        .route("/api/meta", get(handlers::get_metadata))
        
        // Health check
        .route("/healthz", get(handlers::health))
        
        // Static files
        .nest_service("/static", ServeDir::new(static_dir))
        
        .with_state(state)
}
