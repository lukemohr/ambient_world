use crate::core::engine::WorldEngine;
use crate::core::events::Event;
use crate::core::world::WorldSnapshot;
use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant, interval};
use tracing::info;

/// Starts the world task that processes events and sends state snapshots.
///
/// This task:
/// - Receives events from the event channel.
/// - Applies them to the WorldEngine.
/// - Sends updated snapshots to the state channel.
/// - Exits gracefully if the event channel closes.
pub async fn start_world_task(
    mut event_rx: mpsc::Receiver<Event>,
    state_tx: watch::Sender<WorldSnapshot>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut engine = WorldEngine::new();
    info!("World task started");

    loop {
        match event_rx.recv().await {
            Some(event) => {
                engine.apply(event);
                let snapshot = engine.get_snapshot();
                state_tx.send(snapshot)?;
            }
            None => {
                info!("Event channel closed, exiting world task");
                break;
            }
        }
    }

    Ok(())
}

/// Starts the tick sender task that periodically sends Tick events.
///
/// This task:
/// - Runs on a fixed interval.
/// - Computes the time delta (dt) since the last tick.
/// - Sends Event::Tick to the event channel.
/// - Keeps running separately to avoid blocking the world task.
pub async fn start_tick_task(
    event_tx: mpsc::Sender<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut interval = interval(Duration::from_millis(100)); // Adjust interval as needed
    let mut last_time = Instant::now();
    info!("Tick task started");

    loop {
        interval.tick().await;
        let now = Instant::now();
        let dt = now.duration_since(last_time).as_secs_f64();
        last_time = now;

        let event = Event::Tick { dt };
        if let Err(_) = event_tx.send(event).await {
            info!("Event channel closed, stopping tick task");
            break;
        }
    }

    Ok(())
}
