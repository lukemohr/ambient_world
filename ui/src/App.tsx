import { useState, useEffect, useRef } from 'react'
import { AmbientWorldConnection, ConnectionStatus } from './ws/connection'
import type { ConnectionState, ServerMessage, ConnectionStatusType, WorldSnapshot, PerformAction } from './ws/connection'
import './App.css'

function App() {
  const [connection, setConnection] = useState<AmbientWorldConnection | null>(null)
  const [connectionState, setConnectionState] = useState<ConnectionState>({
    status: ConnectionStatus.DISCONNECTED
  })
  const [intensity, setIntensity] = useState<number>(1.0)
  const [selectedScene, setSelectedScene] = useState<string>('default')
  const [isFrozen, setIsFrozen] = useState<boolean>(false)
  const [gestureActive, setGestureActive] = useState<boolean>(false)
  const [activeActions, setActiveActions] = useState<Set<string>>(new Set())
  const [worldState, setWorldState] = useState<WorldSnapshot>({
    density: 0.5,
    rhythm: 0.5,
    tension: 0.5,
    energy: 0.5,
    warmth: 0.5,
    sparkle_impulse: 0.0
  })
  const [eventFeed, setEventFeed] = useState<Array<{timestamp: number, action: string}>>([])
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    // Create WebSocket connection
    const wsConnection = new AmbientWorldConnection('ws://localhost:3000/ws')

    // Set up event listeners
    wsConnection.on('stateChange', (state) => {
      setConnectionState(state)
    })

    wsConnection.on('message', (message: ServerMessage) => {
      console.log('Received message:', message)
      
      if (message.type === 'snapshot') {
        setWorldState(message.payload.world)
      } else if (message.type === 'event_ack') {
        const timestamp = Date.now()
        setEventFeed(prev => [...prev.slice(-10), { timestamp, action: message.payload.action }]) // Keep last 10
      }
    })

    wsConnection.on('error', (error) => {
      console.error('WebSocket error:', error)
    })

    // Connect
    wsConnection.connect()
    setConnection(wsConnection)

    // Cleanup on unmount
    return () => {
      wsConnection.disconnect()
    }
  }, [])

  const handleSetScene = (sceneName: string) => {
    if (connection) {
      connection.setScene(sceneName)
      setSelectedScene(sceneName)
      console.log(`Scene changed to: ${sceneName}`)
    }
  }

  const handleFreeze = () => {
    const newFrozen = !isFrozen
    setIsFrozen(newFrozen)
    if (connection) {
      connection.perform({ Freeze: { seconds: newFrozen ? 300 : 0 } })
    }
  }

  const startAction = (action: PerformAction, actionName: string) => {
    if (activeActions.has(actionName)) return
    setActiveActions(prev => new Set(prev).add(actionName))
    console.log(`Starting ${actionName} action`)
    
    const interval = setInterval(() => {
      if (connection) {
        connection.perform(action)
      }
    }, 150)
    
    ;(window as any)[`interval_${actionName}`] = interval
  }

  const stopAction = (actionName: string) => {
    setActiveActions(prev => {
      const newSet = new Set(prev)
      newSet.delete(actionName)
      return newSet
    })
    const interval = (window as any)[`interval_${actionName}`]
    if (interval) {
      clearInterval(interval)
      delete (window as any)[`interval_${actionName}`]
    }
  }

  const handleCanvasMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    setGestureActive(true)
    handleCanvasGesture(e)
  }

  const handleCanvasMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (gestureActive) {
      handleCanvasGesture(e)
    }
  }

  const handleCanvasMouseUp = () => {
    setGestureActive(false)
  }

  const handleCanvasGesture = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!connection) return

    const canvas = e.currentTarget
    const rect = canvas.getBoundingClientRect()
    const x = (e.clientX - rect.left) / rect.width
    const y = (e.clientY - rect.top) / rect.height

    // Map x to warmth (0 = cool, 1 = warm)
    const warmthIntensity = x * intensity
    // Map y to energy (0 = calm, 1 = energetic)
    const energyIntensity = (1 - y) * intensity

    // Send combined actions
    if (warmthIntensity > 0.1) {
      connection.perform({ Heat: { intensity: warmthIntensity } })
    }
    if (energyIntensity > 0.1) {
      connection.perform({ Pulse: { intensity: energyIntensity } })
    }
  }

  const worldStateRef = useRef<WorldSnapshot>(worldState)
  
  // Update ref when worldState changes
  useEffect(() => {
    worldStateRef.current = worldState
  }, [worldState])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const numParticles = 100
    const particles: Array<{x: number, y: number, vx: number, vy: number, age: number}> = []

    // Initialize particles
    for (let i = 0; i < numParticles; i++) {
      particles.push({
        x: Math.random() * canvas.width,
        y: Math.random() * canvas.height,
        vx: (Math.random() - 0.5) * 1.0,
        vy: (Math.random() - 0.5) * 1.0,
        age: Math.random() * 1000
      })
    }

    let animationId: number
    let lastTime = 0

    const animate = (time: number) => {
      const dt = time - lastTime
      lastTime = time

      // Read latest world state from ref (not React state)
      const currentWorldState = worldStateRef.current

      // Clear with fade
      ctx.fillStyle = 'rgba(0, 0, 0, 0.02)'
      ctx.fillRect(0, 0, canvas.width, canvas.height)

      // Rhythm pulsation increased
      const rhythmScale = 1 + Math.sin(time * 0.001 * currentWorldState.rhythm * 10) * 0.3 * currentWorldState.rhythm

      particles.forEach(particle => {
        // Update age
        particle.age += dt

        // Energy affects velocity
        const speedMultiplier = 0.5 + currentWorldState.energy * 1.5

        // Tension jitter increased
        particle.vx += (Math.random() - 0.5) * currentWorldState.tension * 0.2
        particle.vy += (Math.random() - 0.5) * currentWorldState.tension * 0.2

        // Add continuous baseline motion
        particle.vx += (Math.random() - 0.5) * 0.1
        particle.vy += (Math.random() - 0.5) * 0.1

        // Update position
        particle.x += particle.vx * speedMultiplier * rhythmScale
        particle.y += particle.vy * speedMultiplier * rhythmScale

        // Wrap around edges
        if (particle.x < 0) particle.x = canvas.width
        if (particle.x > canvas.width) particle.x = 0
        if (particle.y < 0) particle.y = canvas.height
        if (particle.y > canvas.height) particle.y = 0

        // Dampen velocity less aggressively
        particle.vx *= 0.98
        particle.vy *= 0.98

        // Draw
        const hue = 240 + currentWorldState.warmth * 120 // Blue to red
        const saturation = 70
        const lightness = 50
        ctx.fillStyle = `hsl(${hue}, ${saturation}%, ${lightness}%)`
        const size = 1 + currentWorldState.energy * 3
        ctx.beginPath()
        ctx.arc(particle.x, particle.y, size, 0, Math.PI * 2)
        ctx.fill()
      })

      animationId = requestAnimationFrame(animate)
    }

    animate(0)

    return () => {
      if (animationId) {
        cancelAnimationFrame(animationId)
      }
    }
  }, []) // No dependencies - canvas runs its own loop

  const getStatusColor = (status: ConnectionStatusType) => {
    switch (status) {
      case ConnectionStatus.CONNECTED:
        return 'green'
      case ConnectionStatus.CONNECTING:
        return 'yellow'
      case ConnectionStatus.DISCONNECTED:
        return 'red'
      default:
        return 'gray'
    }
  }

  return (
    <div className="app">
      <header>
        <h1>Ambient World</h1>
        <div className="connection-status">
          <span
            className="status-indicator"
            style={{ backgroundColor: getStatusColor(connectionState.status) }}
          />
          <span className="status-text">
            {connectionState.status.toUpperCase()}
            {connectionState.sessionId && ` (Session: ${connectionState.sessionId})`}
          </span>
        </div>
      </header>

      <main className="main-layout">
        <div className="left-column">
          <div className="controls">
            <div className="control-group">
              <label>Intensity: {intensity.toFixed(2)}</label>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={intensity}
                onChange={(e) => setIntensity(parseFloat(e.target.value))}
              />
            </div>

            <div className="control-group">
              <div className="button-grid">
                <button
                  onMouseDown={() => startAction({ Pulse: { intensity } }, 'Pulse')}
                  onMouseUp={() => stopAction('Pulse')}
                  onMouseLeave={() => stopAction('Pulse')}
                  className={activeActions.has('Pulse') ? 'active' : ''}
                >
                  Pulse
                </button>
                <button
                  onMouseDown={() => startAction({ Calm: { intensity } }, 'Calm')}
                  onMouseUp={() => stopAction('Calm')}
                  onMouseLeave={() => stopAction('Calm')}
                  className={activeActions.has('Calm') ? 'active' : ''}
                >
                  Calm
                </button>
                <button
                  onMouseDown={() => startAction({ Stir: { intensity } }, 'Stir')}
                  onMouseUp={() => stopAction('Stir')}
                  onMouseLeave={() => stopAction('Stir')}
                  className={activeActions.has('Stir') ? 'active' : ''}
                >
                  Stir
                </button>
                <button
                  onMouseDown={() => startAction({ Tense: { intensity } }, 'Tense')}
                  onMouseUp={() => stopAction('Tense')}
                  onMouseLeave={() => stopAction('Tense')}
                  className={activeActions.has('Tense') ? 'active' : ''}
                >
                  Tense
                </button>
                <button
                  onMouseDown={() => startAction({ Heat: { intensity } }, 'Heat')}
                  onMouseUp={() => stopAction('Heat')}
                  onMouseLeave={() => stopAction('Heat')}
                  className={activeActions.has('Heat') ? 'active' : ''}
                >
                  Heat
                </button>
              </div>
            </div>

            <div className="control-group">
              <label>Scene:</label>
              <select value={selectedScene} onChange={(e) => handleSetScene(e.target.value)}>
                <option value="default">Default - Balanced atmosphere</option>
                <option value="peaceful">Peaceful - Calm, warm, relaxed</option>
                <option value="energetic">Energetic - Fast-paced, intense, lively</option>
                <option value="mysterious">Mysterious - Dark, tense, sparse</option>
              </select>
            </div>

            <div className="control-group">
              <label>
                <input
                  type="checkbox"
                  checked={isFrozen}
                  onChange={handleFreeze}
                />
                Freeze
              </label>
            </div>
          </div>

        </div>

        <div className="right-column">
          <div className="visualizer">
            <h3>Visualizer (Gesture Pad)</h3>
            <canvas
              ref={canvasRef}
              width={500}
              height={350}
              onMouseDown={handleCanvasMouseDown}
              onMouseMove={handleCanvasMouseMove}
              onMouseUp={handleCanvasMouseUp}
              onMouseLeave={handleCanvasMouseUp}
              style={{ cursor: gestureActive ? 'crosshair' : 'pointer' }}
            ></canvas>
          </div>

          <div className="meters">
            <h3>Meters</h3>
            <div className="scene-info">
              <p><strong>Current Scene:</strong> {selectedScene}</p>
              <p><small>Values decay toward scene baselines over time</small></p>
            </div>
            <div className="meter-grid">
              <div className="meter">
                <label>Density: {worldState.density.toFixed(5)}</label>
                <div className="meter-bar">
                  <div className="meter-fill" style={{ width: `${worldState.density * 100}%` }}></div>
                </div>
              </div>
              <div className="meter">
                <label>Rhythm: {worldState.rhythm.toFixed(5)}</label>
                <div className="meter-bar">
                  <div className="meter-fill" style={{ width: `${worldState.rhythm * 100}%` }}></div>
                </div>
              </div>
              <div className="meter">
                <label>Tension: {worldState.tension.toFixed(5)}</label>
                <div className="meter-bar">
                  <div className="meter-fill" style={{ width: `${worldState.tension * 100}%` }}></div>
                </div>
              </div>
              <div className="meter">
                <label>Energy: {worldState.energy.toFixed(5)}</label>
                <div className="meter-bar">
                  <div className="meter-fill" style={{ width: `${worldState.energy * 100}%` }}></div>
                </div>
              </div>
              <div className="meter">
                <label>Warmth: {worldState.warmth.toFixed(5)}</label>
                <div className="meter-bar">
                  <div className="meter-fill" style={{ width: `${worldState.warmth * 100}%` }}></div>
                </div>
              </div>
            </div>
          </div>

          <div className="event-feed">
            <h3>Event Feed</h3>
            <div className="feed-list">
              {eventFeed.map((event, index) => (
                <div key={index} className="feed-item">
                  {new Date(event.timestamp).toLocaleTimeString()}: {event.action}
                </div>
              ))}
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}

export default App
