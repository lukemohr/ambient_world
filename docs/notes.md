# Ambient World - Real-time Audio Synthesis System

## Overview

This is a real-time ambient audio synthesis system built in Rust that generates evolving drone sounds based on a simulated world state. The system uses modern async Rust patterns, real-time audio processing, and reactive programming to create an immersive audio experience.

## Recent Improvements

### Audio Engine Enhancements

- **Multi-format support**: Automatic detection and conversion for F32, I16, U16 sample formats
- **Lock-free architecture**: Removed Mutex from audio callback for zero-latency parameter updates
- **CPU optimizations**: Phase-in-radians approach eliminates per-sample divisions and multiplications
- **Level management**: Proper gain staging with soft limiting prevents clipping

### API Modernization

- **Type-safe event schema**: Replaced stringly-typed parsing with serde enum derives
- **Async-friendly locking**: RwLock with snapshot task for non-blocking API responses
- **Self-documenting JSON**: Compile-time guaranteed API schema for frontend integration

### Performance Optimizations

- **Real-time safety**: No allocations or blocking operations in audio hot paths
- **Efficient phase management**: Pre-calculated phase increments avoid expensive computations
- **Memory safety**: RAII and ownership system prevent resource leaks

### Future Planning

- **Deterministic mode**: RNG injection architecture for reproducible demos/replay
- **WebSocket streaming**: Real-time state updates for reactive frontends

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

- **World State**: Six normalized parameters (0.0-1.0):
  - `density`: Spatial complexity
  - `rhythm`: Temporal patterns
  - `tension`: Emotional intensity
  - `energy`: Overall activity level
  - `warmth`: Tonal character
  - `sparkle_impulse`: Trigger for sparkle audio events

- **Sparkle System**: Procedural generation of audio sparkle events
  - **Probabilistic Generation**: Sparkles occur based on world rhythm and density
  - **Phase Accumulation**: Uses a sparkle_phase accumulator that advances with each world tick
  - **Threshold Triggering**: When phase exceeds threshold, generates sparkle_impulse
  - **Audio Response**: SparkleLayer creates short noise bursts with attack/decay envelope
  - **Smoothing**: Both world generation and audio processing use smoothing to prevent clicks

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

Manages CPAL (Cross-Platform Audio Library) streams with multi-format support:

```rust
pub struct AudioEngine {
    _stream: Stream,        // Keeps stream alive
    config: StreamConfig,   // Audio configuration
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
4. Select optimal config (f32 format preferred, max sample rate)
5. Build output stream with callback function
6. Start playback: `stream.play()`

**Multi-Format Support**:
The engine automatically detects and supports multiple sample formats:

- **F32**: Native floating-point (-1.0 to 1.0)
- **I16**: 16-bit signed integer (-32768 to 32767)
- **U16**: 16-bit unsigned integer (0 to 65535)

Each format has dedicated processing functions that convert from f32 internally.

**Lock-Free Architecture**:
Audio layers are owned by the callback closure, eliminating Mutex contention:

```rust
let mut layers = vec![drone_layer, texture_layer, sparkle_layer];

// Callback owns layers directly - no locking needed
move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
    Self::process_audio_f32(data, &mut layers, &shared_params, config.channels);
}
```

**Level Management**:
Proper gain staging prevents clipping:

```rust
// Conservative per-layer gains
const DRONE_LAYER_GAIN: f32 = 0.3;
const TEXTURE_LAYER_GAIN: f32 = 0.4;
const SPARKLE_LAYER_GAIN: f32 = 0.6;

// Master gain with soft limiting
let master_gain = params.master_gain.min(1.0);
mixed_sample *= master_gain;

