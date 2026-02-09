use ambient_core::events::{Event, PerformAction, TriggerKind};
use ambient_core::world::WorldSnapshot;
use audio::params::AudioParams;
use axum::extract::ws::{Message, WebSocket};
use axum::{
    Json, Router,
    extract::{State, WebSocketUpgrade},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, watch};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::cors::{Any, CorsLayer};

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
    pub world_state_rx: watch::Receiver<WorldSnapshot>,
    pub audio_params_rx: watch::Receiver<AudioParams>,
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

// WebSocket message types
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerMessage {
    #[serde(rename = "hello")]
    Hello {
        version: String,
        payload: HelloPayload,
    },
    #[serde(rename = "snapshot")]
    Snapshot {
        version: String,
        payload: SnapshotPayload,
    },
    #[serde(rename = "event_ack")]
    EventAck {
        version: String,
        payload: EventAckPayload,
    },
    #[serde(rename = "error")]
    Error {
        version: String,
        payload: ErrorPayload,
    },
}

#[derive(Serialize)]
pub struct HelloPayload {
    pub session_id: String,
    pub schema_version: String,
    pub tick_rate_hz: f64,
}

#[derive(Serialize)]
pub struct SnapshotPayload {
    pub world: WorldSnapshot,
    pub audio: AudioParamsSnapshot,
}

#[derive(Serialize)]
pub struct EventAckPayload {
    pub request_id: Option<String>,
    pub action: String,
    pub intensity: Option<f64>,
}

#[derive(Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
}

#[derive(Deserialize)]
pub struct PerformPayload {
    pub request_id: Option<String>,
    pub action: PerformAction,
}

#[derive(Deserialize)]
pub struct SetScenePayload {
    pub request_id: Option<String>,
    pub scene_name: String,
}

#[derive(Deserialize)]
pub struct PingPayload {
    pub timestamp: f64,
}

