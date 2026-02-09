use ambient_core::engine::WorldEngine;
use ambient_core::events::Event;
use ambient_core::world::WorldSnapshot;
use audio::params::{AudioParams, SharedAudioParams};
use std::sync::Arc;
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
/// - Runs at the specified frequency (Hz).
/// - Computes the time delta (dt) since the last tick.
/// - Sends Event::Tick to the event channel.
/// - Keeps running separately to avoid blocking the world task.
pub async fn start_tick_task(
    event_tx: mpsc::Sender<Event>,
    hz: f64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_secs = 1.0 / hz;
    let mut interval = interval(Duration::from_secs_f64(interval_secs));
    let mut last_time = Instant::now();
    info!(
        "Tick task started with frequency {:.2} Hz (interval {:.3}s)",
        hz, interval_secs
    );

    loop {
        interval.tick().await;
        let now = Instant::now();
        let dt = now.duration_since(last_time).as_secs_f64();
        last_time = now;

        let event = Event::Tick { dt };
        if event_tx.send(event).await.is_err() {
            info!("Event channel closed, stopping tick task");
            break;
        }
    }

    Ok(())
}

/// Starts the audio control task that maps world state to audio parameters.
///
/// This task:
/// - Subscribes to world state snapshots.
/// - Computes audio parameters from the latest snapshot.
/// - Updates the shared audio parameters for real-time control.
/// - Sends updates to the audio params watch channel for WebSocket clients.
/// - Runs continuously, updating whenever the world state changes.
pub async fn start_audio_control_task(
    mut state_rx: watch::Receiver<WorldSnapshot>,
    shared_audio_params: Arc<SharedAudioParams>,
    audio_params_tx: watch::Sender<AudioParams>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Audio control task started");

    loop {
        // Wait for a new snapshot
        if state_rx.changed().await.is_err() {
            info!("State channel closed, stopping audio control task");
            break;
        }

        // Get the latest snapshot
        let snapshot = state_rx.borrow();

        // Compute audio params from world state
        let audio_params = AudioParams::from_world_state(
            snapshot.density() as f32,
            snapshot.rhythm() as f32,
            snapshot.tension() as f32,
            snapshot.energy() as f32,
            snapshot.warmth() as f32,
            snapshot.sparkle_impulse() as f32,
        );

        // Update shared audio params (atomic, non-blocking)
        shared_audio_params.set(audio_params);

        // Send to watch channel for WebSocket clients
        let _ = audio_params_tx.send(audio_params);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_tick_task_sends_events() {
        let (event_tx, mut event_rx) = mpsc::channel(10);
        let hz = 10.0; // 10 Hz for faster testing
        let handle = tokio::spawn(start_tick_task(event_tx, hz));

        // Wait for a few ticks
        let mut count = 0;
        while count < 3 {
            match timeout(Duration::from_millis(200), event_rx.recv()).await {
                Ok(Some(Event::Tick { dt })) => {
                    assert!(dt > 0.0 && dt < 0.2); // dt should be around 0.1s
                    count += 1;
                }
                _ => break,
            }
        }

        // Forcibly stop the task to avoid hanging
        handle.abort();
        let _ = handle.await;
        assert_eq!(count, 3);
    }
}
