use ambient_core::events::{Event, TriggerKind};
use ambient_core::world::WorldSnapshot;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct AppState {
    pub event_tx: mpsc::Sender<Event>,
    pub state_rx: Arc<Mutex<watch::Receiver<WorldSnapshot>>>,
}

#[derive(Deserialize)]
pub struct EventRequest {
    kind: String,
    intensity: f64,
}

pub fn create_router(
    event_tx: mpsc::Sender<Event>,
    state_rx: Arc<Mutex<watch::Receiver<WorldSnapshot>>>,
) -> Router {
    let state = AppState { event_tx, state_rx };
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
    let receiver = app_state.state_rx.lock().unwrap();
    let snapshot = receiver.borrow().clone();
    Json(snapshot)
}

async fn event(
    State(app_state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> impl IntoResponse {
    let kind = match req.kind.as_str() {
        "Pulse" => TriggerKind::Pulse,
        "Stir" => TriggerKind::Stir,
        "Calm" => TriggerKind::Calm,
        "Heat" => TriggerKind::Heat,
        "Tense" => TriggerKind::Tense,
        _ => return (StatusCode::BAD_REQUEST, "Invalid trigger kind").into_response(),
    };

    let event = Event::Trigger {
        kind,
        intensity: req.intensity,
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
