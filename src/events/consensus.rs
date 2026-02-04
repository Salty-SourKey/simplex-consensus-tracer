// ============================================================================
// Consensus Events - Voter, Marshal, Batcher actors
// ============================================================================

use super::EventKind;
use crate::parser::extractors;

/// Classify consensus-related events (Voter actor)
pub fn classify_consensus(msg: &str) -> Option<(EventKind, Option<String>)> {
    if msg.contains("leader elected round=") {
        Some((EventKind::LeaderElected, None))
    } else if msg.contains("requested proposal from automaton") {
        Some((EventKind::ProposalRequested, None))
    } else if msg.contains("received proposal view=") {
        Some((EventKind::ProposalReceived, None))
    } else if msg.contains("requested proposal verification") {
        Some((EventKind::ProposalVerify, extractors::extract_payload(msg)))
    } else if msg.contains("broadcasting notarize proposal=") {
        Some((EventKind::NotarizeBroadcast, extractors::extract_payload(msg)))
    } else if msg.contains("broadcasting notarization proposal=") {
        Some((EventKind::NotarizedBuilt, extractors::extract_payload(msg)))
    } else if msg.contains("received notarization view=") {
        Some((EventKind::NotarizationReceived, None))
    } else if msg.contains("broadcasting nullify round=") {
        Some((EventKind::NullifyBroadcast, None))
    } else if msg.contains("broadcasting nullification round=") {
        Some((EventKind::NullifiedBuilt, None))
    } else if msg.contains("received nullification view=") {
        Some((EventKind::NullificationReceived, None))
    } else if msg.contains("broadcasting finalize proposal=") {
        Some((EventKind::FinalizeBroadcast, extractors::extract_payload(msg)))
    } else if msg.contains("broadcasting finalization proposal=") {
        Some((EventKind::FinalizedBuilt, extractors::extract_payload(msg)))
    } else if msg.contains("received finalization view=") {
        Some((EventKind::FinalizationReceived, None))
    } else if msg.contains("attempting certification") {
        Some((EventKind::CertificationAttempt, None))
    } else {
        None
    }
}

/// Classify Marshal actor events
pub fn classify_marshal(msg: &str) -> Option<(EventKind, Option<String>)> {
    if msg.contains("cached round=") {
        Some((EventKind::MarshalCached, None))
    } else if msg.contains("registering subscriber") {
        Some((EventKind::MarshalSubscriber, None))
    } else if msg.contains("requested broadcast of built block") {
        Some((EventKind::MarshalBroadcast, extractors::extract_payload(msg)))
    } else {
        None
    }
}

/// Classify Batcher actor events
pub fn classify_batcher(msg: &str) -> Option<(EventKind, Option<String>)> {
    if msg.contains("no verifier ready") {
        Some((EventKind::BatcherNoVerifier, None))
    } else if msg.contains("skipping duplicate") {
        Some((EventKind::BatcherSkipDuplicate, None))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_classify_consensus() {
        let (kind, _) = classify_consensus("leader elected round=Round { view: View(5) }").unwrap();
        assert_eq!(kind, EventKind::LeaderElected);
        
        let (kind, _) = classify_consensus("requested proposal from automaton").unwrap();
        assert_eq!(kind, EventKind::ProposalRequested);
        
        assert!(classify_consensus("unknown message").is_none());
    }
    
    #[test]
    fn test_classify_marshal() {
        let (kind, _) = classify_marshal("cached round=5").unwrap();
        assert_eq!(kind, EventKind::MarshalCached);
    }
}
