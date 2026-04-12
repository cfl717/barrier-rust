import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import './styles/App.css'

interface AppState {
  mode: string
  is_running: boolean
  connected_clients: number
  server_address: string
  log_messages: string[]
}

function App() {
  const [mode, setMode] = useState<'server' | 'client'>('server')
  const [serverAddress, setServerAddress] = useState('')
  const [appState, setAppState] = useState<AppState | null>(null)
  const [logs, setLogs] = useState<string[]>([])
  const [isRunning, setIsRunning] = useState(false)

  useEffect(() => {
    loadStatus()
    const interval = setInterval(loadStatus, 2000)
    return () => clearInterval(interval)
  }, [])

  const loadStatus = async () => {
    try {
      const status = await invoke<AppState>('get_status')
      setAppState(status)
      setIsRunning(status.is_running)
      if (status.log_messages.length > logs.length) {
        setLogs(status.log_messages)
      }
    } catch (error) {
      console.error('Failed to get status:', error)
    }
  }

  const handleStart = async () => {
    try {
      if (mode === 'server') {
        await invoke('start_server', {
          config: {
            address: '0.0.0.0:24800',
            enable_clipboard: true,
            enable_drag_drop: true,
          }
        })
      } else {
        await invoke('start_client', {
          config: {
            server_address: serverAddress || 'localhost:24800',
            client_name: 'client',
            enable_clipboard: true,
            enable_drag_drop: true,
          }
        })
      }
      setIsRunning(true)
      setLogs(prev => [...prev, `${mode} started successfully`])
    } catch (error) {
      setLogs(prev => [...prev, `Error starting ${mode}: ${error}`])
    }
  }

  const handleStop = async () => {
    try {
      await invoke('stop_service')
      setIsRunning(false)
      setLogs(prev => [...prev, 'Service stopped'])
    } catch (error) {
      setLogs(prev => [...prev, `Error stopping service: ${error}`])
    }
  }

  return (
    <div className="app-container">
      <header className="header">
        <h1>🖥️ Barrier</h1>
        <p className="subtitle">Share your mouse and keyboard between multiple computers</p>
      </header>

      <main className="main-content">
        <div className="mode-selector">
          <button
            className={`mode-btn ${mode === 'server' ? 'active' : ''}`}
            onClick={() => setMode('server')}
            disabled={isRunning}
          >
            🖥️ Server (Share this computer's mouse & keyboard)
          </button>
          <button
            className={`mode-btn ${mode === 'client' ? 'active' : ''}`}
            onClick={() => setMode('client')}
            disabled={isRunning}
          >
            💻 Client (Use another computer's mouse & keyboard)
          </button>
        </div>

        {mode === 'client' && (
          <div className="server-input">
            <label htmlFor="server-address">Server Address:</label>
            <input
              id="server-address"
              type="text"
              value={serverAddress}
              onChange={(e) => setServerAddress(e.target.value)}
              placeholder="e.g., 192.168.1.100:24800"
              disabled={isRunning}
            />
          </div>
        )}

        <div className="action-buttons">
          {!isRunning ? (
            <button className="start-btn" onClick={handleStart}>
              ▶️ Start {mode === 'server' ? 'Server' : 'Client'}
            </button>
          ) : (
            <button className="stop-btn" onClick={handleStop}>
              ⏹️ Stop
            </button>
          )}
        </div>

        {appState && appState.is_running && (
          <div className="status-panel">
            <h3>Status</h3>
            <div className="status-item">
              <span className="status-label">Mode:</span>
              <span className="status-value">{appState.mode}</span>
            </div>
            <div className="status-item">
              <span className="status-label">Address:</span>
              <span className="status-value">{appState.server_address}</span>
            </div>
            {appState.mode === 'server' && (
              <div className="status-item">
                <span className="status-label">Connected Clients:</span>
                <span className="status-value">{appState.connected_clients}</span>
              </div>
            )}
          </div>
        )}

        <div className="log-panel">
          <h3>Activity Log</h3>
          <div className="log-content">
            {logs.length === 0 ? (
              <p className="no-logs">No activity yet...</p>
            ) : (
              logs.map((log, index) => (
                <div key={index} className="log-entry">
                  {log}
                </div>
              ))
            )}
          </div>
        </div>
      </main>

      <footer className="footer">
        <p>Barrier v3.0.0-alpha.1 - Rust + Tauri Rewrite</p>
      </footer>
    </div>
  )
}

export default App
