// ============================================================================
// Events Module - Modular event definitions and registry
// ============================================================================
//
// This module provides a pluggable event system. To add a new event:
// 1. Create a new file in src/events/ (e.g., my_events.rs)
// 2. Define your EventKind variants
// 3. Implement the event classifier
// 4. Register in the EventRegistry
//
// See consensus.rs for examples.

mod registry;
mod consensus;
mod network;
mod storage;
mod broadcast;

pub use registry::*;
pub use consensus::*;
pub use network::*;
pub use storage::*;
pub use broadcast::*;

use serde::Serialize;

// ============================================================================
// Event Categories
// ============================================================================

#[derive(Clone, Debug, Serialize, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Consensus,
    Network,
    Storage,
    Resolver,
    Marshal,
    Batcher,
    Broadcast,
    Custom,
}

impl EventCategory {
    pub fn from_target(target: &str) -> Self {
        if target.contains("voter") || target.contains("round") {
            EventCategory::Consensus
        } else if target.contains("p2p") || target.contains("discovery") {
            EventCategory::Network
        } else if target.contains("storage") || target.contains("journal") || target.contains("archive") {
            EventCategory::Storage
        } else if target.contains("resolver") {
            EventCategory::Resolver
        } else if target.contains("marshal") {
            EventCategory::Marshal
        } else if target.contains("batcher") {
            EventCategory::Batcher
        } else if target.contains("broadcast") || target.contains("buffered") {
            EventCategory::Broadcast
        } else {
            EventCategory::Consensus
        }
    }
    
    pub fn all() -> Vec<EventCategory> {
        vec![
            EventCategory::Consensus,
            EventCategory::Network,
            EventCategory::Storage,
            EventCategory::Resolver,
            EventCategory::Marshal,
            EventCategory::Batcher,
            EventCategory::Broadcast,
            EventCategory::Custom,
        ]
    }
}

// ============================================================================
// Event Kind - All possible event types
// ============================================================================

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    // Consensus - View lifecycle (1-20)
    LeaderElected,
    ProposalRequested,
    ProposalReceived,
    ProposalVerify,
    
    // Consensus - Voting
    NotarizeBroadcast,
    NotarizedBuilt,
    NotarizationReceived,
    NullifyBroadcast,
    NullifiedBuilt,
    NullificationReceived,
    FinalizeBroadcast,
    FinalizedBuilt,
    FinalizationReceived,
    CertificationAttempt,
    
    // Marshal Actor (21-30)
    MarshalCached,
    MarshalSubscriber,
    MarshalBroadcast,
    
    // Batcher Actor (31-40)
    BatcherNoVerifier,
    BatcherSkipDuplicate,
    
    // Buffered Broadcast Engine (41-50)
    BufferedBroadcast,
    BufferedGet,
    BufferedSubscribe,
    BufferedNetwork,
    
    // Resolver Engine (51-60)
    ResolverCancel,
    ResolverRetain,
    
    // Network - P2P (61-80)
    PeerDialed,
    PeerConnected,
    PeerStarted,
    PeerReady,
    PeerRecordUpdate,
    ConnectionAccepted,
    HandshakeComplete,
    
    // Storage (81-90)
    StorageAppend,
    ArchiveInit,
    ArchiveRestore,
    
    // Other
    Other,
}

impl EventKind {
    pub fn as_tag(&self) -> u8 {
        match self {
            // Consensus events: 1-20
            EventKind::LeaderElected => 1,
            EventKind::ProposalRequested => 2,
            EventKind::ProposalReceived => 3,
            EventKind::ProposalVerify => 4,
            EventKind::NotarizeBroadcast => 5,
            EventKind::NotarizedBuilt => 6,
            EventKind::NotarizationReceived => 7,
            EventKind::NullifyBroadcast => 8,
            EventKind::NullifiedBuilt => 9,
            EventKind::NullificationReceived => 10,
            EventKind::FinalizeBroadcast => 11,
            EventKind::FinalizedBuilt => 12,
            EventKind::FinalizationReceived => 13,
            EventKind::CertificationAttempt => 14,
            
            // Marshal events: 21-30
            EventKind::MarshalCached => 21,
            EventKind::MarshalSubscriber => 22,
            EventKind::MarshalBroadcast => 23,
            
            // Batcher events: 31-40
            EventKind::BatcherNoVerifier => 31,
            EventKind::BatcherSkipDuplicate => 32,
            
            // Buffered broadcast: 41-50
            EventKind::BufferedBroadcast => 41,
            EventKind::BufferedGet => 42,
            EventKind::BufferedSubscribe => 43,
            EventKind::BufferedNetwork => 44,
            
            // Resolver: 51-60
            EventKind::ResolverCancel => 51,
            EventKind::ResolverRetain => 52,
            
            // Network: 61-80
            EventKind::PeerDialed => 61,
            EventKind::PeerConnected => 62,
            EventKind::PeerStarted => 63,
            EventKind::PeerReady => 64,
            EventKind::PeerRecordUpdate => 65,
            EventKind::ConnectionAccepted => 66,
            EventKind::HandshakeComplete => 67,
            
            // Storage: 81-90
            EventKind::StorageAppend => 81,
            EventKind::ArchiveInit => 82,
            EventKind::ArchiveRestore => 83,
            
            EventKind::Other => 255,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            EventKind::LeaderElected => "Leader Elected",
            EventKind::ProposalRequested => "Proposal Requested",
            EventKind::ProposalReceived => "Proposal Received",
            EventKind::ProposalVerify => "Proposal Verify",
            EventKind::NotarizeBroadcast => "Notarize Vote",
            EventKind::NotarizedBuilt => "Notarization Built",
            EventKind::NotarizationReceived => "Notarization Received",
            EventKind::NullifyBroadcast => "Nullify Vote",
            EventKind::NullifiedBuilt => "Nullification Built",
            EventKind::NullificationReceived => "Nullification Received",
            EventKind::FinalizeBroadcast => "Finalize Vote",
            EventKind::FinalizedBuilt => "Finalization Built",
            EventKind::FinalizationReceived => "Finalization Received",
            EventKind::CertificationAttempt => "Certification Attempt",
            EventKind::MarshalCached => "Marshal Cached",
            EventKind::MarshalSubscriber => "Marshal Subscriber",
            EventKind::MarshalBroadcast => "Marshal Broadcast",
            EventKind::BatcherNoVerifier => "Batcher No Verifier",
            EventKind::BatcherSkipDuplicate => "Batcher Skip Duplicate",
            EventKind::BufferedBroadcast => "Buffered Broadcast",
            EventKind::BufferedGet => "Buffered Get",
            EventKind::BufferedSubscribe => "Buffered Subscribe",
            EventKind::BufferedNetwork => "Buffered Network",
            EventKind::ResolverCancel => "Resolver Cancel",
            EventKind::ResolverRetain => "Resolver Retain",
            EventKind::PeerDialed => "Peer Dialed",
            EventKind::PeerConnected => "Peer Connected",
            EventKind::PeerStarted => "Peer Started",
            EventKind::PeerReady => "Peer Ready",
            EventKind::PeerRecordUpdate => "Peer Record Update",
            EventKind::ConnectionAccepted => "Connection Accepted",
            EventKind::HandshakeComplete => "Handshake Complete",
            EventKind::StorageAppend => "Storage Append",
            EventKind::ArchiveInit => "Archive Init",
            EventKind::ArchiveRestore => "Archive Restore",
            EventKind::Other => "Other",
        }
    }
    
