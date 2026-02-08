# Ambient World Demo

This document provides curl commands to interact with the Ambient World API.

## Health Check

Check if the server is running:

```bash
curl http://localhost:3000/health
```

Expected response: `"ok"`

## Get Current State

Retrieve the latest world state snapshot:

```bash
curl http://localhost:3000/state
```

Expected response: JSON object with density, rhythm, tension, energy, warmth values.

Example:

```json
{
  "density": 0.5,
  "rhythm": 0.5,
  "tension": 0.5,
  "energy": 0.5,
  "warmth": 0.5
}
```

## Send Trigger Event

Send a trigger event to modify the world state:

```bash
curl -X POST http://localhost:3000/event \
  -H "Content-Type: application/json" \
  -d '{"kind": "Pulse", "intensity": 0.8}'
```

Valid trigger kinds: "Pulse", "Stir", "Calm", "Heat", "Tense"

Expected response: `"Event sent"` on success, or an error message.

## Configuration

The application can be configured via environment variables:

- `TICK_HZ`: Tick rate in Hz (default: 20.0)
- `PORT`: API server port (default: 3000)

Example:

```bash
TICK_HZ=10.0 PORT=8080 cargo run -p app
```
