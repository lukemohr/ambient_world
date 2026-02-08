# Ambient World - Real-time Audio Synthesis System

## Overview

This is a real-time ambient audio synthesis system built in Rust that generates evolving drone sounds based on a simulated world state. The system uses modern async Rust patterns, real-time audio processing, and reactive programming to create an immersive audio experience.

## Architecture

The project is organized as a Cargo workspace with three main crates:

```
ambient-world/
├── crates/
│   ├── ambient_core/    # World state simulation
│   ├── audio/          # Real-time audio synthesis
│   └── app/            # Application orchestration
├── docs/
└── Cargo.toml          # Workspace configuration
```

## Crates Overview

### 1. `ambient_core` - World State Engine

**Purpose**: Manages the evolving world state that drives audio parameters.

**Key Files**:

- `src/lib.rs` - Library exports
- `src/world.rs` - Core world state types and logic
- `src/events.rs` - Event definitions for world interactions
- `src/engine.rs` - World state update engine

**Key Concepts**:

- **World State**: Five normalized parameters (0.0-1.0):
  - `density`: Spatial complexity
  - `rhythm`: Temporal patterns
  - `tension`: Emotional intensity
  - `energy`: Overall activity level
  - `warmth`: Tonal character

- **Event System**: Triggers that modify world state:
  - `Tick`: Regular time-based evolution
  - `Pulse`: Energy injection
  - `Stir`: Density increase
  - `Calm`: Tension reduction
  - `Heat`: Warmth and energy boost
  - `Tense`: Direct tension increase

**Advanced Rust Features**:

- **Trait Objects**: `WorldEngine` uses dynamic dispatch for extensibility
- **Builder Pattern**: Clean API construction
- **Comprehensive Testing**: Unit tests with property-based testing concepts

### 2. `audio` - Real-time Audio Synthesis

**Purpose**: Low-latency audio generation using system audio APIs.

**Key Files**:

- `src/lib.rs` - Library exports
- `src/engine.rs` - CPAL audio stream management
- `src/layers.rs` - Audio synthesis algorithms
- `src/params.rs` - Thread-safe parameter sharing

**Key Components**:

#### Audio Engine (`engine.rs`)

Manages CPAL (Cross-Platform Audio Library) streams:

```rust
pub struct AudioEngine {
    _stream: Stream,        // Keeps stream alive
    config: StreamConfig,   // Audio configuration
    layer: Arc<Mutex<DroneLayer>>, // Synthesis layer
}
```

**CPAL Deep Dive**:
CPAL is Rust's primary cross-platform audio I/O library. Key concepts:

- **Hosts**: Audio system backends (CoreAudio, WASAPI, ALSA, etc.)
- **Devices**: Physical/virtual audio interfaces
- **Streams**: Real-time audio data flow
- **Callbacks**: Low-latency audio processing functions

**Stream Setup Process**:

1. Get default host: `cpal::default_host()`
2. Find output device: `host.default_output_device()`
3. Query supported configs: `device.supported_output_configs()`
4. Select optimal config (f32 format, max sample rate)
5. Build output stream with callback function
6. Start playback: `stream.play()`

#### Audio Layers (`layers.rs`)

Modular synthesis components implementing the `Layer` trait:

```rust
pub trait Layer {
    fn process(&mut self, params: &AudioParams) -> f32;
}
```

**DroneLayer**: Dual-oscillator synthesis with tension-based detuning.

#### Parameter System (`params.rs`)

Thread-safe real-time parameter updates using atomics.

**Atomic Parameters**:

```rust
#[derive(Debug)]
pub struct SharedAudioParams {
    master_gain: AtomicU32,     // f32 stored as bits
    base_freq_hz: AtomicU32,
    detune_ratio: AtomicU32,
    // ... more fields
}
```

**Why Atomics?**

- **Lock-free**: No mutex contention in audio callback
- **Real-time safe**: No allocation or blocking operations
- **Memory model**: Relaxed ordering for performance

**f32 Atomic Storage**:

