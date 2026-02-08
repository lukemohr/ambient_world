mod api;
mod runtime;

use crate::runtime::{start_tick_task, start_world_task};
use ambient_core::world::{WorldSnapshot, WorldState};
use axum::serve;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};
use tokio::time::interval;
use tracing::info;

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
    let (state_tx, state_rx) = watch::channel(WorldSnapshot::from_world_state(&initial_state));

    // Default tick rate
    let tick_hz = config.tick_hz;
    info!("Tick rate: {:.0} Hz", tick_hz);

    // Spawn tasks
    tokio::spawn(start_world_task(event_rx, state_tx));
    tokio::spawn(start_tick_task(event_tx.clone(), tick_hz));

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
    let app = api::create_router(event_tx, Arc::new(Mutex::new(state_rx)));
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