// Soft limiter: 6dB limiting with smooth knee
if mixed_sample.abs() > 0.8 {
    let excess = mixed_sample.abs() - 0.8;
    let compressed = excess * 0.5; // 2:1 ratio
    mixed_sample = mixed_sample.signum() * (0.8 + compressed);
}
```

#### Audio Layers (`layers.rs`)

Modular synthesis components implementing the `Layer` trait:

```rust
pub trait Layer {
    fn process(&mut self, params: &AudioParams) -> f32;
}
```

**DroneLayer**: Dual-oscillator synthesis with tension-based detuning and CPU optimizations.

**Performance Optimizations**:
The DroneLayer uses phase-in-radians for maximum efficiency:

```rust
pub struct DroneLayer {
    phase_a: f32,           // Phase in radians (not sample count)
    phase_b: f32,           // Phase in radians (not sample count)
    phase_incr_a: f32,      // Pre-calculated: 2π * freq_a / sample_rate
    phase_incr_b: f32,      // Pre-calculated: 2π * freq_b / sample_rate
    // ... smoothed parameters
}

// Optimized processing: direct sin() of phase in radians
fn process(&mut self, params: &AudioParams) -> f32 {
    // Update phase increments (avoid per-sample divisions)
    self.phase_incr_a = self.smoothed_base_freq_hz * TWO_PI / self.sample_rate;
    self.phase_incr_b = self.smoothed_base_freq_hz * self.smoothed_detune_ratio * TWO_PI / self.sample_rate;

    // Generate samples (no multiplication in sin() argument)
    let sample_a = self.phase_a.sin();
    let sample_b = self.phase_b.sin();

    // Update phases (increment by pre-calculated radians)
    self.phase_a += self.phase_incr_a;
    self.phase_b += self.phase_incr_b;

    // Wrap at 2π (prevents precision loss)
    if self.phase_a >= TWO_PI { self.phase_a -= TWO_PI; }
    if self.phase_b >= TWO_PI { self.phase_b -= TWO_PI; }

    (sample_a + sample_b) * 0.5
}
```

**Key Optimizations**:

- **Phase-in-radians**: Increment by `2π × f / sr` instead of sample counting
- **Pre-calculated increments**: Avoid per-sample frequency calculations
- **Direct sin() calls**: No complex argument computation per sample
- **Efficient wrapping**: Wrap at 2π instead of division-based wrapping

**TextureLayer**: Provides a subtle noise bed with slow LFO modulation and filtering.

**SparkleLayer**: Generates short, bright noise impulses when sparkle_impulse > 0.

**Sparkle Implementation Details**:

The sparkle system creates natural-sounding audio impulses that occur probabilistically based on world state:

**World-Side Generation** (`ambient_core/src/engine.rs`):

```rust
// In WorldEngine::update_sparkles()
let threshold = 1.0 - (rhythm * density); // Lower threshold = more sparkles
self.sparkle_phase += rhythm * 0.1;       // Phase advances with rhythm

if self.sparkle_phase >= threshold {
    self.sparkle_phase = 0.0;             // Reset phase
    // Generate sparkle_impulse based on density
    let impulse = density * (0.5 + rhythm * 0.5);
    world_state.set_sparkle_impulse(impulse);
}
```

**Audio-Side Processing** (`audio/src/layers.rs`):

```rust
// SparkleLayer envelope: quick attack, slow decay
fn envelope(&self, phase: f32) -> f32 {
    if phase < 0.1 {
        phase * 10.0        // Attack: 0→1 in first 10%
    } else {
        (1.0 - (phase - 0.1) / 0.9).max(0.0)  // Decay: 1→0 over 90%
    }
}

