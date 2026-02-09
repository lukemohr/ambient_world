// WebSocket connection module for Ambient World UI
// Handles connection management, reconnection, and message passing

export const ConnectionStatus = {
  DISCONNECTED: 'disconnected',
  CONNECTING: 'connecting',
  CONNECTED: 'connected',
} as const

export type ConnectionStatusType = typeof ConnectionStatus[keyof typeof ConnectionStatus]

export interface ConnectionState {
  status: ConnectionStatusType;
  sessionId?: string;
  schemaVersion?: string;
  lastError?: string;
}

// Data types mirroring Rust structs
export interface WorldSnapshot {
  density: number;
  rhythm: number;
  tension: number;
  energy: number;
  warmth: number;
  sparkle_impulse: number;
}

export interface AudioParamsSnapshot {
  master_gain: number;
  base_freq_hz: number;
  detune_ratio: number;
  brightness: number;
  motion: number;
  texture: number;
  sparkle_impulse: number;
}

export interface HelloPayload {
  session_id: string;
  schema_version: string;
  tick_rate_hz: number;
}

export interface SnapshotPayload {
  world: WorldSnapshot;
  audio: AudioParamsSnapshot;
}

export interface EventAckPayload {
  request_id?: string;
  action: string;
  intensity?: number;
}

export interface ErrorPayload {
  code: string;
  message: string;
  request_id?: string;
}

// PerformAction types mirroring Rust enum
export type PerformAction =
  | { Pulse: { intensity: number } }
  | { Calm: { intensity: number } }
  | { Stir: { intensity: number } }
  | { Tense: { intensity: number } }
  | { Heat: { intensity: number } }
  | { Scene: { name: string } }
  | { Freeze: { seconds: number } };

// Message types
export interface BaseMessage {
  version: string;
  type: string;
  payload: any;
}

export interface HelloMessage extends BaseMessage {
  type: 'hello';
  payload: HelloPayload;
}

export interface SnapshotMessage extends BaseMessage {
  type: 'snapshot';
  payload: SnapshotPayload;
}

export interface EventAckMessage extends BaseMessage {
  type: 'event_ack';
  payload: EventAckPayload;
}

export interface ErrorMessage extends BaseMessage {
  type: 'error';
  payload: ErrorPayload;
}

export type ServerMessage = HelloMessage | SnapshotMessage | EventAckMessage | ErrorMessage;

// Client message types
export interface PerformPayload {
  request_id?: string;
  action: PerformAction;
}

export interface SetScenePayload {
  request_id?: string;
  scene_name: string;
}

export interface PingPayload {
  timestamp: number;
}

export interface PerformMessage extends BaseMessage {
  type: 'perform';
  payload: PerformPayload;
}

export interface SetSceneMessage extends BaseMessage {
  type: 'set_scene';
  payload: SetScenePayload;
}

export interface PingMessage extends BaseMessage {
  type: 'ping';
  payload: PingPayload;
}

export type ClientMessage = PerformMessage | SetSceneMessage | PingMessage;

// Event types for the connection
export interface ConnectionEvents {
  stateChange: (state: ConnectionState) => void;
  message: (message: ServerMessage) => void;
  error: (error: Event) => void;
}

export class AmbientWorldConnection {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private reconnectDelay = 1000; // Start with 1 second
  private maxReconnectDelay = 30000; // Max 30 seconds
  private reconnectTimer: number | null = null;
  private url: string;
  private state: ConnectionState = { status: ConnectionStatus.DISCONNECTED };
  private eventListeners: Partial<ConnectionEvents> = {};

  constructor(url: string = 'ws://localhost:3000/ws') {
    this.url = url;
  }

  // Event listener management
  on<K extends keyof ConnectionEvents>(event: K, listener: ConnectionEvents[K]) {
    this.eventListeners[event] = listener;
  }

  off<K extends keyof ConnectionEvents>(event: K) {
    delete this.eventListeners[event];
  }

