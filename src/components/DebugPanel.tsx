import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './DebugPanel.css';

interface DAPSession {
  id: string;
  name: string;
  type: string;
  state: 'initializing' | 'running' | 'stopped' | 'terminated';
  config: any;
}

interface StackFrame {
  id: number;
  name: string;
  source: { path: string; sourceReference: number };
  line: number;
  column: number;
}

interface Variable {
  name: string;
  value: string;
  type: string;
  variablesReference: number;
}

interface Breakpoint {
  verified: boolean;
  line: number;
  column?: number;
}

const DebugPanel: React.FC = () => {
  const [, setSessions] = useState<DAPSession[]>([]);
  const [activeSession, setActiveSession] = useState<DAPSession | null>(null);
  const [, setStackFrames] = useState<StackFrame[]>([]);
  const [, setVariables] = useState<Variable[]>([]);
  const [breakpoints, setBreakpoints] = useState<Map<string, Breakpoint[]>>(new Map());
  const [watchExpressions, setWatchExpressions] = useState<string[]>([]);
  const [output, setOutput] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const workspaceRoot = '/Users/niranjan/Downloads/Research/infra-research/elixide';

  const launchDebug = async (configName: string) => {
    setError(null);
    try {
      const configs: Record<string, any> = {
        'Rust': { type: 'codelldb', request: 'launch', name: 'Debug Rust', program: '${workspaceFolder}/target/debug/elixide', args: [], cwd: '${workspaceFolder}' },
        'TypeScript': { type: 'node', request: 'launch', name: 'Debug TypeScript', program: '${workspaceFolder}/src/main.ts', args: [], cwd: '${workspaceFolder}', runtimeExecutable: 'node' },
        'Python': { type: 'python', request: 'launch', name: 'Debug Python', program: '${workspaceFolder}/main.py', args: [], cwd: '${workspaceFolder}' },
      };
      
      const config = configs[configName];
      if (!config) {
        setError(`Unknown config: ${configName}`);
        return;
      }

      const result = await invoke<{ sessionId: string }>('dap_launch', {
        config: JSON.stringify(config),
        workspaceRoot,
      });

      const session: DAPSession = {
        id: result.sessionId,
        name: config.name,
        type: config.type,
        state: 'running',
        config,
      };
      
      setSessions(prev => [...prev, session]);
      setActiveSession(session);
      addOutput(`Started debug session: ${config.name}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to launch debugger');
    }
  };

  const stopDebug = async () => {
    if (!activeSession) return;
    try {
      await invoke('dap_disconnect', { sessionId: activeSession.id });
      setActiveSession(null);
      setStackFrames([]);
      setVariables([]);
      addOutput('Debug session stopped');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stop debugger');
    }
  };

  const continueDebug = async () => {
    if (!activeSession) return;
    try {
      await invoke('dap_continue', { sessionId: activeSession.id });
      addOutput('Continued');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to continue');
    }
  };

  const pauseDebug = async () => {
    if (!activeSession) return;
    try {
      await invoke('dap_pause', { sessionId: activeSession.id });
      addOutput('Paused');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to pause');
    }
  };

  const stepOver = async () => {
    if (!activeSession) return;
    try {
      await invoke('dap_step_over', { sessionId: activeSession.id });
      addOutput('Step over');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to step over');
    }
  };

  const stepInto = async () => {
    if (!activeSession) return;
    try {
      await invoke('dap_step_into', { sessionId: activeSession.id });
      addOutput('Step into');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to step into');
    }
  };

  const stepOut = async () => {
    if (!activeSession) return;
    try {
      await invoke('dap_step_out', { sessionId: activeSession.id });
      addOutput('Step out');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to step out');
    }
  };

  // @ts-ignore unused
  const loadStackFrames = async (threadId: number = 1) => {
    if (!activeSession) return;
    try {
      const result = await invoke<{ frames: StackFrame[] }>('dap_get_stack_trace', {
        sessionId: activeSession.id,
        threadId,
      });
      setStackFrames(result.frames);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load stack frames');
    }
  };

  const loadVariables = async (frameId: number) => {
    try {
      const result = await invoke<{ variables: Variable[] }>('dap_get_variables', {
        sessionId: activeSession.id,
        variablesReference: frameId,
      });
      setVariables(result.variables);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load variables');
    }
  };

  // @ts-ignore unused
  const setBreakpointsInFile = async (uri: string, line: number) => {
    if (!activeSession) return;
    try {
      const result = await invoke<{ breakpoints: Breakpoint[] }>('dap_set_breakpoints', {
        sessionId: activeSession.id,
        sourcePath: uri,
        breakpoints: [{ line, column: 0 }],
      });
      
      const fileBreaks = new Map(breakpoints);
      fileBreaks.set(uri, result.breakpoints);
      setBreakpoints(fileBreaks);
      addOutput(`Breakpoint set at ${uri}:${line}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to set breakpoint');
    }
  };

  const addWatch = async () => {
    const expr = prompt('Enter expression to watch:');
    if (!expr) return;
    setWatchExpressions(prev => [...prev, expr]);
  };

  const removeWatch = (index: number) => {
    setWatchExpressions(prev => prev.filter((_, i) => i !== index));
  };

  const addOutput = (msg: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setOutput(prev => [...prev, `[${timestamp}] ${msg}`].slice(-100));
  };

  const handleFrameClick = (frame: StackFrame) => {
    loadVariables(frame.id);
  };

  return (
    <div className="debug-panel">
      <div className="debug-toolbar">
        <div className="debug-configs">
          <select
            value={activeSession?.name || ''}
            onChange={e => launchDebug(e.target.value)}
            disabled={loading}
          >
            <option value="">Select configuration...</option>
            <option value="Rust">Rust (codelldb)</option>
            <option value="TypeScript">TypeScript (node)</option>
            <option value="Python">Python (debugpy)</option>
          </select>
        </div>

        <div className="debug-actions">
          <button className="btn-icon" onClick={continueDebug} disabled={!activeSession || loading} title="Continue (F5)">
            ▶
          </button>
          <button className="btn-icon" onClick={pauseDebug} disabled={!activeSession || loading} title="Pause">
            ⏸
          </button>
          <button className="btn-icon" onClick={stepOver} disabled={!activeSession || loading} title="Step Over (F10)">
            ↷
          </button>
          <button className="btn-icon" onClick={stepInto} disabled={!activeSession || loading} title="Step Into (F11)">
            ↧
          </button>
          <button className="btn-icon" onClick={stepOut} disabled={!activeSession || loading} title="Step Out (Shift+F11)">
            ↥
          </button>
          <button className="btn-icon danger" onClick={stopDebug} disabled={!activeSession || loading} title="Stop (Shift+F5)">
            ■
          </button>
        </div>

        {loading && <span className="loading">Loading...</span>}
      </div>

      {error && <div className="debug-error">{error}</div>}

      <div className="debug-content">
        <div className="debug-sidebar">
          <div className="debug-section">
            <h4>Call Stack</h4>
            {stackFrames.length === 0 ? (
              <div className="empty">No stack frames</div>
            ) : (
              <ul className="stack-list">
                {stackFrames.map(frame => (
                  <li key={frame.id} onClick={() => handleFrameClick(frame)} className="stack-frame">
                    <span className="frame-name">{frame.name}</span>
                    <span className="frame-location">
                      {frame.source.path.split('/').pop()}:{frame.line}:{frame.column}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className="debug-section">
            <h4>Variables</h4>
            {variables.length === 0 ? (
              <div className="empty">Select a frame to see variables</div>
            ) : (
              <ul className="variable-list">
                {variables.map((v, i) => (
                  <li key={i} className="variable-item">
                    <span className="var-name">{v.name}</span>
                    <span className="var-value">{v.value}</span>
                    <span className="var-type">{v.type}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className="debug-section">
            <h4>Watch</h4>
            {watchExpressions.length === 0 ? (
              <div className="empty">No watch expressions</div>
            ) : (
              <ul className="watch-list">
                {watchExpressions.map((expr, i) => (
                  <li key={i} className="watch-item">
                    <span>{expr}</span>
                    <button className="btn-icon" onClick={() => removeWatch(i)}>✕</button>
                  </li>
                ))}
              </ul>
            )}
            <button className="btn-secondary" onClick={addWatch} style={{ width: '100%', marginTop: 8 }}>
              + Add Watch
            </button>
          </div>

          <div className="debug-section">
            <h4>Breakpoints</h4>
            {breakpoints.size === 0 ? (
              <div className="empty">No breakpoints</div>
            ) : (
              <ul className="breakpoint-list">
                {Array.from(breakpoints.entries()).map(([uri, bps]) => (
                  <li key={uri} className="breakpoint-item">
                    <strong>{uri.split('/').pop()}</strong>
                    {bps.map((bp, i) => (
                      <span key={i} className={`bp-marker ${bp.verified ? 'verified' : 'unverified'}`}>
                        Line {bp.line}
                      </span>
                    ))}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        <div className="debug-main">
          <div className="debug-output">
            <div className="output-header">
              <h4>Debug Console</h4>
            </div>
            <div className="output-content">
              {output.map((line, i) => (
                <div key={i} className="output-line">{line}</div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default DebugPanel;