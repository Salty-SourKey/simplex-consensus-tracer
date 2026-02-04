// ============================================================================
// Broadcast Events - Buffered broadcast and resolver engines
// ============================================================================

use super::EventKind;

/// Classify buffered broadcast engine events
pub fn classify_broadcast(msg: &str) -> Option<(EventKind, Option<String>)> {
    if msg.contains("mailbox: broadcast") {
        Some((EventKind::BufferedBroadcast, None))
    } else if msg.contains("mailbox: get") {
        Some((EventKind::BufferedGet, None))
    } else if msg.contains("mailbox: subscribe") {
        Some((EventKind::BufferedSubscribe, None))
    } else if msg.contains("network peer=") {
        Some((EventKind::BufferedNetwork, None))
    } else {
        None
    }
}

/// Classify resolver engine events
pub fn classify_resolver(msg: &str) -> Option<(EventKind, Option<String>)> {
    if msg.contains("mailbox: cancel") {
        Some((EventKind::ResolverCancel, None))
    } else if msg.contains("mailbox: retain") {
        Some((EventKind::ResolverRetain, None))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_classify_broadcast() {
        let (kind, _) = classify_broadcast("mailbox: broadcast").unwrap();
        assert_eq!(kind, EventKind::BufferedBroadcast);
        
        let (kind, _) = classify_broadcast("mailbox: get").unwrap();
        assert_eq!(kind, EventKind::BufferedGet);
        
        assert!(classify_broadcast("unknown message").is_none());
    }
    
    #[test]
    fn test_classify_resolver() {
        let (kind, _) = classify_resolver("mailbox: cancel").unwrap();
        assert_eq!(kind, EventKind::ResolverCancel);
        
        assert!(classify_resolver("unknown message").is_none());
    }
}
