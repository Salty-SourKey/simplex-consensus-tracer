// ============================================================================
// Network Events - P2P connections and peer management
// ============================================================================

use super::EventKind;

/// Classify network-related events
pub fn classify_network(msg: &str) -> Option<(EventKind, Option<String>)> {
    if msg.contains("dialed peer") {
        Some((EventKind::PeerDialed, None))
    } else if msg.contains("upgraded connection peer=") {
        Some((EventKind::PeerConnected, None))
    } else if msg.contains("peer started peer=") {
        Some((EventKind::PeerStarted, None))
    } else if msg.contains("peer ready peer=") {
        Some((EventKind::PeerReady, None))
    } else if msg.contains("updated peer record") {
        Some((EventKind::PeerRecordUpdate, None))
    } else if msg.contains("accepted incoming connection") {
        Some((EventKind::ConnectionAccepted, None))
    } else if msg.contains("completed handshake") {
        Some((EventKind::HandshakeComplete, None))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_classify_network() {
        let (kind, _) = classify_network("dialed peer abc123").unwrap();
        assert_eq!(kind, EventKind::PeerDialed);
        
        let (kind, _) = classify_network("peer ready peer=abc123").unwrap();
        assert_eq!(kind, EventKind::PeerReady);
        
        assert!(classify_network("unknown message").is_none());
    }
}
