import { useEffect, useRef, useState } from 'react'
import * as tauriCore from '@tauri-apps/api/core'
import './styles/App.css'

interface AppState {
  mode: string
  is_running: boolean
  connected_clients: number
  server_address: string
  log_messages: string[]
}

interface RuntimeCapabilities {
  session_type: string
  has_x11_display: boolean
  has_wayland_display: boolean
  input_backend: string
  recommendations: string[]
  blocking_issues: string[]
}

type ScreenEdge = 'left' | 'right' | 'top' | 'bottom'

interface ClientRoute {
  id: string
  name: string
  address: string
  edge: ScreenEdge
}

function App() {
  const invoke = tauriCore.invoke
  const [mode, setMode] = useState<'server' | 'client'>('server')
  const [serverPort, setServerPort] = useState(24800)
  const [serverScreenName, setServerScreenName] = useState('server')
  const [clientRoutes, setClientRoutes] = useState<ClientRoute[]>([
    { id: crypto.randomUUID(), name: 'client-1', address: '192.168.1.101:24800', edge: 'right' },
  ])
  const [serverAddress, setServerAddress] = useState('')
  const [clientName, setClientName] = useState('client-1')
  const [appState, setAppState] = useState<AppState | null>(null)
  const [capabilities, setCapabilities] = useState<RuntimeCapabilities | null>(null)
  const [logs, setLogs] = useState<string[]>([])
  const [isRunning, setIsRunning] = useState(false)
  const [activeClient, setActiveClient] = useState<string | null>(null)
  const lastEdgeSwitchTsRef = useRef(0)

  useEffect(() => {
    loadStatus()
    loadRuntimeCapabilities()
    const interval = setInterval(loadStatus, 2000)
    return () => clearInterval(interval)
  }, [])

  const loadRuntimeCapabilities = async () => {
    try {
      const caps = await invoke<RuntimeCapabilities>('get_runtime_capabilities')
      setCapabilities(caps)
    } catch (error) {
      console.error('Failed to get runtime capabilities:', error)
    }
  }

  const loadStatus = async () => {
    try {
      const status = await invoke<AppState>('get_status')
      setAppState(status)
      setIsRunning(status.is_running)
      setLogs(prevLogs => (
        status.log_messages.length > prevLogs.length ? status.log_messages : prevLogs
      ))
    } catch (error) {
      console.error('Failed to get status:', error)
    }
  }

  const handleStart = async () => {
    try {
      if (mode === 'server') {
        if (serverPort < 1 || serverPort > 65535) {
          throw new Error('端口范围必须在 1-65535')
        }

        await invoke('start_server', {
          config: {
            address: `0.0.0.0:${serverPort}`,
            screen_name: serverScreenName || 'server',
            max_clients: Math.max(clientRoutes.length, 1),
            enable_clipboard: true,
            enable_drag_drop: true,
            client_routes: clientRoutes,
          }
        })
      } else {
        await invoke('start_client', {
          config: {
            server_address: serverAddress || 'localhost:24800',
            client_name: clientName || 'client-1',
            enable_clipboard: true,
            enable_drag_drop: true,
          }
        })
      }
      await loadStatus()
      setIsRunning(true)
      setLogs(prev => [...prev, `${mode} started successfully`])
    } catch (error) {
      setIsRunning(false)
      setLogs(prev => [...prev, `Error starting ${mode}: ${error}`])
    }
  }

  const handleStop = async () => {
    try {
      await invoke('stop_service')
      setIsRunning(false)
      setActiveClient(null)
      setLogs(prev => [...prev, 'Service stopped'])
    } catch (error) {
      setLogs(prev => [...prev, `Error stopping service: ${error}`])
    }
  }

  const addClientRoute = () => {
    const nextIndex = clientRoutes.length + 1
    setClientRoutes(prev => [
      ...prev,
      {
        id: crypto.randomUUID(),
        name: `client-${nextIndex}`,
        address: '',
        edge: 'right',
      },
    ])
  }

  const removeClientRoute = (id: string) => {
    setClientRoutes(prev => prev.filter(route => route.id !== id))
  }

  const updateClientRoute = (id: string, patch: Partial<ClientRoute>) => {
    setClientRoutes(prev =>
      prev.map(route => (route.id === id ? { ...route, ...patch } : route))
    )
  }

  useEffect(() => {
    if (!isRunning || appState?.mode !== 'server' || clientRoutes.length === 0) {
      return
    }

    const threshold = 20
    const onMouseMove = (event: MouseEvent) => {
      let edge: ScreenEdge | null = null
      if (event.clientX <= threshold) edge = 'left'
      else if (event.clientX >= window.innerWidth - threshold) edge = 'right'
      else if (event.clientY <= threshold) edge = 'top'
      else if (event.clientY >= window.innerHeight - threshold) edge = 'bottom'

      if (!edge) return

      const matched = clientRoutes.find(route => route.edge === edge)
      if (!matched) return

      const now = Date.now()
      if (matched.name === activeClient || now - lastEdgeSwitchTsRef.current < 500) {
        return
      }

      lastEdgeSwitchTsRef.current = now
      setActiveClient(matched.name)
      void invoke<string>('switch_client', {
        request: {
          edge,
          client_name: matched.name,
          client_address: matched.address || null,
        },
      })
        .then((message) => {
          setLogs(prev => [...prev, message])
        })
        .catch((error) => {
          setLogs(prev => [...prev, `Edge switch failed: ${String(error)}`])
        })
    }

    const onMouseLeave = (event: MouseEvent) => {
      let edge: ScreenEdge | null = null
      if (event.clientX <= 0) edge = 'left'
      else if (event.clientX >= window.innerWidth) edge = 'right'
      else if (event.clientY <= 0) edge = 'top'
      else if (event.clientY >= window.innerHeight) edge = 'bottom'
      if (!edge) return

      const matched = clientRoutes.find(route => route.edge === edge)
      if (!matched) return

      const now = Date.now()
      if (matched.name === activeClient || now - lastEdgeSwitchTsRef.current < 500) {
        return
      }

      lastEdgeSwitchTsRef.current = now
      setActiveClient(matched.name)
      void invoke<string>('switch_client', {
        request: {
          edge,
          client_name: matched.name,
          client_address: matched.address || null,
        },
      })
        .then((message) => {
          setLogs(prev => [...prev, message])
        })
        .catch((error) => {
          setLogs(prev => [...prev, `Edge leave switch failed: ${String(error)}`])
        })
    }

    window.addEventListener('mousemove', onMouseMove)
    window.addEventListener('mouseleave', onMouseLeave)
    return () => {
      window.removeEventListener('mousemove', onMouseMove)
      window.removeEventListener('mouseleave', onMouseLeave)
    }
  }, [activeClient, appState?.mode, clientRoutes, isRunning])

  return (
    <div className="app-container">
      <header className="header">
        <h1>🖥️ Barrier</h1>
        <p className="subtitle">Share your mouse and keyboard between multiple computers</p>
      </header>

      <main className="main-content">
        {capabilities && (
          <div className="runtime-capability-panel">
            <h3>运行环境检测</h3>
            <div className="status-item">
              <span className="status-label">Session:</span>
              <span className="status-value">{capabilities.session_type}</span>
            </div>
            <div className="status-item">
              <span className="status-label">输入后端:</span>
              <span className="status-value">{capabilities.input_backend}</span>
            </div>
            {capabilities.blocking_issues.length > 0 && (
              <div className="runtime-warning">
                {capabilities.blocking_issues.map((issue, index) => (
                  <p key={index}>- {issue}</p>
                ))}
              </div>
            )}
            {capabilities.recommendations.length > 0 && (
              <div className="runtime-hint">
                {capabilities.recommendations.map((tip, index) => (
                  <p key={index}>- {tip}</p>
                ))}
              </div>
            )}
          </div>
        )}

        <div className="mode-selector">
          <button
            className={`mode-btn ${mode === 'server' ? 'active' : ''}`}
            onClick={() => setMode('server')}
          >
            🖥️ Server (Share this computer's mouse & keyboard)
          </button>
          <button
            className={`mode-btn ${mode === 'client' ? 'active' : ''}`}
            onClick={() => setMode('client')}
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
            />
            <label htmlFor="client-name" style={{ marginTop: '0.5rem' }}>Client Name:</label>
            <input
              id="client-name"
              type="text"
              value={clientName}
              onChange={(e) => setClientName(e.target.value)}
              placeholder="e.g., client-1"
            />
          </div>
        )}

        {mode === 'server' && (
          <div className="server-config-panel">
            <h3>Server 配置</h3>
            <div className="server-config-grid">
              <div className="server-input">
                <label htmlFor="server-port">监听端口</label>
                <input
                  id="server-port"
                  type="number"
                  min={1}
                  max={65535}
                  value={serverPort}
                  onChange={(e) => setServerPort(Number(e.target.value))}
                />
              </div>
              <div className="server-input">
                <label htmlFor="screen-name">服务端名称</label>
                <input
                  id="screen-name"
                  type="text"
                  value={serverScreenName}
                  onChange={(e) => setServerScreenName(e.target.value)}
                />
              </div>
            </div>

            <div className="clients-header">
              <h4>客户端与边缘映射</h4>
              <button className="small-btn" onClick={addClientRoute}>
                + 添加客户端
              </button>
            </div>

            <div className="route-list">
              {clientRoutes.map(route => (
                <div key={route.id} className="route-row">
                  <input
                    type="text"
                    value={route.name}
                    onChange={(e) => updateClientRoute(route.id, { name: e.target.value })}
                    placeholder="客户端名称"
                  />
                  <input
                    type="text"
                    value={route.address}
                    onChange={(e) => updateClientRoute(route.id, { address: e.target.value })}
                    placeholder="客户端地址，如 192.168.1.10:24800"
                  />
                  <select
                    value={route.edge}
                    onChange={(e) => updateClientRoute(route.id, { edge: e.target.value as ScreenEdge })}
                  >
                    <option value="left">左边缘</option>
                    <option value="right">右边缘</option>
                    <option value="top">上边缘</option>
                    <option value="bottom">下边缘</option>
                  </select>
                  <button className="danger-btn" onClick={() => removeClientRoute(route.id)}>
                    删除
                  </button>
                </div>
              ))}
            </div>
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
            {appState.mode === 'server' && (
              <div className="status-item">
                <span className="status-label">当前边缘目标:</span>
                <span className="status-value">{activeClient ?? '未触发'}</span>
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