  private emit<K extends keyof ConnectionEvents>(event: K, ...args: Parameters<ConnectionEvents[K]>) {
    const listener = this.eventListeners[event];
    if (listener) {
      (listener as any)(...args);
    }
  }

  // Connection management
  connect(): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return; // Already connected
    }

    this.updateState({ status: ConnectionStatus.CONNECTING, lastError: undefined });
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.reconnectAttempts = 0;
      this.reconnectDelay = 1000;
      this.updateState({ status: ConnectionStatus.CONNECTED, lastError: undefined });
    };

    this.ws.onmessage = (event) => {
      try {
        const message: ServerMessage = JSON.parse(event.data);
        this.handleMessage(message);
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
      }
    };

    this.ws.onclose = () => {
      console.log('WebSocket disconnected');
      this.updateState({ status: ConnectionStatus.DISCONNECTED });
      this.scheduleReconnect();
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      this.updateState({
        status: ConnectionStatus.DISCONNECTED,
        lastError: 'WebSocket connection error'
      });
      this.emit('error', error);
    };
  }

  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.updateState({ status: ConnectionStatus.DISCONNECTED });
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.log('Max reconnection attempts reached');
      return;
    }

    this.reconnectAttempts++;
    console.log(`Scheduling reconnect attempt ${this.reconnectAttempts} in ${this.reconnectDelay}ms`);

    this.reconnectTimer = window.setTimeout(() => {
      this.connect();
    }, this.reconnectDelay);

    // Exponential backoff
    this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
  }

  private updateState(updates: Partial<ConnectionState>): void {
    this.state = { ...this.state, ...updates };
    this.emit('stateChange', this.state);
  }

  private handleMessage(message: ServerMessage): void {
    if (message.type === 'hello') {
      const helloMsg = message as HelloMessage;
      this.updateState({
        sessionId: helloMsg.payload.session_id,
        schemaVersion: helloMsg.payload.schema_version,
      });
    }

    this.emit('message', message);
  }

  // Message sending
  sendMessage(message: ClientMessage): boolean {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not connected, readyState:', this.ws?.readyState)
      return false;
    }

    try {
      this.ws.send(JSON.stringify(message));
      return true;
    } catch (error) {
      console.error('Failed to send WebSocket message:', error);
      return false;
    }
  }

  // General perform method
  perform(action: PerformAction, requestId?: string): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'perform',
      payload: {
        request_id: requestId,
        action,
      },
    });
  }

  // Convenience methods for common actions
  performPulse(intensity: number, requestId?: string): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'perform',
      payload: {
        request_id: requestId,
        action: { Pulse: { intensity } },
      },
    });
  }

  performCalm(intensity: number, requestId?: string): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'perform',
      payload: {
        request_id: requestId,
        action: { Calm: { intensity } },
      },
    });
  }

  performStir(intensity: number, requestId?: string): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'perform',
      payload: {
        request_id: requestId,
        action: { Stir: { intensity } },
      },
    });
  }

  performTense(intensity: number, requestId?: string): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'perform',
      payload: {
        request_id: requestId,
        action: { Tense: { intensity } },
      },
    });
  }

  performHeat(intensity: number, requestId?: string): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'perform',
      payload: {
        request_id: requestId,
        action: { Heat: { intensity } },
      },
    });
  }

  setScene(sceneName: string, requestId?: string): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'set_scene',
      payload: {
        request_id: requestId,
        scene_name: sceneName,
      },
    });
  }

  performFreeze(seconds: number, requestId?: string): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'perform',
      payload: {
        request_id: requestId,
        action: { Freeze: { seconds } },
      },
    });
  }

  ping(): boolean {
    return this.sendMessage({
      version: '1.0',
      type: 'ping',
      payload: {
        timestamp: Date.now(),
      },
    });
  }

  // Getters
  getState(): ConnectionState {
    return { ...this.state };
  }

  isConnected(): boolean {
    return this.state.status === ConnectionStatus.CONNECTED;
  }
}