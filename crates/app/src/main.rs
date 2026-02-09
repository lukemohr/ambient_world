mod api;
mod runtime;

use crate::runtime::{start_audio_control_task, start_tick_task, start_world_task};
use ambient_core::world::{WorldSnapshot, WorldState};
use audio::engine::AudioEngine;
use audio::params::{AudioParams, SharedAudioParams};
use axum::serve;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, mpsc, watch};
use tokio::time::interval;
use tracing::{info, warn};

#[derive(Debug)]
struct Config {
    tick_hz: f64,
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tick_hz: 20.0,
            port: 3000,
        }
    }
}

impl Config {
    fn from_env() -> Self {
        let tick_hz = std::env::var("TICK_HZ")
            .unwrap_or_else(|_| "20.0".to_string())
            .parse()
            .unwrap_or(20.0);
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .unwrap_or(3000);
        Self { tick_hz, port }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup tracing with timestamped logs
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .init();

    info!("Starting...");

    let config = Config::from_env();

    // Create channels
    let (event_tx, event_rx) = mpsc::channel(100);
    let initial_state = WorldState::new();
    let initial_snapshot = WorldSnapshot::from_world_state(&initial_state);
    let (state_tx, state_rx) = watch::channel(initial_snapshot.clone());

    // Create initial audio params from initial world snapshot
    let initial_audio_params = AudioParams::from_world_state(
        initial_snapshot.density() as f32,
        initial_snapshot.rhythm() as f32,
        initial_snapshot.tension() as f32,
        initial_snapshot.energy() as f32,
        initial_snapshot.warmth() as f32,
        initial_snapshot.sparkle_impulse() as f32,
    );
    let shared_audio_params = Arc::new(SharedAudioParams::new(initial_audio_params));
    let (audio_params_tx, audio_params_rx) = watch::channel(initial_audio_params);

    // Start audio engine early (with error handling)
    let audio_params_clone = Arc::clone(&shared_audio_params);
    let audio_engine_result = AudioEngine::start(audio_params_clone);
    let _audio_engine = match audio_engine_result {
        Ok(engine) => {
            info!("Audio engine started successfully");
            Some(engine)
        }
        Err(e) => {
            warn!(
                "Audio engine failed to start ({}), continuing without audio output",
                e
            );
            None
        }
    };

    // Default tick rate
    let tick_hz = config.tick_hz;
    info!("Tick rate: {:.0} Hz", tick_hz);

    // Spawn tasks
    tokio::spawn(start_world_task(event_rx, state_tx));
    tokio::spawn(start_tick_task(event_tx.clone(), tick_hz));

    // Start audio control task
    let state_rx_for_audio = state_rx.clone();
    let audio_params_for_control = Arc::clone(&shared_audio_params);
    let audio_params_tx_for_control = audio_params_tx.clone();
    tokio::spawn(start_audio_control_task(
        state_rx_for_audio,
        audio_params_for_control,
        audio_params_tx_for_control,
    ));

    // State logger task: log snapshot every 1 second
    let state_rx_clone = state_rx.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let borrowed = state_rx_clone.borrow();
            info!(
                "State: density={:.3}, rhythm={:.3}, tension={:.3}, energy={:.3}, warmth={:.3}",
                borrowed.density(),
                borrowed.rhythm(),
                borrowed.tension(),
                borrowed.energy(),
                borrowed.warmth()
            );
        }
    });

    // Start API server
    // Create shared snapshot for API handlers
    let initial_snapshot = state_rx.borrow().clone();
    let current_snapshot = Arc::new(RwLock::new(initial_snapshot));

    // Start snapshot task to keep API snapshot updated
    let state_rx_for_api = state_rx.clone();
    let current_snapshot_for_task = Arc::clone(&current_snapshot);
    tokio::spawn(api::start_snapshot_task(
        state_rx_for_api,
        current_snapshot_for_task,
    ));

    let app = api::create_router(event_tx, current_snapshot, state_rx, audio_params_rx);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("API server listening on http://localhost:{}", config.port);
    tokio::spawn(async move {
        serve(listener, app).await.unwrap();
    });

    // Keep the main task alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    Ok(())
}
