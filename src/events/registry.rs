// ============================================================================
// Event Registry - Pluggable event classification system
// ============================================================================
//
// The EventRegistry provides a modular way to classify log messages into events.
// Each classifier is a function that takes a message and returns an optional 
// (EventKind, Option<payload>) tuple.
//
// To add a new event classifier:
// 1. Create a classifier function: fn(msg: &str) -> Option<(EventKind, Option<String>)>
// 2. Register it in EventRegistry::new()
//
// Example:
// ```rust
// fn classify_my_event(msg: &str) -> Option<(EventKind, Option<String>)> {
//     if msg.contains("my custom event") {
//         Some((EventKind::MyCustomEvent, extract_payload(msg)))
//     } else {
//         None
//     }
// }
// ```

use super::EventKind;

/// A classifier function that attempts to classify a message
pub type EventClassifier = fn(&str) -> Option<(EventKind, Option<String>)>;

/// Registry of event classifiers
pub struct EventRegistry {
    classifiers: Vec<(&'static str, EventClassifier)>,
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EventRegistry {
    /// Create a new registry with all default classifiers
    pub fn new() -> Self {
        let mut registry = Self {
            classifiers: Vec::new(),
        };
        
        // Register all default classifiers
        // Order matters - first match wins
        registry.register("consensus", super::consensus::classify_consensus);
        registry.register("marshal", super::consensus::classify_marshal);
        registry.register("batcher", super::consensus::classify_batcher);
        registry.register("broadcast", super::broadcast::classify_broadcast);
        registry.register("resolver", super::broadcast::classify_resolver);
        registry.register("network", super::network::classify_network);
        registry.register("storage", super::storage::classify_storage);
        
        registry
    }
    
    /// Register a new classifier
    pub fn register(&mut self, name: &'static str, classifier: EventClassifier) {
        self.classifiers.push((name, classifier));
    }
    
    /// Classify a message using all registered classifiers
    pub fn classify(&self, msg: &str) -> (EventKind, Option<String>) {
        for (_, classifier) in &self.classifiers {
            if let Some(result) = classifier(msg) {
                return result;
            }
        }
        (EventKind::Other, None)
    }
    
    /// Get all registered classifier names
    pub fn classifier_names(&self) -> Vec<&'static str> {
        self.classifiers.iter().map(|(name, _)| *name).collect()
    }
}

/// Derive a message ID for correlation
pub fn derive_message_id(kind: EventKind, view: Option<u64>, payload: Option<&str>) -> Option<String> {
    use blake3::Hasher;
    
    let v = view?;
    let mut hasher = Hasher::new();
    hasher.update(&v.to_be_bytes());
    hasher.update(&[message_group_tag(kind)]);
    if let Some(p) = payload {
        hasher.update(p.as_bytes());
    }
    Some(hasher.finalize().to_hex()[..16].to_string())
}

/// Get message group tag for correlation
fn message_group_tag(kind: EventKind) -> u8 {
    match kind {
        EventKind::ProposalRequested | EventKind::ProposalReceived => 1,
        EventKind::NotarizeBroadcast => 3,
        EventKind::NotarizedBuilt | EventKind::NotarizationReceived => 4,
        EventKind::NullifyBroadcast => 5,
        EventKind::NullifiedBuilt | EventKind::NullificationReceived => 6,
        EventKind::FinalizeBroadcast => 7,
        EventKind::FinalizedBuilt | EventKind::FinalizationReceived => 8,
        _ => kind.as_tag(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_registry_classification() {
        let registry = EventRegistry::new();
        
        let (kind, _) = registry.classify("leader elected round=Round { view: View(5) }");
        assert_eq!(kind, EventKind::LeaderElected);
        
        let (kind, _) = registry.classify("broadcasting notarize proposal=abc123");
        assert_eq!(kind, EventKind::NotarizeBroadcast);
        
        let (kind, _) = registry.classify("unknown message");
        assert_eq!(kind, EventKind::Other);
    }
}