#[derive(Serialize)]
pub struct AudioParamsSnapshot {
    pub master_gain: f32,
    pub base_freq_hz: f32,
    pub detune_ratio: f32,
    pub brightness: f32,
    pub motion: f32,
    pub texture: f32,
    pub sparkle_impulse: f32,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[allow(unused)]
pub enum ClientMessage {
    #[serde(rename = "perform")]
    Perform {
        version: String,
        payload: PerformPayload,
    },
    #[serde(rename = "ping")]
    Ping {
        version: String,
        payload: PingPayload,
    },
    #[serde(rename = "set_scene")]
    SetScene {
        version: String,
        payload: SetScenePayload,
    },
}

/// Validates a PerformAction and returns an error message if invalid
fn validate_perform_action(action: &PerformAction) -> Result<(), String> {
    match action {
        PerformAction::Pulse { intensity }
        | PerformAction::Stir { intensity }
        | PerformAction::Calm { intensity }
        | PerformAction::Heat { intensity }
        | PerformAction::Tense { intensity } => {
            if *intensity < 0.0 || *intensity > 1.0 {
                return Err(format!(
                    "Intensity must be between 0.0 and 1.0, got {}",
                    intensity
                ));
            }
        }
        PerformAction::Scene { name } => {
            if name.trim().is_empty() {
                return Err("Scene name cannot be empty".to_string());
            }
            if name.len() > 100 {
                return Err("Scene name too long (max 100 characters)".to_string());
            }
        }
        PerformAction::Freeze { seconds } => {
            if *seconds < 0.0 {
                return Err(format!(
                    "Freeze seconds must be non-negative, got {}",
                    seconds
                ));
            }
            if *seconds > 300.0 {
                return Err(format!(
                    "Freeze seconds too long (max 300 seconds), got {}",
                    seconds
                ));
            }
        }
    }
    Ok(())
}

fn default_intensity() -> f64 {
    0.5
}

/// Helper function to extract action name and intensity from PerformAction
fn get_action_info(action: &PerformAction) -> (&str, Option<f64>) {
    match action {
        PerformAction::Pulse { intensity } => ("Pulse", Some(*intensity)),
        PerformAction::Stir { intensity } => ("Stir", Some(*intensity)),
        PerformAction::Calm { intensity } => ("Calm", Some(*intensity)),
        PerformAction::Heat { intensity } => ("Heat", Some(*intensity)),
        PerformAction::Tense { intensity } => ("Tense", Some(*intensity)),
        PerformAction::Scene { .. } => ("Scene", None),
        PerformAction::Freeze { .. } => ("Freeze", None),
    }
}

pub fn create_router(
    event_tx: mpsc::Sender<Event>,
    current_snapshot: Arc<RwLock<WorldSnapshot>>,
    world_state_rx: watch::Receiver<WorldSnapshot>,
    audio_params_rx: watch::Receiver<AudioParams>,
) -> Router {
    let state = AppState {
        event_tx,
        current_snapshot,
        world_state_rx,
        audio_params_rx,
    };

    // Configure CORS for development (allows UI on localhost:5173)
    let cors = CorsLayer::new()
        .allow_origin(Any) // Allow any origin for development
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/state", get(get_state))
        .route("/event", post(event))
        .route("/ws", get(websocket_handler))
        .with_state(state)
        .layer(cors)
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

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: AppState) {
    let (mut sender, receiver) = socket.split();
    let (tx, rx) = mpsc::unbounded_channel();

    // Generate session ID
    let session_id = format!(
        "ws-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    // Send hello message immediately
    let hello = ServerMessage::Hello {
        version: "1.0".to_string(),
        payload: HelloPayload {
            session_id: session_id.clone(),
            schema_version: "1.0".to_string(),
            tick_rate_hz: 20.0, // From main.rs default
        },
    };

    if let Ok(json) = serde_json::to_string(&hello) {
        let _ = tx.send(Message::Text(json.into()));
    }

    // Clone channels for tasks
    let world_rx = state.world_state_rx;
    let audio_rx = state.audio_params_rx;
    let event_tx = state.event_tx;

    // Spawn task to send messages from mpsc to WebSocket
    let send_task = tokio::spawn(async move {
        let mut rx_stream = UnboundedReceiverStream::new(rx);
        while let Some(message) = rx_stream.next().await {
            if sender.send(message).await.is_err() {
                break; // Connection closed
            }
        }
    });

    // Spawn outgoing task (snapshots)
    let outgoing_tx = tx.clone();
    tokio::spawn(async move {
        handle_outgoing_snapshots(world_rx, audio_rx, outgoing_tx).await;
    });

    // Spawn incoming task (client messages)
    let incoming_tx = tx;
    tokio::spawn(async move {
        handle_incoming_messages(receiver, event_tx, incoming_tx, session_id).await;
    });

    // Wait for the send task to finish (connection closed)
    let _ = send_task.await;
}

async fn handle_outgoing_snapshots(
    world_rx: watch::Receiver<WorldSnapshot>,
    audio_rx: watch::Receiver<AudioParams>,
    tx: mpsc::UnboundedSender<Message>,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100)); // 10 Hz - sane update rate

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Get latest world state
                let world = world_rx.borrow().clone();

                // Get latest audio params
                let audio_params = *audio_rx.borrow();
                let audio = AudioParamsSnapshot {
                    master_gain: audio_params.master_gain,
                    base_freq_hz: audio_params.base_freq_hz,
                    detune_ratio: audio_params.detune_ratio,
                    brightness: audio_params.brightness,
                    motion: audio_params.motion,
                    texture: audio_params.texture,
                    sparkle_impulse: audio_params.sparkle_impulse,
                };

                let snapshot = ServerMessage::Snapshot {
                    version: "1.0".to_string(),
                    payload: SnapshotPayload { world, audio },
                };
                if let Ok(json) = serde_json::to_string(&snapshot)
                    && tx.send(Message::Text(json.into())).is_err()
                {
                    break; // Connection closed
                }
            }
        }
    }
}