```rust
// Store: convert f32 to u32 bits
self.field.store(value.to_bits(), Ordering::Relaxed)

// Load: convert u32 bits back to f32
f32::from_bits(self.field.load(Ordering::Relaxed))
```

### 3. `app` - Application Orchestration

**Purpose**: Coordinates all subsystems and provides HTTP API.

**Key Files**:

- `src/main.rs` - Application entry point
- `src/api.rs` - HTTP endpoints
- `src/runtime.rs` - Async task management

**Key Components**:

#### Main Application (`main.rs`)

Orchestrates the entire system:

1. **Configuration**: Environment-based settings
2. **Channel Setup**: mpsc for events, watch for state
3. **Audio Initialization**: Early startup for immediate sound
4. **Task Spawning**: Concurrent execution of subsystems
5. **API Server**: HTTP interface for external control

#### Runtime Tasks (`runtime.rs`)

**World Task** (`start_world_task`):

- Processes events from mpsc channel
- Updates world state via `WorldEngine`
- Publishes snapshots to watch channel

**Tick Task** (`start_tick_task`):

- Generates regular `Event::Tick` messages
- Configurable frequency (Hz)
- Independent timing loop

**Audio Control Task** (`start_audio_control_task`):

- Subscribes to world state changes
- Maps world parameters to audio parameters
- Updates `SharedAudioParams` atomically

#### HTTP API (`api.rs`)

REST endpoints using Axum framework:

- `GET /health` - System status
- `GET /state` - Current world snapshot
- `POST /event` - Trigger world events

**Axum Routing**:

```rust
pub fn create_router(
    event_tx: mpsc::Sender<Event>,
    state_rx: watch::Receiver<WorldSnapshot>
) -> Router
```

## Key Technologies

### Tokio - Async Runtime

**Why Tokio?**

- **Performance**: Work-stealing scheduler
- **Ecosystem**: Most popular async runtime
- **Features**: Timers, channels, I/O, synchronization

**Channel Types Used**:

1. **mpsc (Multi-Producer, Single-Consumer)**:
   - Events from API → World task
   - Multiple senders, one receiver
   - Unbounded capacity for reliability

2. **watch (Single-Producer, Multi-Consumer)**:
   - World state distribution
   - Latest value semantics
   - Efficient for real-time updates

**Task Management**:

```rust
// Spawn independent tasks
tokio::spawn(start_world_task(event_rx, state_tx));
tokio::spawn(start_tick_task(event_tx.clone(), hz));

// Graceful shutdown
tokio::signal::ctrl_c().await?;
```

### Tracing - Structured Logging

**Why Tracing?**

- **Performance**: Zero-cost disabled logging
- **Structured**: Key-value metadata
- **Filtering**: Runtime log level control
- **Ecosystem**: Integrates with monitoring tools

**Usage Patterns**:

```rust
// Info logging with context
info!("Audio engine started");

// Structured events
info!(
    "State: density={:.3}, rhythm={:.3}, tension={:.3}",
    density, rhythm, tension
);
```

**Subscriber Setup**:

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    )
    .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
    .init();