// White noise generation with smoothing
let smoothed_impulse = self.smoothed_sparkle_impulse;
let envelope_value = self.envelope(self.envelope_phase);
let noise_sample = self.noise(); // LCG-generated white noise
let sparkle_sample = noise_sample * envelope_value * smoothed_impulse;
```

**Key Features**:

- **Smoothing**: Prevents clicks by gradually changing sparkle_impulse
- **Envelope Shaping**: 100ms duration with fast attack/slow decay
- **Threshold Triggering**: Only triggers when smoothed impulse crosses threshold
- **Noise Generation**: LCG-based white noise for brightness
- **Finite Checking**: Guards against NaN/inf values in real-time audio

**Texture Implementation Details**:

The texture layer provides a subtle noise bed that responds to world state parameters:

**Parameter Mapping** (`audio/src/params.rs`):

```rust
// World state to audio parameters
texture: (density * 0.3).clamp(0.0, 1.0),         // density -> texture gain
brightness: (1.0 - warmth * 0.5).clamp(0.0, 1.0), // warmth inverse -> brightness
detune_ratio: (1.0 + tension * 0.01).clamp(0.5, 2.0), // tension -> detune
motion: (rhythm * 0.5).clamp(0.0, 1.0),           // energy -> motion (LFO amount)
```

**Audio Processing** (`audio/src/layers.rs`):

```rust
// Generate noise with tension-based roughness
let noise_sample = self.noise(self.smoothed_tension); // LCG + cubic distortion

// Apply warmth-based filtering (low-pass for "darker" sound)
let warmth_cutoff = 1.0 - self.smoothed_warmth;
let filtered_sample = self.filter(noise_sample, warmth_cutoff * 0.3);

// Slow LFO modulation (0.01-0.1 Hz triangle wave)
let lfo_value = self.lfo(self.smoothed_energy);
let modulated_sample = filtered_sample * (0.3 + 0.7 * lfo_value);