async fn handle_incoming_messages(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    event_tx: mpsc::Sender<Event>,
    tx: mpsc::UnboundedSender<Message>,
    session_id: String,
) {
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        match client_msg {
                            ClientMessage::Perform { version, payload } => {
                                let PerformPayload { request_id, action } = payload;
                                // Validate the action before processing
                                match validate_perform_action(&action) {
                                    Ok(_) => {
                                        let event = Event::Perform(action.clone());
                                        if event_tx.send(event).await.is_ok() {
                                            // Send acknowledgment
                                            let (action_name, intensity) = get_action_info(&action);

                                            let ack = ServerMessage::EventAck {
                                                version: "1.0".to_string(),
                                                payload: EventAckPayload {
                                                    request_id,
                                                    action: action_name.to_string(),
                                                    intensity,
                                                },
                                            };

                                            if let Ok(json) = serde_json::to_string(&ack) {
                                                let _ = tx.send(Message::Text(json.into()));
                                            }
                                        } else {
                                            let error = ServerMessage::Error {
                                                version: "1.0".to_string(),
                                                payload: ErrorPayload {
                                                    code: "SEND_FAILED".to_string(),
                                                    message: "Failed to send event".to_string(),
                                                    request_id,
                                                },
                                            };
                                            if let Ok(json) = serde_json::to_string(&error) {
                                                let _ = tx.send(Message::Text(json.into()));
                                            }
                                        }
                                    }
                                    Err(validation_error) => {
                                        let error = ServerMessage::Error {
                                            version: "1.0".to_string(),
                                            payload: ErrorPayload {
                                                code: "VALIDATION_ERROR".to_string(),
                                                message: validation_error,
                                                request_id,
                                            },
                                        };
                                        if let Ok(json) = serde_json::to_string(&error) {
                                            let _ = tx.send(Message::Text(json.into()));
                                        }
                                    }
                                }
                            }
                            ClientMessage::Ping {
                                version: _,
                                payload: _,
                            } => {
                                // Echo back ping (could add pong message type later)
                                tracing::debug!("Received ping from session {}", session_id);
                            }
                            ClientMessage::SetScene { version, payload } => {
                                let SetScenePayload {
                                    request_id,
                                    scene_name,
                                } = payload;
                                if scene_name.trim().is_empty() {
                                    let error = ServerMessage::Error {
                                        version: "1.0".to_string(),
                                        payload: ErrorPayload {
                                            request_id,
                                            code: "VALIDATION_ERROR".to_string(),
                                            message: "Scene name cannot be empty".to_string(),
                                        },
                                    };
                                    if let Ok(json) = serde_json::to_string(&error) {
                                        let _ = tx.send(Message::Text(json.into()));
                                    }
                                    continue;
                                }

                                // For now, treat as scene perform action
                                let action = PerformAction::Scene { name: scene_name };
                                let event = Event::Perform(action);
                                if event_tx.send(event).await.is_ok() {
                                    let ack = ServerMessage::EventAck {
                                        version: "1.0".to_string(),
                                        payload: EventAckPayload {
                                            request_id,
                                            action: "Scene".to_string(),
                                            intensity: None,
                                        },
                                    };
                                    if let Ok(json) = serde_json::to_string(&ack) {
                                        let _ = tx.send(Message::Text(json.into()));
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let error = ServerMessage::Error {
                            version: "1.0".to_string(),
                            payload: ErrorPayload {
                                code: "INVALID_MESSAGE".to_string(),
                                message: format!("Failed to parse message: {}", e),
                                request_id: None,
                            },
                        };
                        if let Ok(json) = serde_json::to_string(&error) {
                            let _ = tx.send(Message::Text(json.into()));
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {} // Ignore other message types
            Err(_) => break,
        }
    }
}
