// ============================================================================
// Extractors - Field extraction from log messages
// ============================================================================

use regex::Regex;
use std::sync::LazyLock;

// Pre-compiled regex patterns for performance
static RE_VIEW: LazyLock<Regex> = LazyLock::new(|| {
    // Matches: view=View(123), view: View(123), view=123 (case-insensitive, ignores spaces)
    Regex::new(r"(?i)view\s*[=:]\s*view?\s*\(?([0-9]+)\)?").unwrap()
});

static RE_ROUND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"round=\(?(?:Round\s*\{[^}]*view:\s*View\()?([0-9]+)").unwrap()
});

static RE_CURRENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"current[=:]?\s*([0-9]+)").unwrap()
});

static RE_PAYLOAD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"payload[:=]\s*([0-9a-fA-F]{64})").unwrap()
});

static RE_LEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"leader=([0-9]+)").unwrap()
});

static RE_PEER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"peer=([0-9a-fA-F]{64})").unwrap()
});

static RE_ACTOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"actor=(\w+)").unwrap()
});

static RE_FROM_ACTOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"from=(\w+)").unwrap()
});

static RE_TO_ACTOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"to=(\w+)").unwrap()
});

/// Extract view number from message
pub fn extract_view(msg: &str) -> Option<u64> {
    // Fast path: scan for common substrings without backtracking
    fn parse_digits(slice: &str) -> Option<u64> {
        let mut digits = String::new();
        for ch in slice.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else if !digits.is_empty() {
                break;
            } else if ch.is_ascii_whitespace() || ch == '(' {
                continue;
            } else {
                break;
            }
        }
        if digits.is_empty() {
            None
        } else {
            digits.parse::<u64>().ok()
        }
    }

    let lower = msg.to_lowercase();

    // view=..., view:..., view(...).
    for needle in ["view=", "view:", "view("] {
        if let Some(pos) = lower.find(needle) {
            if let Some(v) = parse_digits(&msg[pos + needle.len()..]) {
                return Some(v);
            }
        }
    }

    // Try view=View(N) or view=N format
    if let Some(c) = RE_VIEW.captures(msg).and_then(|c| c.get(1)) {
        if let Ok(v) = c.as_str().parse::<u64>() {
            return Some(v);
        }
    }
    // Try round=(0, N) format
    if let Some(c) = RE_ROUND.captures(msg).and_then(|c| c.get(1)) {
        if let Ok(v) = c.as_str().parse::<u64>() {
            return Some(v);
        }
    }
    // Try current=N format
    RE_CURRENT
        .captures(msg)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok())
}

/// Extract payload hash from message
pub fn extract_payload(msg: &str) -> Option<String> {
    RE_PAYLOAD
        .captures(msg)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_lowercase())
}

/// Extract leader ID from message
pub fn extract_leader(msg: &str) -> Option<u8> {
    RE_LEADER
        .captures(msg)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u8>().ok())
}

/// Extract peer ID from message
pub fn extract_peer(msg: &str) -> Option<String> {
    RE_PEER
        .captures(msg)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_lowercase())
}

/// Extract actor name from message
pub fn extract_actor(msg: &str) -> Option<String> {
    RE_ACTOR
        .captures(msg)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract source actor from message
pub fn extract_from_actor(msg: &str) -> Option<String> {
    RE_FROM_ACTOR
        .captures(msg)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract destination actor from message
pub fn extract_to_actor(msg: &str) -> Option<String> {
    RE_TO_ACTOR
        .captures(msg)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_view() {
        assert_eq!(extract_view("view=View(123)"), Some(123));
        assert_eq!(extract_view("view=123"), Some(123));
        assert_eq!(extract_view("round=Round { view: View(456) }"), Some(456));
        assert_eq!(extract_view("current=789"), Some(789));
        assert_eq!(extract_view("no view here"), None);
    }
    
    #[test]
    fn test_extract_payload() {
        let msg = "payload=a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        assert!(extract_payload(msg).is_some());
        assert_eq!(extract_payload("no payload"), None);
    }
    
    #[test]
    fn test_extract_leader() {
        assert_eq!(extract_leader("leader=5"), Some(5));
        assert_eq!(extract_leader("no leader"), None);
    }
}
