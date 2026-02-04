// ============================================================================
// Storage Events - Archive and journal operations
// ============================================================================

use super::EventKind;

/// Classify storage-related events
pub fn classify_storage(msg: &str) -> Option<(EventKind, Option<String>)> {
    if msg.contains("appended item") {
        Some((EventKind::StorageAppend, None))
    } else if msg.contains("initializing archive") {
        Some((EventKind::ArchiveInit, None))
    } else if msg.contains("restored") && (msg.contains("archive") || msg.contains("intervals")) {
        Some((EventKind::ArchiveRestore, None))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_classify_storage() {
        let (kind, _) = classify_storage("appended item to journal").unwrap();
        assert_eq!(kind, EventKind::StorageAppend);
        
        let (kind, _) = classify_storage("initializing archive").unwrap();
        assert_eq!(kind, EventKind::ArchiveInit);
        
        assert!(classify_storage("unknown message").is_none());
    }
}