    pub fn color(&self) -> &'static str {
        match self {
            EventKind::LeaderElected => "#60a5fa",           // blue-400
            EventKind::ProposalRequested => "#34d399",       // emerald-400
            EventKind::ProposalReceived => "#2dd4bf",        // teal-400
            EventKind::ProposalVerify => "#22d3ee",          // cyan-400
            EventKind::NotarizeBroadcast => "#a78bfa",       // violet-400
            EventKind::NotarizedBuilt => "#c4b5fd",          // violet-300
            EventKind::NotarizationReceived => "#ddd6fe",    // violet-200
            EventKind::NullifyBroadcast => "#fb923c",        // orange-400
            EventKind::NullifiedBuilt => "#fdba74",          // orange-300
            EventKind::NullificationReceived => "#fed7aa",   // orange-200
            EventKind::FinalizeBroadcast => "#f87171",       // red-400
            EventKind::FinalizedBuilt => "#fca5a5",          // red-300
            EventKind::FinalizationReceived => "#fecaca",    // red-200
            EventKind::CertificationAttempt => "#fbbf24",    // amber-400
            EventKind::MarshalCached => "#4ade80",           // green-400
            EventKind::MarshalSubscriber => "#86efac",       // green-300
            EventKind::MarshalBroadcast => "#bbf7d0",        // green-200
            EventKind::BatcherNoVerifier => "#94a3b8",       // slate-400
            EventKind::BatcherSkipDuplicate => "#cbd5e1",    // slate-300
            EventKind::BufferedBroadcast => "#38bdf8",       // sky-400
            EventKind::BufferedGet => "#7dd3fc",             // sky-300
            EventKind::BufferedSubscribe => "#bae6fd",       // sky-200
            EventKind::BufferedNetwork => "#e0f2fe",         // sky-100
            EventKind::ResolverCancel => "#e879f9",          // fuchsia-400
            EventKind::ResolverRetain => "#f0abfc",          // fuchsia-300
            EventKind::PeerDialed => "#818cf8",              // indigo-400
            EventKind::PeerConnected => "#a5b4fc",           // indigo-300
            EventKind::PeerStarted => "#c7d2fe",             // indigo-200
            EventKind::PeerReady => "#e0e7ff",               // indigo-100
            EventKind::PeerRecordUpdate => "#6366f1",        // indigo-500
            EventKind::ConnectionAccepted => "#8b5cf6",      // violet-500
            EventKind::HandshakeComplete => "#a855f7",       // purple-500
            EventKind::StorageAppend => "#64748b",           // slate-500
            EventKind::ArchiveInit => "#475569",             // slate-600
            EventKind::ArchiveRestore => "#334155",          // slate-700
            EventKind::Other => "#9ca3af",                   // gray-400
        }
    }

    pub fn is_vote(&self) -> bool {
        matches!(
            self,
            EventKind::NotarizeBroadcast
                | EventKind::NullifyBroadcast
                | EventKind::FinalizeBroadcast
        )
    }

    pub fn is_certificate(&self) -> bool {
        matches!(
            self,
            EventKind::NotarizedBuilt
                | EventKind::NullifiedBuilt
                | EventKind::FinalizedBuilt
        )
    }

    pub fn is_send(&self) -> bool {
        matches!(
            self,
            EventKind::ProposalRequested
                | EventKind::NotarizeBroadcast
                | EventKind::NullifyBroadcast
                | EventKind::FinalizeBroadcast
                | EventKind::NotarizedBuilt
                | EventKind::NullifiedBuilt
                | EventKind::FinalizedBuilt
                | EventKind::MarshalBroadcast
                | EventKind::BufferedBroadcast
        )
    }

    pub fn is_recv(&self) -> bool {
        matches!(
            self,
            EventKind::ProposalReceived
                | EventKind::NotarizationReceived
                | EventKind::NullificationReceived
                | EventKind::FinalizationReceived
        )
    }
}