```

### Serde - Serialization

**Why Serde?**

- **Performance**: Zero-copy where possible
- **Ecosystem**: Supports JSON, TOML, YAML, etc.
- **Derive Macros**: Automatic implementation

**API Serialization**:

```rust
#[derive(serde::Serialize)]
pub struct WorldSnapshot {
    density: f64,
    rhythm: f64,
    // ... other fields
}
```

### Advanced Rust Features

#### 1. Async/Await

**Zero-cost abstraction** for concurrent programming:

- **Futures**: Lazy computation representation
- **Await points**: Suspension/resumption points
- **Colorless functions**: Async functions work in sync contexts

#### 2. Smart Pointers

**Memory safety with performance**:

- **Arc**: Atomic reference counting for shared ownership
- **Mutex**: Interior mutability for shared state
- **Combined**: `Arc<Mutex<T>>` for thread-safe shared mutation

#### 3. Trait Objects

**Dynamic dispatch** for extensibility:

```rust
pub trait Layer {
    fn process(&mut self, params: &AudioParams) -> f32;
}
```

#### 4. Atomic Operations

**Lock-free concurrency**:

- **Memory ordering**: Relaxed for performance
- **Type punning**: f32 ↔ u32 bit representation
- **Real-time safety**: No allocation in hot paths

#### 5. Closure Captures

**Anonymous functions** with captured environment:

```rust
let stream = device.build_output_stream(
    &config,
    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // Captures layer_clone, config.channels, shared_params
        Self::process_audio(data, &mut layer, &shared_params, config.channels);
    },
    // ... error handler
    None,
)?;
```

#### 6. Error Handling

**Comprehensive error management**:

- **anyhow**: Ergonomic error handling
- **thiserror**: Library error definitions
- **? operator**: Early return on error

#### 7. Iterator Patterns

**Functional programming**:

```rust
let supported_config = device
    .supported_output_configs()?
    .find(|config| config.sample_format() == SampleFormat::F32)
    .ok_or_else(|| anyhow::anyhow!("No f32 output config found"))?;
```

## Data Flow

```
HTTP API → Event Channel → World Task → State Updates
    ↓              ↓              ↓
Triggers   Event Processing   WorldEngine.apply()
    ↓              ↓              ↓
JSON       mpsc::Sender       WorldState mutation
    ↓              ↓              ↓
Response   mpsc::Receiver     watch::Sender.broadcast()

State Updates → Audio Control Task → Parameter Mapping
    ↓              ↓              ↓
watch::Receiver   World→Audio     SharedAudioParams
    ↓              ↓              ↓
.changed().await  from_world_state()  Atomic updates

Parameter Updates → Audio Callback → Synthesis
    ↓              ↓              ↓
SharedAudioParams  .get() atomic    Layer.process()
    ↓              ↓              ↓
Real-time values   Non-blocking      Sample generation
    ↓              ↓              ↓
CPAL Buffer       f32 samples        System audio
```

## Performance Considerations

### Real-time Audio

- **Buffer sizes**: Minimize latency (typically 128-512 samples)
- **Atomic parameters**: Lock-free updates
- **No allocation**: In audio callback hot path
- **Smoothing**: Prevent parameter discontinuities

### Async Efficiency

- **Task spawning**: Independent concurrency
- **Channel selection**: Appropriate semantics (mpsc vs watch)
- **Watch channels**: Skip intermediate values for latest-only

### Memory Safety

- **RAII**: Automatic resource management
- **Ownership system**: Compile-time guarantees
- **Arc/Mutex**: Thread-safe sharing where needed

## Development Workflow

### Building

```bash
cargo build                    # Debug build
cargo build --release         # Optimized build
cargo check                   # Compilation check only
```

### Testing

```bash
cargo test                     # Run all tests
cargo test -p ambient_core    # Test specific crate
```

### Running

```bash
cargo run -p app              # Start the application
# API available at http://localhost:3000
```

### API Usage

```bash
# Check health
curl http://localhost:3000/health

# Get current state
curl http://localhost:3000/state

# Trigger events
curl -X POST http://localhost:3000/event \
  -H "Content-Type: application/json" \
  -d '{"kind": "Pulse", "intensity": 0.5}'
```

## Future Extensions

### Audio Enhancements

- **Multiple layers**: Texture, motion, and additional drones
- **Effects processing**: Reverb, filtering, modulation
- **Spatial audio**: 3D positioning
- **Dynamic reconfiguration**: Runtime sample rate changes

### World Complexity

- **Multiple worlds**: Parallel simulations
- **Complex events**: Chained reactions
- **External inputs**: Sensors, network data
- **Persistence**: Save/load world states

### API Features

- **WebSocket streaming**: Real-time state updates
- **Batch operations**: Multiple events
- **Configuration**: Runtime parameter adjustment
- **Metrics**: Performance monitoring

This architecture provides a solid foundation for real-time, reactive audio applications with clean separation of concerns and modern Rust patterns.