// Apply density-based gain (very loud for testing)
let texture_sample = modulated_sample * self.smoothed_density * 2.0;
```

**Key Features**:

- **LFO Modulation**: Very slow triangle wave (0.01-0.1 Hz) for organic movement
- **Filtering**: Simple low-pass filter controlled by warmth for tonal shaping
- **Roughness**: Tension adds cubic distortion to noise for harsher texture
- **Gain Level**: High level (2.0x) for testing audibility
- **Slow Smoothing**: 0.005 coefficient for gradual parameter changes

#### Parameter System (`params.rs`)

Thread-safe real-time parameter updates using atomics.

**Atomic Parameters**:

```rust
#[derive(Debug)]
pub struct SharedAudioParams {
    master_gain: AtomicU32,     // f32 stored as bits
    base_freq_hz: AtomicU32,
    detune_ratio: AtomicU32,
    brightness: AtomicU32,
    motion: AtomicU32,
    texture: AtomicU32,
    sparkle_impulse: AtomicU32,
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

**Parameter Mapping** (`AudioParams::from_world_state`):

World state parameters are mapped to audio synthesis parameters:

```rust
master_gain: (energy * 0.2).clamp(0.0, 1.0),        // energy -> gain
base_freq_hz: (80.0 + warmth * 160.0).clamp(80.0, 240.0), // warmth -> freq
detune_ratio: (1.0 + tension * 0.01).clamp(0.5, 2.0),     // tension -> detune
brightness: (1.0 - warmth * 0.5).clamp(0.0, 1.0),   // warmth inverse
motion: (rhythm * 0.5).clamp(0.0, 1.0),             // rhythm -> motion
texture: (density * 0.3).clamp(0.0, 1.0),           // density -> texture
sparkle_impulse: sparkle_impulse,                   // direct pass-through
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

REST endpoints using Axum framework with type-safe event schema:

- `GET /health` - System status
- `GET /state` - Current world snapshot
- `POST /event` - Trigger world events

**Type-Safe Event Schema**:
Replaced stringly-typed parsing with proper Rust enums and serde derives:

```rust
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EventRequest {
    #[serde(rename = "trigger")]
    Trigger {
        kind: TriggerKind,  // Proper enum, not string
        #[serde(default = "default_intensity")]
        intensity: f64,
    },
    #[serde(rename = "perform")]
    Perform(PerformAction),  // Proper enum, not string
}

// Handler uses direct enum matching (no string parsing)
async fn event(
    State(app_state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> impl IntoResponse {
    let event = match req {
        EventRequest::Trigger { kind, intensity } => Event::Trigger { kind, intensity },
        EventRequest::Perform(action) => Event::Perform(action),
    };
    // ... send event
}
```

**Benefits**:

- **Compile-time safety**: No typos in event types/kinds
- **Self-documenting**: JSON schema generated from code
- **Frontend integration**: TypeScript types can be derived
- **Better errors**: Serde provides clear validation messages

**Async Architecture**:
Uses RwLock with snapshot task for non-blocking API responses:

```rust
// Snapshot task keeps current state updated
pub async fn start_snapshot_task(
    mut state_rx: watch::Receiver<WorldSnapshot>,
    current_snapshot: Arc<RwLock<WorldSnapshot>>,
) {
    loop {
        if state_rx.changed().await.is_err() { break; }
        let snapshot = state_rx.borrow().clone();
        *current_snapshot.write().await = snapshot;
    }
}

// API handlers read without blocking
async fn get_state(State(app_state): State<AppState>) -> impl IntoResponse {
    let snapshot = app_state.current_snapshot.read().await.clone();
    Json(snapshot)
}
```

**Axum Routing**:

```rust
pub fn create_router(
    event_tx: mpsc::Sender<Event>,
    current_snapshot: Arc<RwLock<WorldSnapshot>>,
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
- **Multi-format support**: Automatic format detection and conversion
- **Lock-free layers**: Callback owns layers directly (no Mutex)

### CPU Optimizations

**DroneLayer Efficiency**:

- **Phase-in-radians**: Increment by `2π × f / sr` (prevents per-sample multiplications)
- **Pre-calculated increments**: Avoid division in hot path
- **Direct sin() calls**: No complex argument computation
- **Efficient wrapping**: Wrap at 2π instead of `sample_rate / freq`

**Before (inefficient)**:

```rust
// Per-sample: sin(phase * freq * 2π / sample_rate)
// Per-sample: phase += 1.0; if phase >= sample_rate / freq { ... }
```

**After (optimized)**:

```rust
// Pre-calculate: phase_incr = freq * 2π / sample_rate
// Per-sample: sin(phase); phase += phase_incr; if phase >= 2π { ... }
```

### Async Efficiency

- **Task spawning**: Independent concurrency
- **Channel selection**: Appropriate semantics (mpsc vs watch)
- **Watch channels**: Skip intermediate values for latest-only
- **RwLock snapshots**: Non-blocking reads for API responses

### Memory Safety

- **RAII**: Automatic resource management
- **Ownership system**: Compile-time guarantees
- **Arc/RwLock**: Thread-safe sharing where needed
- **No allocations**: In real-time audio paths

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

# Trigger events (type-safe enum-based JSON)
curl -X POST http://localhost:3000/event \
  -H "Content-Type: application/json" \
  -d '{"type": "trigger", "kind": "Pulse", "intensity": 0.8}'

curl -X POST http://localhost:3000/event \
  -H "Content-Type: application/json" \
  -d '{"type": "perform", "Scene": {"name": "sunrise"}}'

curl -X POST http://localhost:3000/event \
  -H "Content-Type: application/json" \
  -d '{"type": "perform", "Freeze": {"seconds": 5.0}}'
```

**JSON Schema Examples**:

**Trigger Events**:

```json
{"type": "trigger", "kind": "Pulse", "intensity": 0.8}
{"type": "trigger", "kind": "Calm", "intensity": 0.3}
{"type": "trigger", "kind": "Heat"}
```

**Perform Actions**:

```json
{"type": "perform", "Pulse": {"intensity": 0.9}}
{"type": "perform", "Scene": {"name": "sunrise"}}
{"type": "perform", "Freeze": {"seconds": 5.0}}
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
- **Deterministic mode**: Seeded RNG for demos/replay (planned)

### API Features

- **WebSocket streaming**: Real-time state updates
- **Batch operations**: Multiple events
- **Configuration**: Runtime parameter adjustment
- **Metrics**: Performance monitoring
- **Type safety**: Enum-based schema prevents UI bugs

This architecture provides a solid foundation for real-time, reactive audio applications with clean separation of concerns and modern Rust patterns.
