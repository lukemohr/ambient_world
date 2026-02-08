use ambient_core::events::{Event, PerformAction, TriggerKind};
use ambient_core::world::WorldSnapshot;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, watch};

/// Task that keeps the current snapshot updated from the watch channel.
/// This allows async handlers to read the latest snapshot without blocking.
pub async fn start_snapshot_task(
    mut state_rx: watch::Receiver<WorldSnapshot>,
    current_snapshot: Arc<RwLock<WorldSnapshot>>,
) {
    loop {
        // Wait for a new snapshot from the world
        if state_rx.changed().await.is_err() {
            // Channel closed, exit
            break;
        }

        // Update our shared snapshot
        let snapshot = state_rx.borrow().clone();
        *current_snapshot.write().await = snapshot;
    }
}

#[derive(Clone)]
pub struct AppState {
    pub event_tx: mpsc::Sender<Event>,
    pub current_snapshot: Arc<RwLock<WorldSnapshot>>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EventRequest {
    #[serde(rename = "trigger")]
    Trigger {
        kind: TriggerKind,
        #[serde(default = "default_intensity")]
        intensity: f64,
    },
    #[serde(rename = "perform")]
    Perform(PerformAction),
}

fn default_intensity() -> f64 {
    0.5
}

pub fn create_router(
    event_tx: mpsc::Sender<Event>,
    current_snapshot: Arc<RwLock<WorldSnapshot>>,
) -> Router {
    let state = AppState {
        event_tx,
        current_snapshot,
    };
    Router::new()
        .route("/health", get(health))
        .route("/state", get(get_state))
        .route("/event", post(event))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    "ok"
}

#[axum::debug_handler]
async fn get_state(State(app_state): State<AppState>) -> impl IntoResponse {
    let snapshot = app_state.current_snapshot.read().await.clone();
    Json(snapshot)
}

async fn event(
    State(app_state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> impl IntoResponse {
    let event = match req {
        EventRequest::Trigger { kind, intensity } => Event::Trigger { kind, intensity },
        EventRequest::Perform(action) => Event::Perform(action),
    };

    match app_state.event_tx.send(event).await {
        Ok(_) => (StatusCode::OK, "Event sent").into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to send event: channel closed",
        )
            .into_response(),
    }
}
