# Ambient World WebSocket API

This document defines the WebSocket message contract for real-time communication between the Ambient World server and UI clients.

## Message Envelope

All messages use a consistent envelope structure for versioning and easy debugging:

```json
{
  "version": "1.0",
  "type": "message_type",
  "payload": {
    // Message-specific data
  }
}
```

## Server → Client Messages

### hello (Connection Handshake)

Sent immediately after WebSocket connection is established.

```json
{
  "version": "1.0",
  "type": "hello",
  "payload": {
    "session_id": "abc123",
    "schema_version": "1.0"
  }
}
```

### snapshot (World State Update)

Sent periodically with the latest world state and derived audio parameters.

```json
{
  "version": "1.0",
  "type": "snapshot",
  "payload": {
    "world": {
      "density": 0.5,
      "rhythm": 0.5,
      "tension": 0.5,
      "energy": 0.5,
      "warmth": 0.5,
      "sparkle_impulse": 0.0
    },
    "audio": {
      "master_gain": 0.1,
      "base_freq_hz": 200.0,
      "detune_ratio": 1.005,
      "brightness": 0.75,
      "motion": 0.25,
      "texture": 0.15,
      "sparkle_impulse": 0.0
    }
  }
}
```

### event_ack (Action Confirmation)

Sent to confirm that a client action was accepted and processed.

```json
{
  "version": "1.0",
  "type": "event_ack",
  "payload": {
    "request_id": "optional-client-provided-id",
    "action": "Pulse",
    "intensity": 0.8
  }
}
```

### error (Error Notification)

Sent when something goes wrong with a client request or server-side issue.

```json
{
  "version": "1.0",
  "type": "error",
  "payload": {
    "code": "INVALID_ACTION",
    "message": "Unknown perform action: InvalidAction",
    "request_id": "optional-client-provided-id"
  }
}
```

## Client → Server Messages

### perform (Execute World Action)

Request to perform an action that modifies the world state.

```json
{
  "version": "1.0",
  "type": "perform",
  "payload": {
    "request_id": "optional-client-id",
    "action": {
      "Pulse": {
        "intensity": 0.8
      }
    }
  }
}
```

```json
{
  "version": "1.0",
  "type": "perform",
  "payload": {
    "request_id": "scene-change-1",
    "action": {
      "Scene": {
        "name": "sunrise"
      }
    }
  }
}
```

```json
{
  "version": "1.0",
  "type": "perform",
  "payload": {
    "request_id": "freeze-1",
    "action": {
      "Freeze": {
        "seconds": 5.0
      }
    }
  }
}
```

### ping (Keepalive)

Optional keepalive message to maintain connection.

```json
{
  "version": "1.0",
  "type": "ping",
  "payload": {
    "timestamp": 1644345600
  }
}
```

### set_scene (Scene Selection)

Alternative to using perform for scene changes (if separated in UI).

```json
{
  "version": "1.0",
  "type": "set_scene",
  "payload": {
    "request_id": "scene-1",
    "scene_name": "sunrise"
  }
}
```

## Connection Protocol

1. **Connect**: Client establishes WebSocket connection to `/ws`
2. **Hello**: Server sends `hello` message with session info
3. **Subscribe**: Client can optionally send initial preferences
4. **Stream**: Server sends periodic `snapshot` messages
5. **Interact**: Client sends `perform` messages, server responds with `event_ack` or `error`
6. **Keepalive**: Either side can send `ping` messages
7. **Disconnect**: Either side can close the connection

## Error Codes

- `INVALID_ACTION`: Unknown or malformed action
- `RATE_LIMITED`: Too many requests
- `SERVER_ERROR`: Internal server error
- `VERSION_MISMATCH`: Client/server version incompatibility
- `VALIDATION_ERROR`: Input validation failed (see Validation Rules)

## Validation Rules

The server validates all incoming messages to ensure data integrity and prevent invalid state changes:

### Perform Actions

- **intensity**: Must be between 0.0 and 1.0 (inclusive)
- **scene_name**: Must be a non-empty string (after trimming whitespace)
- **freeze_duration_ms**: Must be positive (> 0)

### Set Scene Actions

- **scene_name**: Must be a non-empty string (after trimming whitespace)

Invalid inputs will result in a `VALIDATION_ERROR` response with details about the specific validation failure.

## Version History

- **1.0**: Initial version with basic world state streaming and action execution
