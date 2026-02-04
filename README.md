# Simplex Consensus Trace Visualizer

A visualization tool for analyzing Commonware Simplex consensus protocol traces with timeline-based analysis.

## Features

- **Pipeline Visualization**: Track pipelining of proposal, notarization, and finalization phases
- **Message Tracing**: Visualize broadcast/receive edges across nodes
- **Modular Architecture**: Extend with custom events

## Quick Start

1. **Run validators with full traces**  
   Start your validators that runs blockchain client with Commonware Simplex (e.g., Alto, Tempo) with `log_level: trace` (or `--log-level trace`) so consensus events are emitted. Direct each validator’s stdout/stderr to its own file.

2. **Collect logs in log directory**  
   The visualizer expects files named `validator-{0..n}.log`.

3. **Build and run the visualizer**  
   From this repository root:  
   `cargo run -p simplex-trace-tool -- --logs /path/to/logs --port 3011`

4. **Open the UI**  
   Navigate to `http://localhost:3011/` in your browser. Toggle between Standard and Pipelining modes from the top-right control.


## Architecture

```
tool/src/
├── lib.rs              # Library crate exports
├── main.rs             # Binary entry point
├── events/             # Event classification system
├── parser/             # Log parsing
├── state/              # Application state
└── api/                # HTTP API
```

## Adding Custom Events

### Step 1: Define Event Kind

Add your event to `src/events/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum EventKind {
    // ... existing kinds ...
    
    // Your custom event
    MyCustomEvent,
}

impl EventKind {
    pub fn tag(&self) -> u8 {
        match self {
            // Use a unique tag number (check existing ones first)
            Self::MyCustomEvent => 100,
            // ...
        }
    }
    
    pub fn label(&self) -> &'static str {
        match self {
            Self::MyCustomEvent => "My Custom Event",
            // ...
        }
    }
    
    pub fn color(&self) -> &'static str {
        match self {
            Self::MyCustomEvent => "#ff6b6b",  // Pick a color
            // ...
        }
    }
}
```

### Step 2: Create Classifier

Add a classifier function in your category file (e.g., `src/events/consensus.rs`):

```rust
use super::{EventKind, EventCategory};

pub fn classify_my_custom_event(message: &str) -> Option<(EventKind, EventCategory)> {
    if message.contains("my custom pattern") {
        Some((EventKind::MyCustomEvent, EventCategory::Consensus))
    } else {
        None
    }
}
```

### Step 3: Register Classifier

In `src/main.rs` or wherever you build the registry:

```rust
let mut event_registry = EventRegistry::new();
event_registry.register(classify_my_custom_event);
// ... register other classifiers
```

The event will now be parsed, displayed in the UI, and available via the API.

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/events` | Paginated events with filters |
| `GET /api/edges` | Message send/receive pairs |
| `GET /api/views` | Per-view consensus status |
| `GET /api/segments` | Time-chunked segments |
| `GET /api/meta` | Metadata (nodes, event kinds, time range) |

### Query Parameters

**Events:**
- `limit` - Max results (default: 10000)
- `offset` - Pagination offset
- `nodes` - Comma-separated node IDs
- `kinds` - Comma-separated event kind tags
- `categories` - Comma-separated category names
- `view_from`, `view_to` - View range filter

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `←/→` | Pan timeline 10% |
| `Shift+←/→` | Jump to prev/next view |
| `[` / `]` | Zoom out/in |
| `1-7` | Jump to segment |
| `R` | Reset to full view |
| `Esc` | Close details |

## Event Categories

| Category | Description | Tag Range |
|----------|-------------|-----------|
| Consensus | Core consensus events | 1-14 |
| Marshal | Block finalization | 21-23 |
| Batcher | Message batching | 31-32 |
| Broadcast | P2P broadcasting | 41-44 |
| Resolver | Request resolution | 51-52 |
| Network | Network layer | 61-67 |
| Storage | Persistent storage | 81-83 |

## Development

```bash
# Build
cargo build -p simplex-trace-tool

# Run with custom logs directory
cargo run -p simplex-trace-tool -- --logs /path/to/logs --port 3011

# Run tests
cargo test -p simplex-trace-tool

## License

MIT License